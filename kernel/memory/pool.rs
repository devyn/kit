/*******************************************************************************
 *
 * kit/kernel/memory/pool.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Pools that track regions of objects of identical size.
//!
//! A region is a specific number of pages long, and includes the tracking
//! information necessary for it within those pages.
//!
//! The layout of a region is as follows:
//!
//!     +--------------------------------------------+
//!     | Obj1 | Obj2 | Obj3 | Obj4 | ... RegionInfo |
//!     +--------------------------------------------+
//!     | Page1    | Page2    | Page3    | Page4     |
//!     +--------------------------------------------+
//!
//! RegionInfo is located in the last part of the last page, at the last address
//! that is aligned with enough room to hold it.
//!
//! Objects are stored one after another. Alignment considerations should
//! pre-baked into the size.
//!
//! The algorithm is lock-free.

use core::fmt;
use core::sync::atomic::*;
use core::sync::atomic::Ordering::*;
use core::mem::{align_of, size_of, forget, replace};
use core::ptr;
use core::ops::Deref;

use crate::paging::PAGE_SIZE;

use super::{PageCount, VirtualAddress, align_addr};

#[derive(Debug)]
pub enum Error {
    NoFreeSpace,
    AddressNotAllocated(VirtualAddress),
    RegionInUse(VirtualAddress),
    RegionNotEmpty(VirtualAddress),
    RegionNotFound(VirtualAddress),
}

#[derive(Debug, Clone, Copy)]
struct PoolConfig {
    object_size: usize,
    region_pages: PageCount,
}

impl PoolConfig {
    /// The number of objects a region could hold if not for the bitmap.
    #[inline]
    const fn ideal_object_capacity(self) -> usize {
        let available_bytes =
            self.region_pages * PAGE_SIZE -
                align_addr(size_of::<RegionInfo>(), align_of::<RegionInfo>());

        available_bytes / self.object_size
    }

    /// The number of objects a region can hold.
    #[inline]
    const fn object_capacity(self) -> usize {
        // We can use up to the beginning of the region info
        let available_bytes = self.region_info_offset() as usize;

        available_bytes / self.object_size
    }

    /// The size of the RegionInfo structure, including bitmap and alignment
    #[inline]
    const fn region_info_size(self) -> usize {
        // How much space is required for the RegionInfo + bitmap
        let info_size = size_of::<RegionInfo>() + 
            bitmap::byte_size(self.ideal_object_capacity());

        align_addr(info_size, align_of::<RegionInfo>())
    }

    /// The offset from the start of a region to the RegionInfo structure
    #[inline]
    const fn region_info_offset(self) -> usize {
        self.region_pages * PAGE_SIZE - self.region_info_size()
    }
}

#[derive(Debug)]
pub struct Pool {
    config: PoolConfig,
    objects_used: AtomicUsize,
    objects_capacity: AtomicUsize,
    free_stack: AtomicPtr<RegionInfo>,
    all_stack: AtomicPtr<RegionInfo>,
}

impl Pool {
    /// Create a new, empty pool.
    ///
    /// No allocation is performed. You need to add pages to the pool before any
    /// object allocation can happen within it.
    pub const fn new(object_size: usize, region_pages: PageCount) -> Pool {
        assert!(object_size > 0);
        assert!(region_pages > 0);

        let config = PoolConfig {
            object_size,
            region_pages,
        };

        assert!(config.region_info_offset() > 0);

        Pool {
            config,
            objects_used: AtomicUsize::new(0),
            objects_capacity: AtomicUsize::new(0),
            free_stack: AtomicPtr::new(ptr::null_mut()),
            all_stack: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn object_size(&self) -> usize {
        self.config.object_size
    }
    pub fn region_pages(&self) -> PageCount {
        self.config.region_pages
    }
    pub fn objects_used(&self) -> usize {
        self.objects_used.load(Relaxed)
    }
    pub fn objects_capacity(&self) -> usize {
        self.objects_capacity.load(Relaxed)
    }

    /// Make a region available to the pool.
    ///
    /// # Unsafety
    ///
    /// The region is presumed to be mapped and available, up to the page size
    /// specified by [region_pages]. It will be initialized as free.
    ///
    /// We also assume the region hasn't already been inserted. Inserting the
    /// same region more than once before removing it first is undefined.
    pub unsafe fn insert_region(&self, addr: usize) -> Result<(), Error> {
        assert!(addr > 0);

        initialize_region(addr, self.config);

        let region_info = RegionInfoRef::new(ptr::NonNull::new(
            (addr + self.config.region_info_offset() as usize)
                as *mut RegionInfo,
        )
        .unwrap());

        push(&self.all_stack, region_info.clone(), ListSel::All);
        push(&self.free_stack, region_info, ListSel::Free);

        self.objects_capacity.fetch_add(self.config.object_capacity(), Relaxed);

        debug!("pool({}) region inserted: {:016x}",
            self.object_size(), addr);

        Ok(())
    }

    pub fn remove_region(&self, addr: usize) -> Result<(), Error> {
        // Safety: stack first/next pointers are assumed valid
        //
        // First, we try to take it from the free list. It's okay if we don't
        // find it here but we do have to take it.
        let found_in_free_list = self.remove(ListSel::Free, addr);

        // Then we try to take it from the all list.
        let found_in_all_list = self.remove(ListSel::All, addr);

        if found_in_all_list.is_none() {
            // If we couldn't find it in the all list, we might have stepped on
            // someone else trying to destroy this region, but they won't be
            // able to either.
            //
            // Put it back in the free list if it was.
            if let Some(free_ref) = found_in_free_list {
                unsafe { push(&self.free_stack, free_ref, ListSel::Free); }
            }
            return Err(Error::RegionInUse(addr));
        }

        // Drop this reference so that we can possibly have one exclusive
        // reference.
        let was_in_free_list = found_in_free_list.is_some();
        drop(found_in_free_list);

        let region = found_in_all_list.unwrap();

        // Let the region decide if it can be dropped.
        // 
        // Safety: region was in list, must be a valid region with bitmap in the
        // right place.
        match unsafe { region.prepare_drop(self.config) } {
            // It was exclusively held and empty.
            Ok(_) => {
                self.objects_capacity.fetch_sub(self.config.object_capacity(),
                    Relaxed);

                debug!("pool({}) region removed: 0x{:016x}",
                    self.object_size(), addr);
                Ok(())
            },

            // Not possible, so let's clean up what we did and put it back where
            // we found it.
            Err(e) => {
                unsafe { push(&self.all_stack, region.clone(), ListSel::All); }

                if was_in_free_list {
                    unsafe {
                        push(&self.free_stack, region.clone(), ListSel::All);
                    }
                }

                Err(e)
            }
        }
    }

    pub fn allocate(&self) -> Result<VirtualAddress, Error> {
        // Safety: stack first/next pointers are assumed valid
        //
        // Pop the free stack until we can get an object
        while let Some(free) = unsafe { pop(&self.free_stack, ListSel::Free) } {
            // Operations involving the bitmap are safe because we know our
            // config is good and the references we're using to region info are
            // part of actual regions.
            if let Some(free_address) = unsafe { free.allocate(self.config) } {
                // We have it. Unless it's full, put it back on the free stack
                if !unsafe { free.bitmap(self.config) }.is_full() {
                    unsafe { push(&self.free_stack, free, ListSel::Free); }
                }

                self.objects_used.fetch_add(1, Relaxed);

                let sp: usize;

                unsafe { asm!("mov {}, rsp", out(reg) sp); }

                debug!("pool({},{}) allocated: {:016x} sp={:016x}",
                    self.object_size(), self.region_pages(), free_address,
                    sp);

                return Ok(free_address);
            } else {
                // Is this weird?
                debug!("Weird: found a full region on the free stack: {:?}",
                    unsafe { free.debug(self.config) });
            }

        }
        Err(Error::NoFreeSpace)
    }

    pub fn deallocate(&self, addr: usize) -> Result<Deallocated, Error> {
        todo!()
    }

    #[inline]
    fn first(&self, which: ListSel) -> &AtomicPtr<RegionInfo> {
        match which {
            ListSel::Free => &self.free_stack,
            ListSel::All => &self.all_stack,
        }
    }

    /// Remove the given region (by base address) from the given list.
    fn remove(&self, which: ListSel, region: usize) -> Option<RegionInfoRef> {
        let mut predicate = |found_region: &RegionInfo| {
            found_region.region_base(self.config) == region
        };

        // First try to get it from the head
        let pop_in_first = unsafe {
            pop_if(self.first(which), which, &mut predicate)
        };

        if let Some(_) = pop_in_first {
            pop_in_first
        } else {
            // Otherwise, use the iterator
            for parent_region in self.iter(which) {
                let pop_in_next = unsafe {
                    pop_if(parent_region.next(which), which, &mut predicate)
                };

                if let Some(_) = pop_in_next {
                    return pop_in_next;
                }
            }
            None
        }
    }

    #[inline]
    fn load_first(&self, which: ListSel) -> Option<RegionInfoRef> {
        ptr::NonNull::new(self.first(which).load(Relaxed))
            .map(|p| unsafe { RegionInfoRef::new(p) })
    }

    fn iter(&self, which: ListSel) -> Iter {
        Iter { which, next: self.load_first(which) }
    }
}

/// May provide information about a region that may be empty.
#[derive(Debug, Clone)]
pub struct Deallocated {
    maybe_empty: Option<usize>
}

impl Deallocated {
    pub fn maybe_empty(&self) -> Option<usize> {
        self.maybe_empty
    }
}

/// This iterator holds a reference to whatever node it's currently looking at.
/// Failing to drop the iterator properly would result in a leakage of a memory
/// region.
struct Iter {
    which: ListSel,
    next: Option<RegionInfoRef>,
}

impl Iterator for Iter {
    type Item = RegionInfoRef;

    fn next(&mut self) -> Option<RegionInfoRef> {
        if let Some(next_p) = replace(&mut self.next, None) {
            self.next = next_p.load_next(self.which);
            Some(next_p)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ListSel {
    Free,
    All
}

unsafe fn push(
    prev: &AtomicPtr<RegionInfo>,
    new: RegionInfoRef,
    which: ListSel
) {
    // If next is already set, we should decrement the reference counter to what
    // it currently is.
    //
    // This is because pop() leaves the reference intact, just in case we have
    // an iterator stuck on it.
    let next = new.next(which).load(Relaxed);
    if let Some(next_p) = ptr::NonNull::new(next) {
        next_p.as_ref().decr_ref();
    }

    // Release semantics because we don't want to do this before we've made all
    // of the modifications we were going to do to RegionInfo first
    prev.fetch_update(Release, Relaxed, |val: *mut RegionInfo| {
        // Set the next on the node we will insert to the previous value
        new.next(which).store(val, Relaxed);
        Some(new.ptr.as_ptr())
    }).unwrap_or_else(|_| panic!("always-update fetch_update failed!"));

    // We don't need to change anything about the number of references to
    // whatever we end up setting next to, as the reference is merely moved.
    //
    // But we do need to forget the reference to new, as the reference is now
    // owned by the list.
    forget(new);
}

unsafe fn pop(node: &AtomicPtr<RegionInfo>, which: ListSel)
    -> Option<RegionInfoRef> {

    pop_if(node, which, |_| true)
}

unsafe fn pop_if<F>(node: &AtomicPtr<RegionInfo>, which: ListSel, mut pred: F)
    -> Option<RegionInfoRef>
    where F: FnMut(&RegionInfo) -> bool {

    let mut out;

    // Acquire semantics because the popped node may be modified after taking it
    loop {
        out = ptr::NonNull::new(node.load(Acquire));

        // Set the previous node to the next node of the node we're taking, but
        // don't update a null.
        if let Some(taken) = out {
            // Take a reference to taken while we work on it
            let taken_ref = RegionInfoRef::new(taken);

            // Apply predicate and return early if it doesn't match.
            if !pred(&*taken_ref) {
                return None;
            }

            let next = taken_ref.next(which).load(Relaxed);

            // Add a reference to next, because the node we pop will still
            // have a reference to it.
            if let Some(next_p) = ptr::NonNull::new(next) {
                next_p.as_ref().incr_ref();
            }

            // Try to compare_exchange node -> next
            let cas_res = node.compare_exchange(
                taken.as_ptr(), next, Acquire, Relaxed);

            if cas_res.is_ok() {
                break;
            } else {
                // Need to clean up the extra reference we added to next
                if let Some(next_p) = ptr::NonNull::new(next) {
                    next_p.as_ref().decr_ref();
                }
            }
        } else {
            break;
        }
    }

    // Wrap the acquired reference in RegionInfoRef so the caller can drop it if
    // needed.
    out.map(|taken| RegionInfoRef { ptr: taken })
}

/// A held reference to a RegionInfo. Automatically decrements the reference
/// pointer on drop.
struct RegionInfoRef {
    ptr: ptr::NonNull<RegionInfo>,
}

impl RegionInfoRef {
    unsafe fn new(ptr: ptr::NonNull<RegionInfo>) -> RegionInfoRef {
        ptr.as_ref().incr_ref();
        RegionInfoRef { ptr }
    }
}

impl Drop for RegionInfoRef {
    fn drop(&mut self) {
        unsafe { self.ptr.as_ref().decr_ref() }
    }
}

impl Deref for RegionInfoRef {
    type Target = RegionInfo;
    fn deref(&self) -> &RegionInfo {
        unsafe { self.ptr.as_ref() }
    }
}

impl Clone for RegionInfoRef {
    fn clone(&self) -> RegionInfoRef {
        self.incr_ref();
        RegionInfoRef { ptr: self.ptr }
    }
}

impl fmt::Debug for RegionInfoRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("RegionInfoRef")
            .field(&**self)
            .finish()
    }
}

struct RegionInfo {
    references: AtomicI32,
    next_free: AtomicPtr<RegionInfo>,
    next_all: AtomicPtr<RegionInfo>,
}

// Bitmap comes immediately after RegionInfo, always.
const BITMAP_OFFSET: usize = size_of::<RegionInfo>();

unsafe fn initialize_region(region_base: usize, config: PoolConfig) {
    debug!("initialize_region(0x{:016x}, {:?})", region_base, config);

    let region_info_ptr = ptr::NonNull::new(
        (region_base + config.region_info_offset() as usize) as *mut RegionInfo
    ).unwrap();

    debug!("region_info_ptr = {:?}, size = {:x}", region_info_ptr, config.region_info_size());

    RegionInfo::initialize(region_info_ptr, config);
}

impl RegionInfo {
    fn new() -> RegionInfo {
        RegionInfo {
            references: AtomicI32::new(0),
            next_free: AtomicPtr::new(ptr::null_mut()),
            next_all: AtomicPtr::new(ptr::null_mut())
        }
    }

    unsafe fn initialize(mut ptr: ptr::NonNull<RegionInfo>, config: PoolConfig) {
        *ptr.as_mut() = RegionInfo::new();
        ptr.as_ref().bitmap(config).clear();
    }

    /// Increment the reference counter
    fn incr_ref(&self) {
        self.references.fetch_add(1, Acquire);
    }

    /// Decrement the reference counter
    fn decr_ref(&self) {
        self.references.fetch_sub(1, Release);
    }

    /// Returns true if there is currently more than one valid reference to the
    /// region.
    fn is_shared(&self) -> bool {
        // For safety, we do this 100 times before we ever return false. This is
        // to make extra sure that we don't end up treating this as unreferenced
        // just because another thread didn't get a chance to add a reference
        // quickly enough.
        for _ in 0..100 {
            if self.references.load(SeqCst) > 1 {
                return true;
            }
        }
        false
    }

    /// Prepares the region for destruction. Returns `Ok` if the region is safe
    /// to release, `Err` otherwise.
    ///
    /// If [Error::RegionInUse] is returned, this may be a temporary condition;
    /// a short loop might be called for to have a higher chance of being able
    /// to remove the region.
    ///
    /// # Unsafety
    ///
    /// Calls [bitmap], so the same caveats apply.
    unsafe fn prepare_drop(&self, config: PoolConfig) -> Result<(), Error> {
        if !self.is_shared() {
            // If there are zero references, we should be able to guarantee that
            // nothing else will change the region anymore.
            //
            // We need to clear our next pointers and remove their references.
            // This won't cause a problem for readers, as there aren't any other
            // references to this node. Even if we end up not destroying the
            // region, we would have to do this anyway when re-inserting it, so
            // we might as well.
            for which in [ListSel::Free, ListSel::All] {
                let next = self.next(which).load(Relaxed);

                if let Some(next_p) = ptr::NonNull::new(next) {
                    next_p.as_ref().decr_ref();
                    self.next(which).store(ptr::null_mut(), Relaxed);
                }
            }

            // Check to make sure the region is actually empty.
            if self.bitmap(config).is_empty() {
                // If so, it's definitely safe to drop this region.
                Ok(())
            } else {
                Err(Error::RegionNotEmpty(self.region_base(config)))
            }
        } else {
            Err(Error::RegionInUse(self.region_base(config)))
        }
    }

    /// Get the free object bitmap for the region.
    ///
    /// # Unsafety
    ///
    /// Does pointer arithmetic on self to find the bitmap. The region info
    /// reference must therefore actually be where the region is.
    unsafe fn bitmap(&self, config: PoolConfig) -> FreeBitmap {
        let bits = config.object_capacity();
        let ptr = (self as *const RegionInfo) as usize + BITMAP_OFFSET;

        FreeBitmap::new(bits, ptr)
    }

    /// Get the next list pointer according to the value of `which`
    #[inline]
    fn next(&self, which: ListSel) -> &AtomicPtr<RegionInfo> {
        match which {
            ListSel::Free => &self.next_free,
            ListSel::All => &self.next_all,
        }
    }

    /// Read the next list pointer according to the value of `which` and return
    /// an auto-ref
    #[inline]
    fn load_next(&self, which: ListSel) -> Option<RegionInfoRef> {
        ptr::NonNull::new(self.next(which).load(Relaxed))
            .map(|p| unsafe { RegionInfoRef::new(p) })
    }

    /// Calculate the start of the region
    #[inline]
    fn region_base(&self, config: PoolConfig) -> usize {
        (self as *const RegionInfo) as usize -
            config.region_info_offset() as usize
    }

    /// True if the address is in range for the region
    #[inline]
    fn contains(&self, config: PoolConfig, addr: VirtualAddress) -> bool {
        let region_base = self.region_base(config);
        let region_end  = region_base + config.region_info_offset() as usize;

        addr >= region_base && addr < region_end
    }

    unsafe fn allocate(&self, config: PoolConfig) -> Option<VirtualAddress> {
        if let Some(object_index) = self.bitmap(config).acquire_bit() {
            // Index the Nth object and return that pointer
            Some(self.region_base(config) as VirtualAddress +
                object_index * config.object_size)
        } else {
            None
        }
    }

    unsafe fn deallocate(
        &self,
        config: PoolConfig,
        addr: VirtualAddress
    ) -> bool {
        assert!(self.contains(config, addr));

        // First, do the math on the address to figure out which object it is
        let region_base = self.region_base(config);
        let addr_from_region = addr - region_base;
        let object_index = addr_from_region / config.object_size;

        // Ask the bitmap to dealloc
        self.bitmap(config).release_bit(object_index)
    }

    /// Provides better debugging information, including the bitmap
    unsafe fn debug(&self, config: PoolConfig) -> RegionInfoDebug {
        RegionInfoDebug { info: self, config }
    }
}

// Safe debug
impl fmt::Debug for RegionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RegionInfo")
            .field("references", &self.references.load(Relaxed))
            .field("next_free", &self.next_free.load(Relaxed))
            .field("next_all", &self.next_all.load(Relaxed))
            .finish_non_exhaustive()
    }
}

pub struct RegionInfoDebug<'a> {
    info: &'a RegionInfo,
    config: PoolConfig
}

impl<'a> fmt::Debug for RegionInfoDebug<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RegionInfo")
            .field("references", &self.info.references.load(Relaxed))
            .field("next_free", &self.info.next_free.load(Relaxed))
            .field("next_all", &self.info.next_all.load(Relaxed))
            .field("bitmap", &unsafe { self.info.bitmap(self.config) })
            .finish()
    }
}

// Submodule to enforce unsafe constructor
mod bitmap {
    use core::sync::atomic::*;
    use core::sync::atomic::Ordering::*;
    use core::fmt;

    #[derive(Clone)]
    pub struct FreeBitmap {
        bits: usize,
        ptr: *mut AtomicU8,
    }

    pub const fn byte_size(bits: usize) -> usize {
        if bits % 8 == 0 {
            bits/8
        } else {
            bits/8 + 1
        }
    }

    impl FreeBitmap {
        pub unsafe fn new(bits: usize, ptr: usize) -> FreeBitmap {
            FreeBitmap { bits, ptr: ptr as *mut AtomicU8 }
        }

        pub fn clear(&self) {
            debug!("clear({}, {:?})", self.bits, self.ptr);

            let mut pointer = self.ptr;

            for byte in 0..byte_size(self.bits) {
                let desired_state =
                    if self.bits % 8 != 0 && byte == byte_size(self.bits) - 1 {
                        // Last byte should pad out of range bits to 1
                        0xFF << (8 - self.bits % 8)
                    } else {
                        0x00
                    };

                unsafe {
                    (&*pointer).store(desired_state, Relaxed);
                    pointer = pointer.offset(1);
                }
            }
        }

        /// Atomically finds the first free bit and sets it to used, then
        /// returns the index of the acquired bit.
        pub fn acquire_bit(&self) -> Option<usize> {
            let mut pointer = self.ptr;

            for byte in 0..byte_size(self.bits) {
                let mut acquired_bit = None;

                unsafe {
                    // We don't care about success/failure because that's
                    // actually captured by acquired_bit.
                    //
                    // We shouldn't need to order other memory - we are just
                    // concerned with this byte - so Relaxed is appropriate.
                    let _ = (&*pointer).fetch_update(Relaxed, Relaxed, |val| {
                        for bit in 0..8 {
                            if val & (1 << bit) == 0 {
                                // We may have acquired it, but this can run
                                // again if we fail to CAS.
                                acquired_bit = Some(bit);
                                return Some(val | (1 << bit));
                            }
                        }
                        // Don't change.
                        acquired_bit = None;
                        None
                    });
                }

                if let Some(bit) = acquired_bit {
                    return Some(byte * 8 + bit);
                } else {
                    unsafe { pointer = pointer.offset(1); }
                }
            }

            // None of the bytes had a free value
            None
        }

        /// Atomically sets the bit at the given index to free. Returns true if
        /// the bit was used, false if the bit was already free.
        pub fn release_bit(&self, index: usize) -> bool {
            let pointer = {
                assert!(index < self.bits,
                    "Index out of range: {} >= {}", index, self.bits);

                self.ptr.wrapping_add(index / 8)
            };

            let bit = index % 8;

            // Use the fetch_update result - if a modification was made, we
            // freed the bit.
            unsafe {
                (&*pointer).fetch_update(Relaxed, Relaxed, |val| {
                    if val & (1 << bit) != 0 {
                        // Clear the bit
                        Some(val & !(1 << bit))
                    } else {
                        None
                    }
                }).is_ok()
            }
        }

        /// Checks if the region is empty. Only can be trusted if there are no
        /// other references to the region, as the whole operation is not
        /// atomic.
        pub fn is_empty(&self) -> bool {
            let mut pointer = self.ptr;

            for byte in 0..byte_size(self.bits) {
                let desired_state =
                    if self.bits % 8 != 0 && byte == byte_size(self.bits) - 1 {
                        // Last byte should pad out of range bits to 1
                        0xFF << (8 - self.bits % 8)
                    } else {
                        0x00
                    };

                unsafe {
                    if (&*pointer).load(Relaxed) != desired_state {
                        return false;
                    }
                    pointer = pointer.offset(1);
                }
            }

            true
        }

        /// Checks if the region is full. Only can be trusted if there are no
        /// other references to the region, as the whole operation is not
        /// atomic.
        pub fn is_full(&self) -> bool {
            let mut pointer = self.ptr;

            for _ in 0..byte_size(self.bits) {
                let desired_state = 0xFF;

                unsafe {
                    if (&*pointer).load(Relaxed) != desired_state {
                        return false;
                    }
                    pointer = pointer.offset(1);
                }
            }

            true
        }
    }

    impl fmt::Debug for FreeBitmap {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "FreeBitmap({}, \"", self.bits)?;

            let mut pointer = self.ptr;

            for _ in 0..byte_size(self.bits) {
                write!(f, "{:02X}", unsafe { (&*pointer).load(Relaxed) })?;
                unsafe { pointer = pointer.offset(1); }
            }

            write!(f, "\")")?;
            Ok(())
        }
    }
}
use bitmap::FreeBitmap;
