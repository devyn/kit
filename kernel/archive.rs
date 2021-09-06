/*******************************************************************************
 *
 * kit/kernel/archive.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
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

    debug!("modules={:08X?}",
        core::slice::from_raw_parts(modules, modules_count as usize));

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

    use crate::ptr::UserPtr;

    use crate::c_ffi::{c_int, c_char, int64_t, c_void};

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
    pub extern fn archive_utils_spawn(
        filename: UserPtr<u8>,
        argc: c_int,
        argv: UserPtr<UserPtr<u8>>
    ) -> int64_t {

        if argc < 0 {
            return -1;
        }
        
        if argc > 1024 {
            return -1;
        }

        macro_rules! try_i {
            ($value:expr) => (match $value {
                Ok(v) => v,
                Err(err) => {
                    debug!("archive_utils_spawn error: {}", err);
                    return -1;
                }
            })
        }

        let mut filename_buffer: Vec<u8> = vec![0; 256];

        let filename = try_i!(filename.read_c_string(&mut filename_buffer));

        let argv_ptrs: Vec<UserPtr<u8>> =
            try_i!(argv.read_to_vec(argc as usize));

        let argv = try_i!(argv_ptrs.iter().map(|ptr| {
            let mut arg_buffer: Vec<u8> = vec![0; 256];

            let len = ptr.read_c_string(&mut arg_buffer)?.len();

            arg_buffer.truncate(len + 1);

            Ok(arg_buffer)
        }).collect::<Result<Vec<Vec<u8>>, crate::ptr::Error>>());

        utils::spawn(filename, &argv)
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

    pub fn spawn<'a, A>(filename: CStr<'a>, argv: &[A])
        -> Result<process::Id, SpawnError>
    where
        A: AsRef<[u8]>,
    {

        if filename.is_empty() {
            return Err(NoProgramSpecified);
        }

        let system = archive::system();

        let data = system.get(filename).ok_or(FileNotFound)?;

        let elf = Elf::new(data).ok_or(ElfVerifyError)?;

        let exec = elf.as_executable().ok_or(ElfNotExecutable)?;

        let process = Process::create(filename);

        let process_id = process.lock().id();

        {
            let mut process = process.lock();

            process.load(&exec).map_err(|_| ExecLoadError)?;

            process.set_args(argv).map_err(|_| SetArgsError)?;

            process.run();
        }

        scheduler::push(process);

        Ok(process_id)
    }
}
