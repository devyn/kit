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

use crate::multiboot;
use crate::c_ffi::CStr;

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
    Archive {
        header: unsafe { ffi::archive_system }
    }
}

/// C interface. See `kit/kernel/include/archive.h`.
pub mod ffi {
    use crate::multiboot;
    use crate::archive::utils;

    use crate::c_ffi::{c_int, c_char, int64_t, CStr, c_void};

    use alloc::vec::Vec;

    #[repr(C)]
    pub struct ArchiveHeader(c_void); // data undefined

    extern {
        pub static archive_system: *const ArchiveHeader;

        pub fn archive_initialize(modules_count: u64,
                                  modules: *const multiboot::Module) -> i8;

        pub fn archive_get(header: *const ArchiveHeader,
                           entry_name: *const c_char,
                           buffer: *mut *const u8,
                           length: *mut u64) -> i8;
    }

    #[no_mangle]
    pub unsafe extern fn archive_utils_spawn(filename: *const c_char,
                                             argc: c_int,
                                             argv: *const *const c_char)
                                             -> int64_t {

        // XXX: This is dangerous in so many ways.
        // Beyond FIXME status. Get rid of it.

        let filename = CStr::from_ptr(filename);

        let argv: Vec<Vec<u8>> = (0..argc as isize).map(|i| {
            CStr::from_ptr(*argv.offset(i)).as_bytes().iter().map(|&b| b).collect()
        }).collect();

        let argv_ptrs: Vec<&[u8]> = argv.iter().map(|v| &v[..]).collect();

        utils::spawn(filename, &argv_ptrs)
            .map(|pid| pid as i64)
            .unwrap_or_else(|e| -((e as u32) as i64))
    }
}

/// Archive utilities.
pub mod utils {
    use crate::archive;
    use crate::process::{self, Process};
    use crate::elf::Elf;
    use crate::scheduler;
    use crate::c_ffi::CStr;

    #[derive(Debug)]
    pub enum SpawnError {
        NoProgramSpecified,
        FileNotFound,
        ElfVerifyError,
        ElfNotExecutable,
        ExecLoadError,
        SetArgsError
    }

    use self::SpawnError::*;

    pub fn spawn<'a>(filename: CStr<'static>, argv: &[&[u8]])
                     -> Result<process::Id, SpawnError> {

        if filename.is_empty() {
            return Err(NoProgramSpecified);
        }

        let system = archive::system();

        let data = system.get(filename).ok_or(FileNotFound)?;

        let elf = Elf::new(data).ok_or(ElfVerifyError)?;

        let exec = elf.as_executable().ok_or(ElfNotExecutable)?;

        let process = Process::create(filename);

        let process_id = process.borrow().id();

        {
            let mut process = process.borrow_mut();

            process.load(&exec).map_err(|_| ExecLoadError)?;

            process.set_args(argv).map_err(|_| SetArgsError)?;

            process.run();
        }

        scheduler::push(process);

        Ok(process_id)
    }
}
