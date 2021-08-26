/*******************************************************************************
 *
 * kit/kernel/keyboard/ps2/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! PS/2 keyboard driver.

pub mod i8042;

/// Initializes the PS/2 keyboard state machine.
pub unsafe fn initialize() {
    ffi::ps2key_initialize()
}

/// C interface. See `kit/kernel/include/ps2key.h`.
pub mod ffi {
    extern {
        pub fn ps2key_initialize();
    }
}
