/*******************************************************************************
 *
 * kit/kernel/process.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Process management functions.

pub unsafe fn initialize() {
    ffi::process_initialize()
}

/// C interface. See `kit/kernel/include/archive.h`.
pub mod ffi {
    extern {
        pub fn process_initialize();
    }
}
