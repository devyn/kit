/*******************************************************************************
 *
 * kit/kernel/process.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Process management functions.

use core::prelude::*;

use libc::{c_int, c_char};
use c_ffi::CStr;

pub unsafe fn initialize() {
    ffi::process_initialize()
}

pub struct Process {
    pub internal: *mut ffi::Process,
}

impl Process {
    pub fn new(name: CStr<'static>) -> Option<Process> {
        let ptr = unsafe { ffi::process_create(name.as_ptr()) };

        if !ptr.is_null() {
            Some(Process { internal: ptr })
        } else {
            None
        }
    }

    pub fn load<T: Image>(&mut self, image: &T) -> bool {
        image.load_into(self)
    }

    pub fn set_args<'a>(&mut self, args: &[*const c_char]) -> bool {
        unsafe {
            ffi::process_set_args(self.internal,
                                  args.len() as c_int,
                                  args.as_ptr()) == 1
        }
    }

    pub fn run(&mut self) {
        unsafe { ffi::process_run(self.internal) }
    }
}

pub trait Image {
    fn load_into(&self, &mut Process) -> bool;
}

/// C interface. See `kit/kernel/include/archive.h`.
pub mod ffi {
    use libc::{c_int, c_char};

    #[repr(C)]
    pub struct Process;

    extern {
        pub fn process_initialize();

        pub fn process_create(name: *const c_char) -> *mut Process;

        pub fn process_set_args(process: *mut Process,
                                argc:    c_int,
                                argv:    *const *const c_char) -> i8;

        pub fn process_run(process: *mut Process);
    }
}
