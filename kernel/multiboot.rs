/*******************************************************************************
 *
 * kit/kernel/multiboot.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Multiboot header file.
//!
//! Original `multiboot.h` copyright information:
//!
//! > Copyright (C) 1999,2003,2007,2008,2009  Free Software Foundation, Inc.
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a
//! > copy of this software and associated documentation files (the "Software"),
//! > to deal in the Software without restriction, including without limitation
//! > the rights to use, copy, modify, merge, publish, distribute, sublicense,
//! > and/or sell copies of the Software, and to permit persons to whom the
//! > Software is furnished to do so, subject to the following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included in
//! > all copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! > IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! > FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
//! > ANY DEVELOPER OR DISTRIBUTOR BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! > LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
//! > FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//! > DEALINGS IN THE SOFTWARE.

use core::mem;

use c_ffi::CStr;
use constants::translate_low_addr;

/// How many bytes from the start of the file we search for the header.
pub static SEARCH: usize                 = 8192;

/// The magic field should contain this.
pub static H_MAGIC: u32                  = 0x1BADB002;

/// This should be in %eax.
pub static BOOTLOADER_MAGIC: u32         = 0x2BADB002;

/// The bits in the required part of flags field we don't support.
pub static UNSUPPORTED: u32              = 0x0000fffc;

/// Alignment of multiboot modules.
pub static MOD_ALIGN: u32                = 0x00001000;

/// Alignment of the multiboot info structure.
pub static INFO_ALIGN: u32               = 0x00000004;

/// Flags set in the 'flags' member of the multiboot header.
pub mod header_flags {
    /// Align all boot modules on i386 page (4KB) boundaries.
    pub static PAGE_ALIGN: u32          = 0x00000001;

    /// Must pass memory information to OS.
    pub static MEMORY_INFO: u32         = 0x00000002;

    /// Must pass video information to OS.
    pub static VIDEO_MODE: u32          = 0x00000004;

    /// This flag indicates the use of the address fields in the header.
    pub static AOUT_KLUDGE: u32         = 0x00010000;
}

/// Flags to be set in the 'flags' member of the multiboot info structure.
pub mod info_flags {
    /// is there basic lower/upper memory information?
    pub static MEMORY: u32              = 0x00000001;
    /// is there a boot device set?
    pub static BOOTDEV: u32             = 0x00000002;
    /// is the command-line defined?
    pub static CMDLINE: u32             = 0x00000004;
    /// are there modules to do something with?
    pub static MODS: u32                = 0x00000008;

    /// is there a symbol table loaded?
    pub static AOUT_SYMS: u32           = 0x00000010;
    /// is there an ELF section header table?
    pub static ELF_SHDR: u32            = 0x00000020;

    /// is there a full memory map?
    pub static MEM_MAP: u32             = 0x00000040;

    /// Is there drive info?
    pub static DRIVE_INFO: u32          = 0x00000080;

    /// Is there a config table?
    pub static CONFIG_TABLE: u32        = 0x00000100;

    /// Is there a boot loader name?
    pub static BOOT_LOADER_NAME: u32    = 0x00000200;

    /// Is there a APM table?
    pub static APM_TABLE: u32           = 0x00000400;

    /// Is there video information?
    pub static VIDEO_INFO: u32          = 0x00000800;
}

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    /// Must be H_MAGIC - see above.
    pub magic: u32,

    /// Feature flags. (see header_flags)
    pub flags: u32,

    /// The above fields plus this one must equal 0 mod 2^32.
    pub checksum: u32,

    // These are only valid if AOUT_KLUDGE is set.
    pub header_addr:   u32,
    pub load_addr:     u32,
    pub load_end_addr: u32,
    pub bss_end_addr:  u32,
    pub entry_addr:    u32,

    // These are only valid if VIDEO_MODE is set.
    pub mode_type: u32,
    pub width:     u32,
    pub height:    u32,
    pub depth:     u32,
}

/// The symbol table for a.out.
#[repr(C)]
#[derive(Debug)]
pub struct AoutSymbolTable {
    pub tabsize:  u32,
    pub strsize:  u32,
    pub addr:     u32,
    pub reserved: u32,
}

/// The section header table for ELF.
#[repr(C)]
#[derive(Debug)]
pub struct ElfSectionHeaderTable {
    pub num:   u32,
    pub size:  u32,
    pub addr:  u32,
    pub shndx: u32,
}

/// An unsafe union of AoutSymbolTable and ElfSectionHeaderTable.
#[repr(C)]
#[derive(Debug)]
pub struct AoutElfTableUnion {
    data: [u32; 4],
}

impl AoutElfTableUnion {
    /// Interpret the union as AoutSymbolTable.
    pub unsafe fn as_aout_symbol_table<'a>(&'a self) -> &'a AoutSymbolTable {
        mem::transmute(self)
    }

    /// Interpret the union as ElfSectionHeaderTable.
    pub unsafe fn as_elf_section_header_table<'a>(&'a self)
                                              -> &'a ElfSectionHeaderTable {
        mem::transmute(self)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Info {
    /// Feature flags. (see info_flags)
    pub flags: u32,

    /// Available lower memory from BIOS.
    pub mem_lower: u32,

    /// Available upper memory from BIOS.
    ///
    /// Only accurate for systems with fewer than 4 TiB of upper memory.
    pub mem_upper: u32,

    /// "root" partition.
    pub boot_device: u32,

    /// Kernel command line pointer (C string).
    pub cmdline: u32,

    /// Number of modules provided.
    pub mods_count: u32,

    /// Address of the first module.
    pub mods_addr: u32,

    pub u: AoutElfTableUnion,

    /// Memory mapping buffer length.
    pub mmap_length: u32,

    /// Memory mapping buffer address.
    pub mmap_addr: u32,

    /// Drive info buffer length.
    pub drives_length: u32,

    /// Drive info buffer address.
    pub drives_addr: u32,

    /// ROM configuration table.
    pub config_table: u32,

    /// Boot loader name pointer (C string).
    pub boot_loader_name: u32,

    /// APM table address.
    pub apm_table: u32,

    // VESA BIOS Extensions information.
    pub vbe_control_info:  u32,
    pub vbe_mode_info:     u32,
    pub vbe_mode:          u16,
    pub vbe_interface_seg: u16,
    pub vbe_interface_off: u16,
    pub vbe_interface_len: u16,
}

impl Info {
    /// Available lower and upper memory sizes, if present.
    ///
    /// Only accurate for systems with fewer than 4 GiB of upper memory.
    pub fn mem_sizes(&self) -> Option<(usize, usize)> {
        if self.flags & info_flags::MEMORY != 0 {
            Some((self.mem_lower as usize, self.mem_upper as usize))
        } else {
            None
        }
    }

    /// The kernel command line, if present.
    ///
    /// There is no guarantee that the command line is valid UTF-8.
    pub unsafe fn cmdline<'a>(&'a self) -> Option<CStr<'a>> {
        if self.flags & info_flags::CMDLINE != 0 {
            let cmdline =
                translate_low_addr(self.cmdline)
                    .expect("cmdline pointer outside low region");

            Some(CStr::from_ptr(cmdline))
        } else {
            None
        }
    }
}

pub static MEMORY_AVAILABLE: u32 = 1;
pub static MEMORY_RESERVED: u32 = 2;

#[repr(C)]
#[derive(Debug)]
pub struct MmapEntry {
    pub size: u32,
    pub addr: u64,
    pub len:  u64,

    /// Assume reserved if not equal to MEMORY_AVAILABLE.
    ///
    /// Probably comes directly from the BIOS.
    pub kind: u32,
}

impl MmapEntry {
    pub fn is_available(&self) -> bool {
        self.kind == MEMORY_AVAILABLE
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Module {
    // The memory used goes from bytes 'mod_start' to 'mod_end-1' inclusive.
    pub mod_start: u32,
    pub mod_end:   u32,

    /// Module command line pointer (C string).
    pub cmdline: u32,

    /// Padding to take the structure to 16 bytes (must be zero).
    pub pad: u32,
}

extern {
    static kernel_multiboot_info: Info;
}

/// Get the multiboot info from the bootloader.
pub unsafe fn get_info() -> &'static Info {
    let info_low: *const Info = &kernel_multiboot_info;

    let info: *const Info =
        translate_low_addr(info_low as u32)
            .expect("kernel_multiboot_info outside low region");

    info.as_ref().expect("kernel_multiboot_info is null")
}
