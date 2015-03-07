/*******************************************************************************
 *
 * kit/kernel/paging/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Kernel page management.

use core::prelude::*;
use core::cell::*;
use memory::Box;

pub mod generic;

#[cfg(any(doc, target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64 as target;

pub use self::target::Pageset;

static mut KERNEL_PAGESET: *const RefCell<Pageset> =
    0 as *const RefCell<Pageset>;

pub fn kernel_pageset<'a>() -> Ref<'a, Pageset> {
    unsafe { (*KERNEL_PAGESET).borrow() }
}

pub fn kernel_pageset_mut<'a>() -> RefMut<'a, Pageset> {
    unsafe { (*KERNEL_PAGESET).borrow_mut() }
}


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
