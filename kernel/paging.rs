/*******************************************************************************
 *
 * kit/kernel/paging.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Kernel page management.

/// Call this on system initialization.
pub unsafe fn initialize() {
    ffi::paging_initialize()
}

/// C interface. See `kit/kernel/include/paging.h`.
pub mod ffi {
    extern {
        pub fn paging_initialize();
    }
}
