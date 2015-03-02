/*******************************************************************************
 *
 * kit/kernel/scheduler.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Time and event based task scheduler.

pub unsafe fn enter() {
    ffi::scheduler_enter()
}

/// C interface. See `kit/kernel/include/scheduler.h`.
pub mod ffi {
    extern {
        pub fn scheduler_enter();
    }
}
