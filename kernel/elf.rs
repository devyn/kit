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

use process::{self, Process, Image};

static MAGIC: &'static [u8] = b"\x7fELF";

#[derive(Clone, Copy)]
pub struct Elf<'a> {
    buffer: &'a [u8],
}

impl<'a> Elf<'a> {
    pub fn new(buffer: &'a [u8]) -> Option<Elf<'a>> {
        // Require at least 16 bytes.
        if buffer.len() < 16 {
            return None;
        }

        // Match magic string and version number (1).
        if &buffer[0..4] != MAGIC || buffer[6] != 1 {
            return None;
        }

        Some(Elf {
            buffer: buffer
        })
    }

    pub fn os_abi(&self) -> u8 {
        self.buffer[7]
    }

    pub fn abi_version(&self) -> u8 {
        self.buffer[8]
    }

    pub fn as_elf64_le(&'a self) -> Option<Elf64Le<'a>> {
        Elf64Le::new(self.buffer)
    }

    pub fn as_executable(&'a self) -> Option<Executable<'a>> {
        Executable::new(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfType {
    None,
    Relocatable,
    Executable,
    Dynamic,
    CoreDump,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Machine {
    None,
    Intel386,
    Amd64,
    Unknown,
}

#[derive(Clone, Copy)]
pub struct Elf64Le<'a> {
    buffer: &'a [u8],
}

impl<'a> Elf64Le<'a> {
    fn new(buffer: &'a [u8]) -> Option<Elf64Le<'a>> {
        // Require at least 64 bytes.
        if buffer.len() < 64 {
            return None;
        }

        // Require 64-bit class.
        if buffer[4] != 2 {
            return None;
        }

        // Require little endian.
        if buffer[5] != 1 {
            return None;
        }

        Some(Elf64Le {
            buffer: buffer
        })
    }

    pub fn elf_type(&self) -> ElfType {
        match self.read_u16(16) {
            0 => ElfType::None,
            1 => ElfType::Relocatable,
            2 => ElfType::Executable,
            3 => ElfType::Dynamic,
            4 => ElfType::CoreDump,
            _ => ElfType::Unknown,
        }
    }

    pub fn machine(&self) -> Machine {
        match self.read_u16(18) {
            0  => Machine::None,
            3  => Machine::Intel386,
            62 => Machine::Amd64,
            _  => Machine::Unknown,
        }
    }

    fn read_u16(&self, offset: usize) -> u16 {
        u16::from_le((self.buffer[offset] as u16) +
                     ((self.buffer[offset + 1] as u16) << 8))
    }
}

#[derive(Clone, Copy)]
pub struct Executable<'a> {
    elf: &'a Elf<'a>,
    elf64_le: Elf64Le<'a>,
}

impl<'a> Executable<'a> {
    pub fn new(elf: &'a Elf<'a>) -> Option<Executable<'a>> {
        if elf.os_abi() != 0 {
            return None;
        }

        if elf.abi_version() != 0 {
            return None;
        }

        if let Some(elf64_le) = Elf64Le::new(elf.buffer) {
            if elf64_le.elf_type() != ElfType::Executable {
                return None;
            }

            if elf64_le.machine() != Machine::Amd64 /* FIXME */ {
                return None;
            }

            Some(Executable {
                elf: elf,
                elf64_le: elf64_le,
            })
        } else {
            None
        }
    }
}

impl<'a> Image for Executable<'a> {
    fn load_into(&self, process: &mut Process)
                 -> Result<(), process::Error> {
        unimplemented!()
    }
}
