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

pub use self::target::{Pageset, Error};

static mut INITIALIZED: bool = false;

static mut KERNEL_PAGESET: Option<*mut Pageset> = None;

static mut CURRENT_PAGESET: Option<*mut RcContents<RefCell<Pageset>>> = None;

/// A reference-counted, shared pageset.
///
/// This is required in order to be able to set a pageset as the current
/// pageset, because we need to be able to guarantee that it will still be valid
/// while the hardware is using it.
pub type RcPageset = Rc<RefCell<Pageset>>;

/// # Safety
///
/// Modifying or making assumptions about the kernel pageset can result in
/// system instability, data loss, and/or pointer aliasing.
///
/// Simultaneous (aliased) access to the kernel pageset is allowed, because it's
/// necessary. As such, it isn't wrapped in a RefCell.
pub unsafe fn kernel_pageset() -> &'static mut Pageset {
    &mut *KERNEL_PAGESET.expect("paging not initialized")
}

/// # Safety
///
/// Modifying or making assumptions about the current pageset without checking
/// what the current pageset belongs to (i.e., the current process) is
/// dangerous.
pub unsafe fn current_pageset() -> Option<RcPageset> {
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
pub unsafe fn set_current_pageset(pageset: Option<RcPageset>) {
    let old = CURRENT_PAGESET.map(|ptr| Rc::from_raw(ptr));

    if let Some(pageset) = pageset {
        pageset.borrow_mut().load_into_hw();

        CURRENT_PAGESET = Some(pageset.into_raw());
    } else {
        kernel_pageset().load_into_hw();

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

    KERNEL_PAGESET = Some((box Pageset::new_kernel()).into_raw());

    assert!(kernel_pageset().lookup(initialized as usize).is_some());

    kernel_pageset().load_into_hw();

    INITIALIZED     = true;
}

/// C interface. See `kit/kernel/include/paging.h`.
pub mod ffi {
    use core::prelude::*;
    use core::cell::*;
    use core::iter::{iterate, repeat};
    use core::default::Default;
    use core::ptr;

    use libc::c_void;

    use memory::Rc;
    use memory::rc::Contents as RcContents;

    use super::*;

    pub type PagesetCRef = *mut RcContents<RefCell<Pageset>>;

    fn from_flags(flags: u8) -> PageType {
        let readonly   = 0x01;
        let user       = 0x02;
        let executable = 0x04;

        let mut page_type = PageType::default();

        if flags & readonly   == 0 { page_type = page_type.writable(); }
        if flags & user       != 0 { page_type = page_type.user(); }
        if flags & executable != 0 { page_type = page_type.executable(); }

        page_type
    }

    fn to_flags(page_type: PageType) -> u8 {
        let readonly   = 0x01;
        let user       = 0x02;
        let executable = 0x04;

        let mut flags = 0;

        if !page_type.is_writable()   { flags |= readonly; }
        if  page_type.is_user()       { flags |= user; }
        if  page_type.is_executable() { flags |= executable; }

        flags
    }

    fn to_page_count(result: Result<(), Error>,
                     vaddr:  usize,
                     pages:  usize) -> u64 {
        use super::Error::*;

        let page_size = Pageset::page_size();

        (match result {
            Err(OutOfKernelRange(last_vaddr)) =>
                (last_vaddr - vaddr) / page_size,
            Err(OutOfUserRange(last_vaddr)) =>
                (last_vaddr - vaddr) / page_size,
            Ok(()) =>
                pages
        }) as u64
    }

    unsafe fn unpack(pageset: PagesetCRef) -> Option<RcPageset> {
        if pageset.is_null() {
            None
        } else {
            let rc1 = Rc::from_raw(pageset);
            let rc2 = rc1.clone();
            let _   = rc1.into_raw();

            Some(rc2)
        }
    }

    #[no_mangle]
    pub unsafe extern fn paging_create_pageset(pageset: *mut PagesetCRef) {
        *pageset = Pageset::alloc().into_raw();
    }

    #[no_mangle]
    pub unsafe extern fn paging_clone_ref(pageset: PagesetCRef) -> PagesetCRef {
        if pageset.is_null() {
            pageset
        } else {
            let rc1 = Rc::from_raw(pageset);
            let rc2 = rc1.clone();
            let _   = rc1.into_raw();

            rc2.into_raw()
        }
    }

    #[no_mangle]
    pub unsafe extern fn paging_drop_ref(pageset: *mut PagesetCRef) {
        if !(*pageset).is_null() {
            drop(Rc::from_raw(*pageset));

            *pageset = 0xdead as PagesetCRef; // to make it obvious if used
        }
    }

    #[no_mangle]
    pub unsafe extern fn
        paging_resolve_linear_address(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      physical_address: *mut u64) -> i8 {
        let pageset = unpack(pageset);
        let pageset = pageset.as_ref().map(|x| x.borrow());
        let pageset = pageset.as_ref().map(|x| &**x)
                                      .unwrap_or(kernel_pageset());
        let vaddr   = linear_address as usize;

        if let Some(paddr) = pageset.lookup(vaddr) {
            *physical_address = paddr as u64;
            1
        } else {
            0
        }
    }

    #[no_mangle]
    pub unsafe extern fn paging_map(pageset: PagesetCRef,
                                    linear_address: *const c_void,
                                    physical_address: u64,
                                    pages: u64,
                                    flags: u8) -> u64 {
        let pageset     = unpack(pageset);
        let mut pageset = pageset.as_ref().map(|x| x.borrow_mut());
        let pageset     = pageset.as_mut().map(|x| &mut **x)
                                          .unwrap_or(kernel_pageset());

        let vaddr       = linear_address as usize;
        let paddr       = physical_address as usize;
        let pages       = pages as usize;
        let page_size   = Pageset::page_size();

        to_page_count(
            pageset.map_pages_with_type(
                vaddr,
                iterate(paddr, |paddr| paddr + page_size).take(pages),
                from_flags(flags)),
            vaddr, pages)
    }

    #[no_mangle]
    pub unsafe extern fn paging_unmap(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      pages: u64) -> u64 {
        let pageset     = unpack(pageset);
        let mut pageset = pageset.as_ref().map(|x| x.borrow_mut());
        let pageset     = pageset.as_mut().map(|x| &mut **x)
                                          .unwrap_or(kernel_pageset());

        let vaddr       = linear_address as usize;
        let pages       = pages as usize;

        to_page_count(pageset.unmap_pages(vaddr, pages), vaddr, pages)
    }

    #[no_mangle]
    pub unsafe extern fn paging_get_flags(pageset: PagesetCRef,
                                          linear_address: *const c_void,
                                          flags: *mut u8) -> i8 {
        let pageset = unpack(pageset);
        let pageset = pageset.as_ref().map(|x| x.borrow());
        let pageset = pageset.as_ref().map(|x| &**x)
                                      .unwrap_or(kernel_pageset());

        let vaddr   = linear_address as usize;

        if let Some((_, page_type)) = pageset.get(vaddr) {
            *flags = to_flags(page_type);
            1
        } else {
            0
        }
    }

    #[no_mangle]
    pub unsafe extern fn paging_set_flags(pageset: PagesetCRef,
                                          linear_address: *const c_void,
                                          pages: u64,
                                          flags: u8) -> u64 {
        let pageset     = unpack(pageset);
        let mut pageset = pageset.as_ref().map(|x| x.borrow_mut());
        let pageset     = pageset.as_mut().map(|x| &mut **x)
                                          .unwrap_or(kernel_pageset());

        let vaddr       = linear_address as usize;
        let pages       = pages as usize;

        to_page_count(
            pageset.set_page_types(vaddr,
                                   repeat(from_flags(flags)).take(pages)),
            vaddr, pages)
    }

    #[no_mangle]
    pub unsafe extern fn paging_get_current_pageset() -> PagesetCRef {
        let null: *const RcContents<RefCell<Pageset>> = ptr::null();
        let null = null as PagesetCRef;

        current_pageset().map(|p| p.into_raw()).unwrap_or(null)
    }

    #[no_mangle]
    pub unsafe extern fn paging_set_current_pageset(pageset: PagesetCRef) {
        set_current_pageset(unpack(pageset));
    }
}
