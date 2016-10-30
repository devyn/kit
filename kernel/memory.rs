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
use core::cmp::Ordering;
use collections::{Vec, BTreeMap, BinaryHeap};

use paging::{Pageset, GenericPageset};
use multiboot::MmapEntry;
use process::Id as ProcessId;

/// The first "safe" physical address. Memory below this is not likely to be
/// safe for general use, and may include parts of the kernel image among other
/// things.
const SAFE_BOUNDARY: usize = 0x400000;

const INITIAL_HEAP_LENGTH: usize = 131072;

extern {
    static mut MEMORY_INITIAL_HEAP: [u8; INITIAL_HEAP_LENGTH];
}

static mut KERNEL_HEAP: KernelHeap = KernelHeap::InitialHeap(0);

enum KernelHeap {
    InitialHeap(usize),
    LargeHeap(HeapState)
}

struct HeapState {
    start: *const u8,
    end: *const u8,
    length: usize,
    can_grow: bool
}

static mut REGIONS: Option<BTreeMap<PageNumber, Region>> = None;
static mut FREE_REGIONS: Option<BinaryHeap<FreeRegion>> = None;

type PageNumber = usize;
type PageCount = usize;

struct PhysicalMemoryState {
    regions: BTreeMap<PageNumber, Region>,
    free_regions: BinaryHeap<FreeRegion>
}

struct Region {
    start: PageNumber,
    length: PageCount,
    users: Vec<ProcessId>
}

#[derive(PartialEq, Eq)]
struct FreeRegion {
    start: PageNumber,
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
            self.length.cmp(&other.length)
        }
    }
}

// remove owner on drop, if no more users then add free region
pub struct PhysicalMemory {
    start: PageNumber,
    length: PageCount,
    owner: ProcessId
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
                start: physical_base / page_size,
                length: pages
            });
        }
    }

    FREE_REGIONS = Some(free_regions);
}

const LARGE_HEAP_START: usize = 0xffff_ffff_8100_0000;
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

fn initial_heap_allocate(counter: &mut usize, size: usize, align: usize)
                         -> *mut u8 {

    let mut new_counter = *counter;

    // Align the counter to the requested alignment
    if new_counter % align != 0 {
        new_counter += align - (new_counter % align);
    }

    if new_counter + size >= INITIAL_HEAP_LENGTH {
        panic!("out of memory in initial heap!");
    }

    let ptr = unsafe { (&mut MEMORY_INITIAL_HEAP[new_counter]) as *mut u8 };

    *counter = new_counter + size;

    ptr
}

fn large_heap_allocate(state: &mut HeapState, size: usize, align: usize)
                       -> *mut u8 {
    unimplemented!()
}

pub fn enable_large_heap() {
    unimplemented!()
}

pub fn acquire_region(pages: usize) -> Option<(usize, usize)> {
    unimplemented!()
}

pub fn release_region(paddr: usize, pages: usize) {
    unimplemented!()
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
