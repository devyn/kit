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

use core::slice;
use core::ptr;

use multiboot;
use c_ffi::CStr;

pub unsafe fn initialize(modules: *const multiboot::Module,
                         modules_count: u32) -> bool {

    ffi::archive_initialize(modules_count as u64, modules) == 1
}

pub struct Archive {
    header: *const ffi::ArchiveHeader,
}

impl Archive {
    pub fn get<'a>(&self, filename: CStr<'a>) -> Option<&[u8]> {
        let mut buffer: *const u8 = ptr::null();
        let mut length: u64       = 0;

        unsafe {
            if ffi::archive_get(self.header, filename.as_ptr(),
                   &mut buffer, &mut length) == 1 {
                Some(slice::from_raw_parts(buffer, length as usize))
            } else {
                None
            }
        }
    }
}

pub fn system() -> Archive {
    Archive { header: ffi::archive_system }
}

/// C interface. See `kit/kernel/include/archive.h`.
pub mod ffi {
    use multiboot;

    use c_ffi::c_char;

    #[repr(C)]
    pub enum ArchiveHeader {
        __variant1,
        __variant2,
    }

    extern {
        pub static archive_system: *const ArchiveHeader;

        pub fn archive_initialize(modules_count: u64,
                                  modules: *const multiboot::Module) -> i8;

        pub fn archive_get(header: *const ArchiveHeader,
                           entry_name: *const c_char,
                           buffer: *mut *const u8,
                           length: *mut u64) -> i8;
    }
}
