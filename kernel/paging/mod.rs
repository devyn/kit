/*******************************************************************************
 *
 * kit/kernel/paging/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Kernel page management.

use alloc::sync::Arc;
use alloc::boxed::Box;

use crate::sync::Spinlock;

pub mod generic;

pub use self::generic::Pageset as GenericPageset;
pub use self::generic::{PageType, PagesetExt};

#[cfg(any(doc, target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64 as target;

pub use self::target::{PAGE_SIZE, Pageset, Error};

static mut INITIALIZED: bool = false;

static mut KERNEL_PAGESET: Option<*mut Pageset> = None;

static mut CURRENT_PAGESET: Option<RcPageset> = None;

/// A reference-counted, shared pageset.
///
/// This is required in order to be able to set a pageset as the current
/// pageset, because we need to be able to guarantee that it will still be valid
/// while the hardware is using it.
pub type RcPageset = Arc<Spinlock<Pageset>>;

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
    CURRENT_PAGESET.clone()
}

/// # Safety
///
/// `process` assumes that the current pageset is the current process's pageset,
/// and that if there is no current process, the kernel pageset is active.
pub unsafe fn set_current_pageset(pageset: Option<RcPageset>) {
    let old: Option<RcPageset> = CURRENT_PAGESET.clone();

    if let Some(pageset) = pageset {
        pageset.lock().load_into_hw();

        CURRENT_PAGESET = Some(pageset);
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

    KERNEL_PAGESET = Some(Box::into_raw(box Pageset::new_kernel()));

    assert!(kernel_pageset().lookup(initialized as usize).is_some());

    kernel_pageset().load_into_hw();

    INITIALIZED     = true;
}

/// C interface. See `kit/kernel/include/paging.h`.
pub mod ffi {
    use core::iter::repeat;
    use core::ptr;
    use core::mem;

    use crate::c_ffi::c_void;
    use crate::sync::Spinlock;

    use alloc::sync::Arc;

    use super::*;

    #[repr(C)]
    pub struct PagesetCRef(*const c_void);

    impl PagesetCRef {
        pub fn new(rc_pageset: Arc<Spinlock<Pageset>>) -> PagesetCRef {
            PagesetCRef(Arc::into_raw(rc_pageset) as *const c_void)
        }

        pub fn to_rc(&self) -> Arc<Spinlock<Pageset>> {
            if self.is_null() {
                panic!("Tried to call into_rc() on null PagesetCRef");
            }

            unsafe {
                let PagesetCRef(ptr) = *self;
                Arc::increment_strong_count(ptr as *const Spinlock<Pageset>);
                Arc::from_raw(ptr as *const Spinlock<Pageset>)
            }
        }

        pub fn to_option(&self) -> Option<Arc<Spinlock<Pageset>>> {
            if self.is_null() {
                None
            } else {
                Some(self.to_rc())
            }
        }

        pub fn into_rc(self) -> Arc<Spinlock<Pageset>> {
            if self.is_null() {
                panic!("Tried to call into_rc() on null PagesetCRef");
            }

            unsafe {
                Arc::from_raw(self.0 as *const Spinlock<Pageset>)
            }
        }

        pub fn is_null(&self) -> bool {
            let PagesetCRef(ptr) = *self;

            ptr.is_null()
        }
    }

    impl Clone for PagesetCRef {
        fn clone(&self) -> PagesetCRef {
            PagesetCRef::new(self.to_rc())
        }
    }

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

    #[no_mangle]
    pub unsafe extern fn paging_create_pageset(pageset: *mut PagesetCRef) {
        *pageset = PagesetCRef::new(Pageset::alloc());
    }

    #[no_mangle]
    pub unsafe extern fn paging_clone_ref(pageset: PagesetCRef) -> PagesetCRef {
        if pageset.is_null() {
            pageset
        } else {
            PagesetCRef::new(pageset.to_rc())
        }
    }

    #[no_mangle]
    pub unsafe extern fn paging_drop_ref(pageset: *mut PagesetCRef) {
        if !(*pageset).is_null() {
            // Replace with 0xdead to make it obvious if it's used again
            drop(mem::replace(&mut *pageset,
                              PagesetCRef(0xdead as *const c_void)).into_rc())
        }
    }

    #[no_mangle]
    pub unsafe extern fn
        paging_resolve_linear_address(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      physical_address: *mut u64) -> i8 {
        let pageset = pageset.to_option();
        let pageset = pageset.as_ref().map(|x| x.lock());
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
        let pageset     = pageset.to_option();
        let mut pageset = pageset.as_ref().map(|x| x.lock());
        let pageset     = pageset.as_mut().map(|x| &mut **x)
                                          .unwrap_or(kernel_pageset());

        let vaddr       = linear_address as usize;
        let paddr       = physical_address as usize;
        let pages       = pages as usize;
        let page_size   = Pageset::page_size();

        to_page_count(
            pageset.map_pages_with_type(
                vaddr,
                (paddr..).step_by(page_size).take(pages),
                from_flags(flags)),
            vaddr, pages)
    }

    #[no_mangle]
    pub unsafe extern fn paging_unmap(pageset: PagesetCRef,
                                      linear_address: *const c_void,
                                      pages: u64) -> u64 {
        let pageset     = pageset.to_option();
        let mut pageset = pageset.as_ref().map(|x| x.lock());
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
        let pageset = pageset.to_option();
        let pageset = pageset.as_ref().map(|x| x.lock());
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
        let pageset     = pageset.to_option();
        let mut pageset = pageset.as_ref().map(|x| x.lock());
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
        current_pageset().map(|p| PagesetCRef::new(p))
            .unwrap_or(PagesetCRef(ptr::null()))
    }

    #[no_mangle]
    pub unsafe extern fn paging_set_current_pageset(pageset: PagesetCRef) {
        set_current_pageset(pageset.to_option());
    }
}
