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
use core::cmp::{min, Ordering};
use core::alloc::{GlobalAlloc, Layout};

use alloc::vec::Vec;
use alloc::collections::{BTreeMap, BinaryHeap};

use crate::paging::{self, kernel_pageset, Pageset};
use crate::paging::{GenericPageset, PagesetExt, PageType};

use crate::multiboot::MmapEntry;
use crate::process::Id as ProcessId;
use crate::sync::Spinlock;
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
    alloc_regions: BTreeMap<PhysicalAddress, AllocRegionState>,
    free_regions: BinaryHeap<FreeRegion<PhysicalAddress>>,
    total_free: PageCount,
}

static mut REGION_STATE: Option<Spinlock<RegionState>> = None;

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

#[derive(Debug, PartialEq, Eq, Clone)]
struct FreeRegion<T> {
    start: T,
    length: PageCount
}

impl<T: Ord> PartialOrd for FreeRegion<T> {
    fn partial_cmp(&self, other: &FreeRegion<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for FreeRegion<T> {
    fn cmp(&self, other: &FreeRegion<T>) -> Ordering {
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

    REGION_STATE = Some(Spinlock::new(RegionState {
        total_free: free_regions.iter().fold(0, |s, r| s + r.length),
        free_regions,
        alloc_regions: BTreeMap::new(),
    }));
}

pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
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
