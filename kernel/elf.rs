/*******************************************************************************
 *
 * kit/kernel/elf.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Executable and Linkable Format loader.

use core::mem;

use process::{Process, Image};

#[derive(Clone, Copy)]
pub struct Elf<'a> {
    header: &'a ffi::ElfHeader64,
}

impl<'a> Elf<'a> {
    pub fn new(bytes: &'a [u8]) -> Option<Elf<'a>> {
        unsafe {
            let header = mem::transmute(bytes.as_ptr());

            if ffi::elf_verify(header) == 1 {
                Some(Elf { header: header.as_ref().unwrap() })
            } else {
                None
            }
        }
    }
}

impl<'a> Image for Elf<'a> {
    fn load_into(&self, process: &mut Process) -> bool {
        unsafe { ffi::elf_load(self.header, process.internal) == 1 }
    }
}

/// C interface. See `kit/kernel/include/elf.h`.
pub mod ffi {
    use process::ffi::Process;

    #[repr(C)]
    pub struct ElfHeader64;

    extern {
        pub fn elf_verify(header: *const ElfHeader64) -> i8;

        pub fn elf_load(header:  *const ElfHeader64,
                        process: *mut Process) -> i8;
    }
}
