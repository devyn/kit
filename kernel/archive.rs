/*******************************************************************************
 *
 * kit/kernel/archive.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Kit archive (init files) loader.

use multiboot;

pub unsafe fn initialize(modules: *const multiboot::Module,
                         modules_count: u32) -> bool {

    ffi::archive_initialize(modules_count as u64, modules) == 1
}

/// C interface. See `kit/kernel/include/archive.h`.
pub mod ffi {
    use multiboot;

    extern {
        pub fn archive_initialize(modules_count: u64,
                                  modules: *const multiboot::Module) -> i8;
    }
}
