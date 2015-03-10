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

use core::ptr::PtrExt;
use core::cell::*;
use core::mem;

use memory::Box;

pub mod generic;

pub use self::generic::Pageset as GenericPageset;
pub use self::generic::{PageType, PagesetExt};

#[cfg(any(doc, target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64 as target;

pub use self::target::Pageset;

static mut INITIALIZED: bool = false;

static mut KERNEL_PAGESET: *const RefCell<Pageset> =
    0 as *const RefCell<Pageset>;

pub fn kernel_pageset<'a>() -> Ref<'a, Pageset> {
    unsafe {
        if KERNEL_PAGESET.is_null() {
            panic!("paging not initialized");
        }

        (*KERNEL_PAGESET).borrow()
    }
}

pub unsafe fn kernel_pageset_unsafe<'a>() -> &'a Pageset {
    if KERNEL_PAGESET.is_null() {
        panic!("paging not initialized");
    }

    (*KERNEL_PAGESET).as_unsafe_cell().get().as_ref().unwrap()
}

/// # Unsafety
///
/// Modifying the kernel pageset can result in system instability, data loss,
/// and/or pointer aliasing.
pub unsafe fn kernel_pageset_mut<'a>() -> RefMut<'a, Pageset> {
    if KERNEL_PAGESET.is_null() {
        panic!("paging not initialized");
    }

    (*KERNEL_PAGESET).borrow_mut()
}

pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}


/// Call this on system initialization.
pub unsafe fn initialize() {
    let pageset = Box::new(RefCell::new(Pageset::new_kernel()));

    KERNEL_PAGESET = &*pageset;

    mem::forget(pageset);

    INITIALIZED = true;

    ffi::paging_initialize();

}

/// C interface. See `kit/kernel/include/paging.h`.
pub mod ffi {
    extern {
        pub fn paging_initialize();
    }
}
