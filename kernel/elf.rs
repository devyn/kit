/*******************************************************************************
 *
 * kit/kernel/elf.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Executable and Linkable Format loader.

use core::slice;

use crate::process::{self, Process, Image, ProcessMem};
use crate::paging::{self, PageType};
use crate::util::{copy_memory, zero_memory};

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
    Unknown(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Machine {
    None,
    Intel386,
    Amd64,
    Unknown(u16),
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
            n => ElfType::Unknown(n),
        }
    }

    pub fn machine(&self) -> Machine {
        match self.read_u16(18) {
            0  => Machine::None,
            3  => Machine::Intel386,
            62 => Machine::Amd64,
            n  => Machine::Unknown(n),
        }
    }

    pub fn entry(&self) -> usize {
        self.read_u64(24) as usize
    }

    pub fn program_headers(&'a self) -> ElfProgramHeaders<'a> {
        let e_phoff     = self.read_u64(32) as usize;
        let e_phentsize = self.read_u16(54) as usize;
        let e_phnum     = self.read_u16(56) as usize;

        ElfProgramHeaders {
            elf: self,
            offset: e_phoff,
            header_size: e_phentsize,
            index: 0,
            count: e_phnum,
        }
    }

    fn read_u16(&self, offset: usize) -> u16 {
        (0..2).map(|index| {
            (self.buffer[index + offset] as u16) << (index * 8)
        }).sum()
    }

    fn read_u32(&self, offset: usize) -> u32 {
        (0..4).map(|index| {
            (self.buffer[index + offset] as u32) << (index * 8)
        }).sum()
    }

    fn read_u64(&self, offset: usize) -> u64 {
        (0..8).map(|index| {
            (self.buffer[index + offset] as u64) << (index * 8)
        }).sum()
    }
}

pub struct ElfProgramHeaders<'a> {
    elf: &'a Elf64Le<'a>,
    offset: usize,
    header_size: usize,
    index: usize,
    count: usize,
}

impl<'a> Iterator for ElfProgramHeaders<'a> {
    type Item = ElfProgramHeader<'a>;

    fn next(&mut self) -> Option<ElfProgramHeader<'a>> {
        if self.index < self.count {
            let o = self.offset + self.index * self.header_size;

            self.index += 1;

            let flags = self.elf.read_u32(o+4);

            let data_start = self.elf.read_u64(o+8) as usize;
            let data_end   = data_start + self.elf.read_u64(o+32) as usize;

            Some(ElfProgramHeader {
                region_type: match self.elf.read_u32(o) {
                    0 => RegionType::Null,
                    1 => RegionType::Load,
                    2 => RegionType::Dynamic,
                    3 => RegionType::Interpreter,
                    4 => RegionType::Note,
                    6 => RegionType::ProgramHeader,
                    n => RegionType::Unknown(n),
                },
                readable:    flags & 4 == 4,
                writable:    flags & 2 == 2,
                executable:  flags & 1 == 1,
                data:        &self.elf.buffer[data_start..data_end],
                mem_offset:  self.elf.read_u64(o + 16) as usize,
                mem_size:    self.elf.read_u64(o + 40) as usize,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Null,
    Load,
    Dynamic,
    Interpreter,
    Note,
    ProgramHeader,
    Unknown(u32),
}

#[derive(Clone)]
pub struct ElfProgramHeader<'a> {
    pub region_type: RegionType,
    pub readable:    bool,
    pub writable:    bool,
    pub executable:  bool,
    pub data:        &'a [u8],
    pub mem_offset:  usize,
    pub mem_size:    usize,
}

impl<'a> ElfProgramHeader<'a> {
    /// Copy the data referenced by the program header into the destination
    /// buffer. The buffer must be large enough to fit the entire `mem_size`
    /// specified by the header, or `Err` will be returned with the difference.
    ///
    /// # Panics
    ///
    /// Panics if the `mem_size` is less than the length of the `data`, unless
    /// `mem_size` is zero (in which case nothing is loaded and `Ok` is
    /// returned).
    pub fn load_into(&self, destination: &mut [u8]) -> Result<(), usize> {
        if self.mem_size == 0 {
            return Ok(());
        }

        assert!(self.data.len() <= self.mem_size);

        if destination.len() < self.mem_size {
            Err(self.mem_size - destination.len())
        } else {
            copy_memory(self.data, destination);

            zero_memory(&mut destination[self.data.len()..self.mem_size]);

            Ok(())
        }
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
        let mut result = Ok(());

        unsafe {
            // Load the process's pageset, making sure to restore the previous
            // one after.
            let mem = process.mem().unwrap();
            let new_pageset = mem.lock().pageset();

            let original_pageset = paging::current_pageset();
            paging::set_current_pageset(Some(new_pageset));

            // Must not return directly from this loop. Set result and break if
            // necessary.
            for phdr in self.elf64_le.program_headers() {
                if phdr.region_type == RegionType::Load {
                    result = phdr_load(phdr, &mut *mem.lock());
                }

                if result.is_err() { break }
            }

            // Restore the previous pageset.
            paging::set_current_pageset(original_pageset);
        }

        process.set_entry_point(self.elf64_le.entry());

        result
    }
}

unsafe fn phdr_load<'a>(phdr: ElfProgramHeader<'a>,
                        mem: &mut ProcessMem)
                        -> Result<(), process::Error> {

    let mut page_type = PageType::default().user();

    if phdr.writable   { page_type = page_type.writable(); }
    if phdr.executable { page_type = page_type.executable(); }

    // What we need first while writing the pages
    let page_type_init = PageType::default().writable();

    mem.map_allocate(phdr.mem_offset, phdr.mem_size, page_type_init)?;

    // Access the memory directly via a slice into userspace.
    let memory = slice::from_raw_parts_mut(
        phdr.mem_offset as *mut u8, phdr.mem_size);

    assert!(phdr.load_into(memory).is_ok());

    // Change the pages to our real page_type
    mem.set_permissions(phdr.mem_offset, phdr.mem_size, page_type)?;

    Ok(())
}
