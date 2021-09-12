/*******************************************************************************
 *
 * kit/kernel/memory.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Physical memory management and kernel heap.

use core::cmp::min;
use core::ops::Range;
use core::alloc::{GlobalAlloc, Layout};

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;

use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::paging::{self, kernel_pageset};
use crate::paging::{PagesetExt, PageType, Page};
use crate::paging::PAGE_SIZE;

use crate::multiboot;
use crate::process::Id as ProcessId;
use crate::sync::LockFreeList;
use crate::sync::lock_free_list::Node;
use crate::util::align_up;

pub mod pool;

mod large_heap;

pub mod region_math;
pub use region_math::RegionSet;

/// The first "safe" physical address. Memory below this is not likely to be
/// safe for general use, and may include parts of the kernel image among other
/// things.
///
/// 0x0 to SAFE_BOUNDARY are identity-mapped starting at 0xffff_ffff_8000_0000,
/// so we avoid that region
const SAFE_BOUNDARY: usize = 0x100000;

const INITIAL_HEAP_LENGTH: usize = 131072;

/// Rust is stack-hungry. We may even want more?
pub const KSTACK_SIZE: usize = 32768;
pub const KSTACK_ALIGN: usize = 16;

const_assert!(KSTACK_SIZE < isize::MAX as usize);
const_assert!(KSTACK_SIZE > 0);
const_assert!(KSTACK_ALIGN & (KSTACK_ALIGN-1) == 0);
const_assert!(KSTACK_ALIGN > 0);

extern {
    static mut MEMORY_INITIAL_HEAP: [u8; INITIAL_HEAP_LENGTH];
}

#[cfg(not(test))]
#[global_allocator]
#[export_name = "_kernel_heap"]
static mut KERNEL_HEAP: KernelHeap = KernelHeap::InitialHeap(128);

#[cfg(test)]
static mut KERNEL_HEAP: KernelHeap = KernelHeap::StdHeap;

#[derive(Debug)]
enum KernelHeap {
    #[cfg_attr(test, allow(unused))]
    InitialHeap(usize),
    LargeHeap(large_heap::HeapState),
    #[cfg(test)]
    StdHeap,
}

// Support for Rust library allocation using the kernel heap
unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocate(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        deallocate(ptr, layout.size(), layout.align())
    }
}

// What to do on an allocation error
#[cfg(not(test))]
#[alloc_error_handler]
fn handle_alloc_error(layout: Layout) -> ! {
    panic!("Memory allocation failed: {:?}", layout);
}

#[derive(Debug)]
struct RegionState {
    alloc_regions: LockFreeList<AllocRegionState>,
    free_regions: LockFreeList<FreeRegion<PhysicalAddress>>,
    total_page_count: usize,
    free_page_count: AtomicUsize,
}

static mut REGION_STATE: Option<RegionState> = None;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;
pub type PageCount = usize;

#[derive(Debug)]
struct AllocRegionState {
    start: PhysicalAddress,
    length: PageCount,
    users: Arc<Vec<RegionUser>>
}

impl AllocRegionState {
    fn range(&self) -> Range<PhysicalAddress> {
        self.start .. (self.start + self.length * PAGE_SIZE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionUser {
    Kernel,
    Process(ProcessId)
}

#[derive(Debug)]
struct FreeRegion<T> {
    start: T,
    length: AtomicUsize, // page count
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InitMemoryMap {
    pub usable: Vec<(PhysicalAddress, PhysicalAddress)>,
    pub reserved: Vec<(PhysicalAddress, PhysicalAddress)>,
    pub boot_mappings: Vec<(VirtualAddress, PageCount, Page<PhysicalAddress>)>,
}

impl Default for InitMemoryMap {
    fn default() -> InitMemoryMap {
        InitMemoryMap {
            usable: vec![],
            reserved: vec![],
            boot_mappings: vec![],
        }
    }
}

impl InitMemoryMap {
    pub fn heap_usable(&self) -> RegionSet<PhysicalAddress> {
        let mut set: RegionSet<PhysicalAddress> = RegionSet::new();

        for &(start, end) in &self.usable {
            set.insert(start..end);
        }

        for &(start, end) in &self.reserved {
            set.remove(start..end);
        }

        for &(_, pages, page) in &self.boot_mappings {
            if let Some((start, _)) = page {
                set.remove(start..(start + pages * PAGE_SIZE));
            }
        }

        set
    }

    pub unsafe fn load_from_multiboot(&mut self, info: &multiboot::Info) {
        info.parse_available(&mut self.usable);
        info.parse_reserved(&mut self.reserved);
        info.generate_identity_maps(&mut self.boot_mappings);
    }
}

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(memory_map: &InitMemoryMap) {
    let free_regions = LockFreeList::new();

    for range in memory_map.heap_usable().iter() {
        let addr = range.start;
        let len  = range.end - range.start;

        // Align physical base address to page size.
        let mut physical_base = align_up(addr, PAGE_SIZE);

        // Remove remainder from length and count pages.
        let mut pages = (len - (addr % PAGE_SIZE)) / PAGE_SIZE;

        // If the base starts before SAFE_BOUNDARY, remove the pages before that
        // (and make sure we still have pages left).
        if physical_base < SAFE_BOUNDARY {
            let diff = (SAFE_BOUNDARY - physical_base) / PAGE_SIZE;

            if diff < pages {
                physical_base += diff * PAGE_SIZE;
                pages         -= diff;
            } else {
                continue; // skip this entry
            }
        }

        // If the entry has at least one page, add it to the free regions.
        if pages > 0 {
            free_regions.push(Node::new(FreeRegion {
                start: physical_base,
                length: pages.into(),
            }));
        }
    }

    let total_page_count = free_regions.iter().fold(0,
        |s, r| s + r.length.load(Relaxed));

    REGION_STATE = Some(RegionState {
        total_page_count,
        free_page_count: total_page_count.into(),
        free_regions,
        alloc_regions: LockFreeList::new(),
    });
}

pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    trace!("allocate({}, {})", size, align);

    let ptr = match KERNEL_HEAP {
        KernelHeap::InitialHeap(ref mut counter) =>
            initial_heap_allocate(counter, size, align),
        KernelHeap::LargeHeap(ref state) =>
            large_heap::allocate(state, size, align),

        #[cfg(test)]
        KernelHeap::StdHeap => {
            std::alloc::alloc(
                std::alloc::Layout::from_size_align(size, align).unwrap())
        }
    };

    ptr
}

pub unsafe fn deallocate(ptr: *mut u8, size: usize, align: usize) {
    trace!("deallocate({:p}, {}, {})", ptr, size, align);

    match KERNEL_HEAP {
        KernelHeap::InitialHeap(_) =>
            // Deallocation not supported.
            (),
        KernelHeap::LargeHeap(ref state) =>
            large_heap::deallocate(state, ptr, size, align),

        #[cfg(test)]
        KernelHeap::StdHeap => {
            std::alloc::dealloc(ptr,
                std::alloc::Layout::from_size_align(size, align).unwrap());
        }
    }
}

unsafe fn initial_heap_allocate(counter: &mut usize, size: usize, align: usize)
                                -> *mut u8 {

    let new_counter = align_up(*counter, align);

    if new_counter + size >= INITIAL_HEAP_LENGTH {
        panic!("not enough memory for ({}, {}) in initial heap!", size, align);
    }

    let ptr = (&mut MEMORY_INITIAL_HEAP[new_counter]) as *mut u8;

    *counter = new_counter + size;

    ptr
}

pub unsafe fn enable_large_heap() {
    assert!(paging::initialized());

    if let KernelHeap::LargeHeap(_) = KERNEL_HEAP {
        // Already enabled, don't need to do anything.
        return;
    }

    KERNEL_HEAP = KernelHeap::LargeHeap(large_heap::initialize());
}

pub fn acquire_region(owner: RegionUser, pages: PageCount)
                      -> Option<(PhysicalAddress, PageCount)> {

    // Safety: initialized once
    let state = unsafe { 
        REGION_STATE.as_ref().expect("memory::initialize() not called")
    };

    let free_page_count = state.free_page_count.load(Relaxed);
    if free_page_count < pages {
        trace!("free_page_count={} < {}", free_page_count, pages);
        return None;
    }

    let mut alloc_start = 0;
    let mut acq_pages = 0;

    // Find the first free region with (any) space
    for free_region in state.free_regions.iter() {
        let mut original_length = 0;

        // Take as many pages as we can from it, up to pages
        let res = free_region.length.fetch_update(Relaxed, Relaxed,
            |length| {
                original_length = length;

                if length > 0 {
                    acq_pages = min(length, pages);
                    Some(length - min(length, pages))
                } else {
                    acq_pages = 0;
                    None
                }
            }
        );

        // Our allocated region will be at the end, since we can only update
        // length
        alloc_start = free_region.start +
            ((original_length - acq_pages) * PAGE_SIZE);

        // Remove free region if it's empty
        if free_region.length.load(Relaxed) == 0 {
            state.free_regions.remove(&free_region);
        }

        if res.is_ok() {
            break;
        }
    }

    if acq_pages > 0 {
        state.free_page_count.fetch_sub(acq_pages, Relaxed);

        state.alloc_regions.push(Node::new(AllocRegionState {
            start: alloc_start,
            length: acq_pages,
            users: vec![owner].into()
        }));

        Some((alloc_start, acq_pages))
    } else {
        trace!("No free physical region available.");
        None
    }
}

pub fn release_region(
    user: RegionUser,
    paddr: PhysicalAddress,
    pages: PageCount
) {
    // Safety: initialized once
    let state = unsafe { 
        REGION_STATE.as_ref().expect("memory::initialize() not called")
    };

    let region_to_release = paddr .. (paddr + pages * PAGE_SIZE);

    let mut pages_to_release = pages;
    let mut tries = 0;

    while pages_to_release > 0 && tries < 1000 {
        // Find an overlapping region
        let region = state.alloc_regions
            .drain_filter(|region| {
                region_math::overlaps(
                    &region.range(), &region_to_release) &&
                region.users.contains(&user)
            })
            .next();

        if let Some(region) = region {
            let cut = region_math::cut(
                region.range(),
                region_to_release.clone()).unwrap();

            // Insert the before and after regions that still exist
            for r in [cut.before, cut.after].iter().flatten() {
                state.alloc_regions.push(Node::new(AllocRegionState {
                    start: r.start,
                    length: (r.end - r.start)/PAGE_SIZE,
                    users: region.users.clone(),
                }));
            }

            let excluded_length =
                (cut.excluded.end - cut.excluded.start)/PAGE_SIZE;

            if region.users.len() > 1 {
                // If there are other users, re-insert the excluded region with
                // the other users
                let reinsert = AllocRegionState {
                    start: cut.excluded.start,
                    length: excluded_length,
                    users: region.users.iter().cloned()
                        .filter(|u| *u != user)
                        .collect::<Vec<_>>()
                        .into(),
                };
                state.alloc_regions.push(Node::new(reinsert));
            } else {
                // We owned it exclusively. Just release the data to the free
                // region list.

                release_to_free_region_list(&state.free_regions,
                    cut.excluded.start, excluded_length, "physical");

                // Update the counter
                state.free_page_count.fetch_add(excluded_length, Relaxed);
            }

            pages_to_release -= excluded_length;
        } else {
            // Increment retry counter. It's possible it'll be there if we look
            // again
            tries += 1;
        }
    }

    if pages_to_release > 0 {
        panic!("Wanted to release physical region {:?}, {:016x} x {}, \
            but can't find at least {} pages.",
            user, paddr, pages, pages_to_release);
    }
}

fn release_to_free_region_list(
    list: &LockFreeList<FreeRegion<usize>>,
    start: usize,
    length: usize,
    type_: &'static str,
) {
    // Optimistically make a new region to place. It's important to have this
    // ready now in case we end up setting the only free region to zero...
    let new_node = Node::new(FreeRegion {
        start: start,
        length: length.into(),
    });

    // Try to find a free region that we can extend to include what we just
    // freed.
    for free_region in list.iter() {
        if start + length * PAGE_SIZE == free_region.start {
            // The free region starts where our new node ends. We can unify it
            // with the free region, but it's a little complicated, we have to
            // set that free region's length to zero to lock it while we add the
            // new node.
            let other_length = free_region.length.swap(0, Relaxed);
            new_node.length.fetch_add(other_length, Relaxed);

            // Now the new_node should be possible to insert.
            list.push(new_node);

            // We can remove the old free region since it's zero anyway
            list.remove(&free_region);

            trace!("released {} {:016x} + {} + {} by unify start",
                type_, start, length, other_length);

            return;
        } else if start < free_region.start { 
            continue;
        }

        if free_region.length.fetch_update(Relaxed, Relaxed, |len| {
            // Skip len = 0, as it's a lock value
            if len != 0 && free_region.start + len * PAGE_SIZE == start {
                // It starts where this ends, so we can just add the length
                Some(len + length)
            } else {
                // Can't unify.
                None
            }
        }).is_ok() {
            // We were able to extend it.
            trace!("released {} {:016x} + {} by unify end into {:?}",
                type_, start, length, *free_region);
            return;
        }
    }

    // We weren't able to find a free region that overlaps with this, so
    // just add it to the free list.
    list.push(new_node);

    trace!("released {} {:016x} + {} as new free region", type_, start, length);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AcquiredMappedRegion {
    vaddr: *mut u8,
    paddr: PhysicalAddress,
    pages: PageCount,
}

/// Returns the number of pages successfully allocated on error
fn kernel_acquire_and_map<F>(
    vaddr: *mut u8,
    pages: PageCount,
    mut push_region: F,
) -> Result<(), PageCount>
    where F: FnMut(AcquiredMappedRegion) {

    trace!("kernel_acquire_and_map({:?}, {})", vaddr, pages);

    let mut cur_vaddr = vaddr as usize;
    let mut cur_pages = pages;

    while cur_pages > 0 {
        let (got_paddr, got_pages) =
            match acquire_region(RegionUser::Kernel, cur_pages) {
                Some(x) => x,
                None => return Err(pages - cur_pages)
            };

        unsafe {
            kernel_pageset()
                .map_pages_with_type(
                    cur_vaddr,
                    (got_paddr..).step_by(PAGE_SIZE).take(got_pages),
                    PageType::default().writable(),
                )
                .expect("unable to map acquired pages into kernel space")
        }

        push_region(AcquiredMappedRegion {
            vaddr: cur_vaddr as *mut u8,
            paddr: got_paddr,
            pages: got_pages
        });

        cur_vaddr += got_pages * PAGE_SIZE;
        cur_pages -= got_pages;
    }

    Ok(())
}

fn kernel_unmap_and_release(r: AcquiredMappedRegion) {
    trace!("kernel_unmap_and_release({:?}, {:016x}, {})",
        r.vaddr, r.paddr, r.pages);

    // First, unmap the pages so that they aren't in use anymore
    unsafe {
        kernel_pageset()
            .unmap_pages(r.vaddr as usize, r.pages)
            .unwrap_or_else(|err| {
                panic!("unable to kernel_unmap_and_release({:?}, {:016x}, {}): \
                    unmap failed: {}", r.vaddr, r.paddr, r.pages, err);
            })
    }

    release_region(RegionUser::Kernel, r.paddr, r.pages);
}

pub fn allocate_kernel_stack() -> *mut u8 {
    let heap_state = unsafe {
        match KERNEL_HEAP {
            KernelHeap::LargeHeap(ref mut heap_state) => heap_state,
            _ => panic!("Kernel large heap must be initialized before \
                memory::allocate_kernel_stack()")
        }
    };

    large_heap::allocate_kernel_stack(heap_state)
}

pub fn debug_print_allocator_stats() {
    use crate::terminal::console;

    unsafe {
        match KERNEL_HEAP {
            KernelHeap::InitialHeap(count) => {
                let _ = writeln!(console(), "Initial heap: {} / {}",
                    count, INITIAL_HEAP_LENGTH);
            },
            KernelHeap::LargeHeap(ref state) => {
                large_heap::debug_print_allocator_stats(state);
            },
            #[cfg(test)]
            KernelHeap::StdHeap => {
                unimplemented!()
            },
        }
    }
}

pub fn debug_print_physical_mem_stats() {
    use crate::terminal::console;

    // Safety: initialized once
    let state = unsafe { 
        REGION_STATE.as_ref().expect("memory::initialize() not called")
    };

    for region in state.free_regions.iter() {
        let _ = writeln!(console(), "FREE {:016x} - {:016x}",
            region.start,
            region.start + region.length.load(Relaxed) * PAGE_SIZE);
    }

    let free = state.free_page_count.load(Relaxed);
    let total = state.total_page_count;
    let used = total - free;

    let _ = writeln!(console(), "Pages: {} free / {} used / {} total",
        free, used, total);

    let _ = writeln!(console(), "Bytes: {} M free / {} M used / {} M total",
        free * PAGE_SIZE / 1048576,
        used * PAGE_SIZE / 1048576,
        total * PAGE_SIZE / 1048576);
}

/// C foreign interface.
pub mod ffi {
    use super::*;

    #[no_mangle]
    pub unsafe extern fn memory_alloc_aligned(size: u64, align: u64)
                                              -> *mut u8 {
        allocate(size as usize, align as usize)
    }

    #[no_mangle]
    pub unsafe extern fn memory_alloc(size: u64) -> *mut u8 {
        memory_alloc_aligned(size, 8)
    }

    #[no_mangle]
    pub unsafe extern fn memory_free(ptr: *mut u8) {
        deallocate(ptr, 0, 0)
    }

    #[no_mangle]
    pub extern fn memory_get_total_free() -> u64 {
        unimplemented!()
    }
}
