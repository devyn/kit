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

use core::mem;
use core::cmp::min;
use core::alloc::{GlobalAlloc, Layout};

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;

use alloc::vec::Vec;

use crate::paging::{self, kernel_pageset};
use crate::paging::{PagesetExt, PageType};
use crate::paging::PAGE_SIZE;

use crate::multiboot::MmapEntry;
use crate::process::Id as ProcessId;
use crate::sync::LockFreeList;
use crate::sync::lock_free_list::Node;
use crate::constants::KERNEL_LOW_END;

pub mod pool;

mod large_heap;

/// The first "safe" physical address. Memory below this is not likely to be
/// safe for general use, and may include parts of the kernel image among other
/// things.
///
/// 0x0 to SAFE_BOUNDARY are identity-mapped starting at 0xffff_ffff_8000_0000,
/// so we avoid that region
const SAFE_BOUNDARY: usize = KERNEL_LOW_END as usize;

const INITIAL_HEAP_LENGTH: usize = 131072;

pub const KSTACK_SIZE: usize = 8192;
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
static mut KERNEL_HEAP: KernelHeap = KernelHeap::InitialHeap(0);

#[cfg(test)]
static mut KERNEL_HEAP: KernelHeap = KernelHeap::InitialHeap(0);

#[derive(Debug)]
enum KernelHeap {
    InitialHeap(usize),
    LargeHeap(large_heap::HeapState)
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
    alloc_regions: LockFreeList<(PhysicalAddress, AllocRegionState)>,
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
    length: PageCount,
    users: Vec<RegionUser>
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

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(mmap_buffer: *const u8, mmap_length: u32) {
    let mut current_mmap = mmap_buffer;
    let mmap_end = mmap_buffer.offset(mmap_length as isize);

    let mut free_regions = LockFreeList::new();

    while current_mmap < mmap_end {
        let entry_ptr: *const MmapEntry = mem::transmute(current_mmap);
        let entry = entry_ptr.as_ref().unwrap();

        current_mmap = current_mmap.offset(entry.size as isize + 4);

        let addr = entry.addr as usize;
        let len  = entry.len  as usize;

        // Align physical base address to page size.
        let mut physical_base = align_addr(addr, PAGE_SIZE);

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

        // If the entry is marked as available and has at least one page, add it
        // to the free regions.
        if entry.is_available() && pages > 0 {
            free_regions.push(Node::new(FreeRegion {
                start: physical_base,
                length: AtomicUsize::new(pages)
            }));
        }
    }

    let total_page_count = free_regions.iter().fold(0,
        |s, r| s + r.length.load(Relaxed));

    REGION_STATE = Some(RegionState {
        total_page_count,
        free_page_count: AtomicUsize::new(total_page_count),
        free_regions,
        alloc_regions: LockFreeList::new(),
    });
}

pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    debug!("allocate({}, {})", size, align);
    match KERNEL_HEAP {
        KernelHeap::InitialHeap(ref mut counter) =>
            initial_heap_allocate(counter, size, align),
        KernelHeap::LargeHeap(ref state) =>
            large_heap::allocate(state, size, align)
    }
}

pub unsafe fn deallocate(_ptr: *mut u8, _size: usize, _align: usize) {
    // TODO
}

const fn align_addr(mut addr: usize, align: usize) -> usize {
    if addr % align != 0 {
        addr += align - (addr % align);
    }
    addr
}

const fn align_addr_down(mut addr: usize, align: usize) -> usize {
    if addr % align != 0 {
        addr -= addr % align;
    }
    addr
}

unsafe fn initial_heap_allocate(counter: &mut usize, size: usize, align: usize)
                                -> *mut u8 {

    let new_counter = align_addr(*counter, align);

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
        debug!("free_page_count={} < {}", free_page_count, pages);
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

        if res.is_ok() {
            break;
        }
    }

    if acq_pages > 0 {
        state.free_page_count.fetch_sub(acq_pages, Relaxed);

        state.alloc_regions.push(Node::new((alloc_start, AllocRegionState {
            length: acq_pages,
            users: vec![owner]
        })));

        Some((alloc_start, acq_pages))
    } else {
        debug!("No free physical region available.");
        None
    }
}

pub fn release_region(user: RegionUser, paddr: PhysicalAddress) {
    unimplemented!()
}

/// Returns the number of pages successfully allocated on error
fn kernel_acquire_and_map<F>(
    vaddr: *mut u8,
    pages: PageCount,
    mut push_region: F,
) -> Result<(), PageCount>
    where F: FnMut(PhysicalAddress, PageCount) {

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

        cur_vaddr += got_pages * PAGE_SIZE;
        cur_pages -= got_pages;

        push_region(got_paddr, got_pages);
    }

    Ok(())
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
