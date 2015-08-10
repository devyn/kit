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

//! Kernel memory management.

use paging;

use c_ffi::{size_t, uint64_t, c_void};

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(mmap_buffer: *const u8, mmap_length: u32) {
    ffi::memory_initialize(mmap_buffer, mmap_length)
}

/// Enables the 'large' (non-static) heap. Requires paging to have been
/// initialized first.
pub fn enable_large_heap() {
    assert!(paging::initialized());

    unsafe { ffi::memory_enable_large_heap(); }
}

/// Allocate from the heap.
pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    let ptr = ffi::memory_alloc_aligned(size as size_t, align as size_t);

    if ptr.is_null() {
        panic!("out of memory");
    }

    ptr as *mut u8
}

/// Deallocate to the heap.
pub unsafe fn deallocate(ptr: *mut u8, _size: usize, _align: usize) {
    ffi::memory_free(ptr as *mut c_void);
}

/// Acquire a contiguous physical memory region at most `pages` long.
///
/// Returns `None` if are no remaining free physical memory regions.
///
/// Otherwise, returns `Some((paddr, acq_pages))` where `paddr` is the first
/// physical address, and `acq_pages` is the number of actual pages acquired
/// equal to or less than the requested `pages`.
pub fn acquire_region(pages: usize) -> Option<(usize, usize)> {
    let mut paddr: uint64_t = 0;

    let acq_pages = unsafe {
        ffi::memory_free_region_acquire(pages as uint64_t, &mut paddr)
    };

    if acq_pages == 0 {
        None
    } else {
        Some((paddr as usize, acq_pages as usize))
    }
}

/// Release a contiguous physical memory region to the pool for allocation
/// later.
pub fn release_region(paddr: usize, pages: usize) {
    unsafe {
        ffi::memory_free_region_release(paddr as uint64_t, pages as uint64_t)
    }
}

/// C interface. See `kit/kernel/include/memory.h`.
pub mod ffi {
    use c_ffi::{size_t, uint64_t, c_void};

    extern {
        pub fn memory_initialize(mmap_buffer: *const u8, mmap_length: u32);

        pub fn memory_enable_large_heap();

        pub fn memory_alloc_aligned(size: size_t, alignment: size_t)
                                    -> *mut c_void;

        pub fn memory_free(pointer: *mut c_void);

        pub fn memory_free_region_acquire(pages: uint64_t,
                                          physical_base: *mut uint64_t)
                                          -> uint64_t;

        pub fn memory_free_region_release(physical_base: uint64_t,
                                          pages: uint64_t);
    }
}
