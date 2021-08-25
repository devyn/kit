/*******************************************************************************
 *
 * kit/kernel/memory.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Physical memory management and kernel heap.

use core::mem;
use core::cmp::{min, Ordering};
use core::alloc::{GlobalAlloc, Layout};

use alloc::vec::Vec;
use alloc::collections::{BTreeMap, BinaryHeap};

use crate::paging::{self, kernel_pageset, Pageset};
use crate::paging::{GenericPageset, PagesetExt, PageType};

use crate::multiboot::MmapEntry;
use crate::process::Id as ProcessId;
use crate::sync::CriticalSpinlock;
use crate::constants::KERNEL_LOW_END;

/// The first "safe" physical address. Memory below this is not likely to be
/// safe for general use, and may include parts of the kernel image among other
/// things.
///
/// 0x0 to SAFE_BOUNDARY are identity-mapped starting at 0xffff_ffff_8000_0000,
/// so we avoid that region
const SAFE_BOUNDARY: usize = KERNEL_LOW_END as usize;

const INITIAL_HEAP_LENGTH: usize = 131072;

extern {
    static mut MEMORY_INITIAL_HEAP: [u8; INITIAL_HEAP_LENGTH];
}

#[global_allocator]
static mut KERNEL_HEAP: KernelHeap = KernelHeap::InitialHeap(0);

#[derive(Debug)]
enum KernelHeap {
    InitialHeap(usize),
    LargeHeap(HeapState)
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
#[alloc_error_handler]
fn handle_alloc_error(layout: Layout) -> ! {
    panic!("Memory allocation failed: {:?}", layout);
}

#[derive(Debug)]
struct HeapState {
    start: *mut u8,
    end: *mut u8,
    length: usize,
    // For allocating stacks
    stacks_start: *mut u8,
    stacks_end: *mut u8,
    // Keeping track of what we allocated
    regions: CriticalSpinlock<Vec<(PhysicalAddress, PageCount)>>,
}

#[derive(Debug)]
struct RegionState {
    alloc_regions: BTreeMap<PhysicalAddress, AllocRegionState>,
    free_regions: BinaryHeap<FreeRegion>,
    total_free: PageCount,
}

static mut REGION_STATE: Option<CriticalSpinlock<RegionState>> = None;

pub type PhysicalAddress = usize;
pub type PageCount = usize;

#[derive(Debug)]
struct AllocRegionState {
    length: PageCount,
    users: Vec<RegionUser>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionUser {
    Kernel,
    Process(ProcessId)
}

#[derive(Debug, PartialEq, Eq)]
struct FreeRegion {
    start: PhysicalAddress,
    length: PageCount
}

impl PartialOrd for FreeRegion {
    fn partial_cmp(&self, other: &FreeRegion) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FreeRegion {
    fn cmp(&self, other: &FreeRegion) -> Ordering {
        if self.length == other.length {
            self.start.cmp(&other.start)
        } else {
            other.length.cmp(&self.length)
        }
    }
}

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(mmap_buffer: *const u8, mmap_length: u32) {
    let mut current_mmap = mmap_buffer;
    let mmap_end = mmap_buffer.offset(mmap_length as isize);

    let page_size = Pageset::page_size();

    let mut free_regions = BinaryHeap::with_capacity(16);

    while current_mmap < mmap_end {
        let entry_ptr: *const MmapEntry = mem::transmute(current_mmap);
        let entry = entry_ptr.as_ref().unwrap();

        current_mmap = current_mmap.offset(entry.size as isize + 4);

        let addr = entry.addr as usize;
        let len  = entry.len  as usize;

        // Align physical base address to page size.
        let mut physical_base =
            if addr % page_size != 0 {
                (addr / page_size + 1) * page_size
            } else {
                addr
            };

        // Remove remainder from length and count pages.
        let mut pages = (len - (addr % page_size)) / page_size;

        // If the base starts before SAFE_BOUNDARY, remove the pages before that
        // (and make sure we still have pages left).
        if physical_base < SAFE_BOUNDARY {
            let diff = (SAFE_BOUNDARY - physical_base) / page_size;

            if diff < pages {
                physical_base += diff * page_size;
                pages         -= diff;
            } else {
                continue; // skip this entry
            }
        }

        // If the entry is marked as available and has at least one page, add it
        // to the free regions.
        if entry.is_available() && pages > 0 {
            free_regions.push(FreeRegion {
                start: physical_base,
                length: pages
            });
        }
    }

    REGION_STATE = Some(CriticalSpinlock::new(RegionState {
        total_free: free_regions.iter().fold(0, |s, r| s + r.length),
        free_regions,
        alloc_regions: BTreeMap::new(),
    }));
}

const LARGE_HEAP_START: usize = 0xffff_ffff_9000_0000;
const STACKS_START: usize = 0xffff_ffff_f000_0000;
const BUFZONE_SIZE: usize = 0x4000;

pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    match KERNEL_HEAP {
        KernelHeap::InitialHeap(ref mut counter) =>
            initial_heap_allocate(counter, size, align),
        KernelHeap::LargeHeap(ref mut state) =>
            large_heap_allocate(state, size, align)
    }
}

pub unsafe fn deallocate(_ptr: *mut u8, _size: usize, _align: usize) {
    // TODO
}

fn align_addr(mut addr: usize, align: usize) -> usize {
    if addr % align != 0 {
        addr += align - (addr % align);
    }
    addr
}

unsafe fn initial_heap_allocate(counter: &mut usize, size: usize, align: usize)
                                -> *mut u8 {

    let new_counter = align_addr(*counter, align);

    if new_counter + size >= INITIAL_HEAP_LENGTH {
        panic!("out of memory in initial heap!");
    }

    let ptr = (&mut MEMORY_INITIAL_HEAP[new_counter]) as *mut u8;

    *counter = new_counter + size;

    ptr
}

unsafe fn large_heap_allocate(state: &mut HeapState, size: usize, align: usize)
                              -> *mut u8 {

    // Note: there are parts in here where allocations can happen. Avoid taking
    // values from state between when these allocations could happen.

    // Don't even try to run any allocation if we're already doing an operation.
    if let Some(mut regions) = state.regions.try_lock() {
        // The maximum amount of room we will need to align is (align - 1)
        // bytes, so reserve that just in case.
        //
        // We can't predict what the alignment will be after the
        // kernel_acquire_and_map() call, since mapping pages sometimes requires
        // memory allocation, but it should always fit within the BUFZONE_SIZE.
        let min_end = state.start as usize + state.length +
            (align - 1) + size + BUFZONE_SIZE;

        if min_end >= state.end as usize {
            let page_size = Pageset::page_size();
            let needed_bytes = min_end - state.end as usize;
            let mut needed_pages = needed_bytes / page_size;

            if needed_bytes % page_size != 0 {
                needed_pages += 1;
            }

            // Warning: allocation possible here.
            let result = 
                kernel_acquire_and_map(state.end, needed_pages, &mut *regions);

            // If we get an error, still add whatever number of pages we could
            // allocate
            let allocated_pages = match result {
                Ok(()) => needed_pages,
                Err(some_pages) => some_pages
            };

            state.end =
                (state.end as usize + (allocated_pages * page_size)) as *mut u8;
        }
    }

    // It's possible that we failed to actually acquire the pages we need, but
    // try to proceed anyway and hope the buffer zone is enough:

    let start = state.start as usize;

    state.length = align_addr(start + state.length, align) - start;

    let alloc_addr = (start + state.length) as *mut u8;

    // It's possible that even after our best attempt to allocate more memory,
    // we still don't have enough. In that case we have to panic.
    if (start + state.length + size) >= state.end as usize {
        panic!("ran out of bufzone memory");
    }

    state.length += size;

    alloc_addr
}

pub unsafe fn enable_large_heap() {
    assert!(paging::initialized());

    if let KernelHeap::LargeHeap(_) = KERNEL_HEAP {
        // Already enabled, don't need to do anything.
        return;
    }

    let bufzone_pages = BUFZONE_SIZE / Pageset::page_size();

    let mut regions = vec![];

    kernel_acquire_and_map(
        LARGE_HEAP_START as *mut u8,
        bufzone_pages,
        &mut regions,
    )
    .expect("Failed to initialize large heap bufzone");

    KERNEL_HEAP = KernelHeap::LargeHeap(HeapState {
        start: LARGE_HEAP_START as *mut u8,
        end: (LARGE_HEAP_START + BUFZONE_SIZE) as *mut u8,
        stacks_start: STACKS_START as *mut u8,
        stacks_end: STACKS_START as *mut u8,
        length: 0,
        regions: CriticalSpinlock::new(regions),
    });
}

pub fn acquire_region(owner: RegionUser, pages: PageCount)
                      -> Option<(PhysicalAddress, PageCount)> {
    let page_size = Pageset::page_size();

    // Safety: initialized once
    let state_lock = unsafe { 
        REGION_STATE.as_ref().expect("memory::initialize() not called")
    };

    if let Some(mut state) = state_lock.try_lock() {
        if state.total_free < pages {
            return None;
        }

        if let Some(free_region) = state.free_regions.pop() {
            if free_region.length > pages {
                state.free_regions.push(FreeRegion {
                    start: free_region.start + (pages * page_size),
                    length: free_region.length - pages
                });
            }

            let captured_length = min(free_region.length, pages);

            state.alloc_regions.insert(free_region.start, AllocRegionState {
                length: captured_length,
                users: vec![owner]
            });

            state.total_free -= captured_length;

            Some((free_region.start, captured_length))
        } else {
            None
        }
    } else {
        // Nested calls to acquire_region are not allowed.
        None
    }
}

pub fn release_region(user: RegionUser, paddr: PhysicalAddress) {
    unimplemented!()
}

/// Returns the number of pages successfully allocated on error
fn kernel_acquire_and_map(
    vaddr: *mut u8,
    pages: PageCount,
    regions: &mut Vec<(PhysicalAddress, PageCount)>,
) -> Result<(), PageCount> {

    let page_size = Pageset::page_size();

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
                    (got_paddr..).step_by(page_size).take(got_pages),
                    PageType::default().writable(),
                )
                .expect("unable to map acquired pages into kernel space")
        }

        cur_vaddr += got_pages * page_size;
        cur_pages -= got_pages;

        regions.push((got_paddr, got_pages));
    }

    Ok(())
}

pub const KSTACK_SIZE: usize = 8192;
pub const KSTACK_ALIGN: usize = 16;

const_assert!(KSTACK_SIZE < isize::MAX as usize);
const_assert!(KSTACK_SIZE > 0);
const_assert!(KSTACK_ALIGN & (KSTACK_ALIGN-1) == 0);
const_assert!(KSTACK_ALIGN > 0);

pub fn allocate_kernel_stack() -> *mut u8 {
    let heap_state = unsafe {
        match KERNEL_HEAP {
            KernelHeap::LargeHeap(ref mut heap_state) => heap_state,
            _ => panic!("Kernel large heap must be initialized before \
                memory::allocate_kernel_stack()")
        }
    };

    assert_eq!(KSTACK_SIZE % Pageset::page_size(), 0,
        "Kernel stack size must be a multiple of the page size");

    let pages = KSTACK_SIZE / Pageset::page_size();

    let mut regions = heap_state.regions.lock();

    let new_stack = heap_state.stacks_end;

    // Allocate pages to the stack area
    kernel_acquire_and_map(new_stack, pages, &mut regions)
        .unwrap_or_else(|_| panic!("Out of memory"));

    // Increment, skip one page for next time, stacks have guard page on either
    // side
    unsafe {
        heap_state.stacks_end = heap_state.stacks_end.offset(
            ((pages + 1) * Pageset::page_size()) as isize);
    }

    // Actually return the stack pointer (going down)
    unsafe {
        new_stack.offset(KSTACK_SIZE as isize)
    }
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
        memory_alloc_aligned(size, 1)
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
