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

use memory::Rc;
use memory::rc::Contents as RcContents;

pub mod generic;

pub use self::generic::Pageset as GenericPageset;
pub use self::generic::{PageType, PagesetExt};

#[cfg(any(doc, target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64 as target;

pub use self::target::Pageset;

static mut INITIALIZED: bool = false;

static mut KERNEL_PAGESET:  Option<*mut RcContents<RefCell<Pageset>>> = None;
static mut CURRENT_PAGESET: Option<*mut RcContents<RefCell<Pageset>>> = None;

/// # Safety
///
/// Modifying or making assumptions about the kernel pageset can result in
/// system instability, data loss, and/or pointer aliasing.
pub unsafe fn kernel_pageset() -> Rc<RefCell<Pageset>> {
    let rc1 = Rc::from_raw(KERNEL_PAGESET.expect("paging not initialized"));
    let rc2 = rc1.clone();
    let _   = rc1.into_raw();

    rc2
}

/// Get the kernel pageset without borrowing.
unsafe fn kernel_pageset_unsafe<'a>() -> &'a Pageset {
    let rc   = Rc::from_raw(KERNEL_PAGESET.expect("paging not initialized"));
    let refr = rc.as_unsafe_cell().get().as_ref().unwrap();
    let _    = rc.into_raw();

    refr
}

/// # Safety
///
/// Modifying or making assumptions about the current pageset without checking
/// what the current pageset belongs to (i.e., the current process) is
/// dangerous.
pub unsafe fn current_pageset() -> Option<Rc<RefCell<Pageset>>> {
    CURRENT_PAGESET.map(|ptr| {
        let rc1 = Rc::from_raw(ptr);
        let rc2 = rc1.clone();
        let _   = rc1.into_raw();

        rc2
    })
}

/// # Safety
///
/// `process` assumes that the current pageset is the current process's pageset,
/// and that if there is no current process, the kernel pageset is active.
pub unsafe fn set_current_pageset(pageset: Option<Rc<RefCell<Pageset>>>) {
    let old = CURRENT_PAGESET.map(|ptr| Rc::from_raw(ptr));

    if let Some(pageset) = pageset {
        pageset.borrow_mut().load_into_hw();

        CURRENT_PAGESET = Some(pageset.into_raw());
    } else {
        let kernel_pageset = kernel_pageset();
        
        kernel_pageset.borrow_mut().load_into_hw();

        CURRENT_PAGESET = None;
    }

    drop(old); // explicitly drop it here
}

pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}


/// Call this on system initialization.
pub unsafe fn initialize() {
    if INITIALIZED {
        panic!("paging already initialized");
    }

    let pageset = Pageset::alloc_kernel();

    pageset.borrow_mut().load_into_hw();

    KERNEL_PAGESET  = Some(pageset.into_raw());
    CURRENT_PAGESET = None;
    INITIALIZED     = true;
}

/// C interface. See `kit/kernel/include/paging.h`.
pub mod ffi {
    use core::prelude::*;
    use core::cell::*;
    use core::ptr;

    use libc::c_void;

    use memory::Rc;
    use memory::rc::Contents as RcContents;

    use super::*;

    pub type PagesetCRef = *mut RcContents<RefCell<Pageset>>;

    #[no_mangle]
    pub unsafe extern fn paging_create_pageset(pageset: *mut PagesetCRef) {
        *pageset = Pageset::alloc().into_raw();
    }

    #[no_mangle]
    pub unsafe extern fn paging_clone_ref(pageset: PagesetCRef) -> PagesetCRef {
        let rc1 = Rc::from_raw(pageset);
        let rc2 = rc1.clone();
        let _   = rc1.into_raw();

        rc2.into_raw()
    }

    #[no_mangle]
    pub unsafe extern fn paging_drop_ref(pageset: *mut PagesetCRef) {
        if !(*pageset).is_null() {
            drop(Rc::from_raw(*pageset));

            *pageset = ptr::null::<RcContents<RefCell<Pageset>>>()
                           as PagesetCRef;
        }
    }

    #[no_mangle]
    pub unsafe extern fn
        paging_resolve_linear_address(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      physical_address: *mut u64) {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_map(pageset: PagesetCRef,
                                    linear_address: *const c_void,
                                    physical_address: u64,
                                    pages: u64,
                                    flags: u8) -> u64 {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_unmap(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      pages: u64) -> u64 {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_get_flags(pageset: PagesetCRef,
                                          linear_address: *const c_void,
                                          flags: *mut u8) -> i8 {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_set_flags(pageset: PagesetCRef,
                                          linear_address: *const c_void,
                                          pages: u64,
                                          flags: u8) -> u64 {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_get_current_pageset() -> PagesetCRef {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern fn paging_set_current_pageset(pageset: PagesetCRef) {
        unimplemented!()
    }
}
