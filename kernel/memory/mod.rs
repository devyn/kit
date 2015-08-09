/*******************************************************************************
 *
 * kit/kernel/memory/mod.rs
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

use libc::{size_t, c_void};

pub mod boxed;
pub mod rc;

pub use self::boxed::Box;
pub use self::rc::Rc;

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
#[lang = "exchange_malloc"]
pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    let ptr = ffi::memory_alloc_aligned(size as size_t, align as size_t);

    if ptr.is_null() {
        panic!("out of memory");
    }

    ptr as *mut u8
}

/// Deallocate to the heap.
#[lang = "exchange_free"]
pub unsafe fn deallocate(ptr: *mut u8, _size: usize, _align: usize) {
    ffi::memory_free(ptr as *mut c_void);
}

/// C interface. See `kit/kernel/include/memory.h`.
pub mod ffi {
    use libc::{size_t, c_void};

    extern {
        pub fn memory_initialize(mmap_buffer: *const u8, mmap_length: u32);

        pub fn memory_enable_large_heap();

        pub fn memory_alloc_aligned(size: size_t, alignment: size_t)
                                    -> *mut c_void;

        pub fn memory_free(pointer: *mut c_void);
    }
}
