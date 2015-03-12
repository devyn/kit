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

pub mod boxed;
pub mod rc;

pub use self::boxed::Box;
pub use self::rc::Rc;

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(mmap_buffer: *const u8, mmap_length: u32) {
    ffi::memory_initialize(mmap_buffer, mmap_length)
}


/// C interface. See `kit/kernel/include/memory.h`.
pub mod ffi {
    use libc::{size_t, c_void};

    extern {
        pub fn memory_initialize(mmap_buffer: *const u8, mmap_length: u32);

        pub fn memory_alloc_aligned(size: size_t, alignment: size_t)
                                    -> *mut c_void;

        pub fn memory_free(pointer: *mut c_void);
    }
}
