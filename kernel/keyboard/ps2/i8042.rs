/*******************************************************************************
 *
 * kit/kernel/keyboard/ps2/i8042.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Intel 8042 PS/2 controller driver.

/// Initializes the PS/2 keyboard state machine.
pub unsafe fn initialize() -> bool {
    ffi::ps2_8042_initialize() == 1
}

/// C interface. See `kit/kernel/include/ps2_8042.h`.
pub mod ffi {
    extern {
        pub fn ps2_8042_initialize() -> i8;
    }
}
