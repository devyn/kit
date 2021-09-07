/*******************************************************************************
 *
 * kit/kernel/multiboot.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
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
use core::fmt;

use alloc::vec::Vec;

use crate::c_ffi::CStr;
use crate::constants::{KERNEL_OFFSET, translate_low_addr};
use crate::paging::{PAGE_SIZE, Page, PageType};
use crate::memory::{VirtualAddress, PageCount, PhysicalAddress};

/// How many bytes from the start of the file we search for the header.
pub const SEARCH: usize                 = 8192;

/// The magic field should contain this.
pub const H_MAGIC: u32                  = 0x1BADB002;

/// This should be in %eax.
pub const BOOTLOADER_MAGIC: u32         = 0x2BADB002;

/// The bits in the required part of flags field we don't support.
pub const UNSUPPORTED: u32              = 0x0000fffc;

/// Alignment of multiboot modules.
pub const MOD_ALIGN: u32                = 0x00001000;

/// Alignment of the multiboot info structure.
pub const INFO_ALIGN: u32               = 0x00000004;

/// Flags set in the 'flags' member of the multiboot header.
pub mod header_flags {
    /// Align all boot modules on i386 page (4KB) boundaries.
    pub const PAGE_ALIGN: u32          = 0x00000001;

    /// Must pass memory information to OS.
    pub const MEMORY_INFO: u32         = 0x00000002;

    /// Must pass video information to OS.
    pub const VIDEO_MODE: u32          = 0x00000004;

    /// This flag indicates the use of the address fields in the header.
    pub const AOUT_KLUDGE: u32         = 0x00010000;
}

/// Flags to be set in the 'flags' member of the multiboot info structure.
pub mod info_flags {
    /// is there basic lower/upper memory information?
    pub const MEMORY: u32              = 0x00000001;
    /// is there a boot device set?
    pub const BOOTDEV: u32             = 0x00000002;
    /// is the command-line defined?
    pub const CMDLINE: u32             = 0x00000004;
    /// are there modules to do something with?
    pub const MODS: u32                = 0x00000008;

    /// is there a symbol table loaded?
    pub const AOUT_SYMS: u32           = 0x00000010;
    /// is there an ELF section header table?
    pub const ELF_SHDR: u32            = 0x00000020;

    /// is there a full memory map?
    pub const MEM_MAP: u32             = 0x00000040;

    /// Is there drive info?
    pub const DRIVE_INFO: u32          = 0x00000080;

    /// Is there a config table?
    pub const CONFIG_TABLE: u32        = 0x00000100;

    /// Is there a boot loader name?
    pub const BOOT_LOADER_NAME: u32    = 0x00000200;

    /// Is there a APM table?
    pub const APM_TABLE: u32           = 0x00000400;

    /// Is there video information?
    pub const VIDEO_INFO: u32          = 0x00000800;

    /// Is there framebuffer information?
    pub const FRAMEBUFFER_INFO: u32    = 0x00001000;
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

    // Framebuffer information.
    pub framebuffer_addr:   u64,
    pub framebuffer_pitch:  u32,
    pub framebuffer_width:  u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp:    u8,
    pub framebuffer_type:   u8,
    pub color_info:         ColorInfo,
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

    /// The mmap entries, if present.
    pub unsafe fn mmap_entries(&self) -> Option<MmapEntryIter> {
        if self.flags & info_flags::MEM_MAP != 0 {
            let current_mmap =
                translate_low_addr(self.mmap_addr).unwrap() as *const u8;
            let mmap_end =
                current_mmap.wrapping_add(self.mmap_length as usize);

            Some(MmapEntryIter {
                current_mmap, mmap_end, phantom: core::marker::PhantomData })
        } else {
            None
        }
    }

    /// The modules, if present.
    pub unsafe fn modules(&self) -> Option<&[Module]> {
        if self.flags & info_flags::MODS != 0 {
            let slice =
                core::slice::from_raw_parts(
                    translate_low_addr(self.mods_addr).unwrap(),
                    self.mods_count as usize);

            Some(slice)
        } else {
            None
        }
    }

    /// Parse available memory maps
    pub unsafe fn parse_available(&self, out: &mut Vec<(usize, usize)>) {
        if let Some(entries) = self.mmap_entries() {
            for entry in entries {
                debug!("Multiboot memory entry: {:08X?}", entry);

                if entry.is_available() {
                    out.push(entry.range());
                }
            }
        }
    }

    /// Parse reserved memory maps
    pub unsafe fn parse_reserved(&self, out: &mut Vec<(usize, usize)>) {
        if let Some(entries) = self.mmap_entries() {
            for entry in entries {
                if !entry.is_available() {
                    out.push(entry.range());
                }
            }
        }
    }

    /// Generate required identity maps to preserve kernel, modules, and
    /// multiboot info.
    pub unsafe fn generate_identity_maps(
        &self,
        out: &mut Vec<(VirtualAddress, PageCount, Page<PhysicalAddress>)>,
    ) {
        let initial_len = out.len();

        // First, identity map low 1 MB of memory.
        out.push((KERNEL_OFFSET, 0x100000/PAGE_SIZE,
            Some((0x0, PageType::default().writable()))));

        // Load kernel section symbols
        extern {
            static _bootstrap_begin: u8;
            static _bootstrap_end: u8;
            static _kernel_text_begin: u8;
            static _kernel_text_end: u8;
            static _kernel_rodata_begin: u8;
            static _kernel_rodata_end: u8;
            static _kernel_data_begin: u8;
            static _kernel_data_end: u8;
            static _kernel_got_begin: u8;
            static _kernel_got_end: u8;
            static _kernel_got_plt_begin: u8;
            static _kernel_got_plt_end: u8;
            static _kernel_bss_begin: u8;
            static _kernel_bss_end: u8;
        }

        // These symbols are low memory addressed, so we have to add the offset
        let bootstrap_begin = &_bootstrap_begin as *const u8 as usize;
        let bootstrap_end = &_bootstrap_end as *const u8 as usize;

        {
            let bytes = bootstrap_end - bootstrap_begin;
            let pages = bytes / PAGE_SIZE +
                if bytes % PAGE_SIZE != 0 { 1 } else { 0 };

            out.push((
                KERNEL_OFFSET + bootstrap_begin,
                pages,
                Some((bootstrap_begin, PageType::default().writable()))
            ));
        }

        // The rest are high memory addressed, just map them accordingly
        let sections: [(*const u8, *const u8, PageType); 6] = [
            (&_kernel_text_begin, &_kernel_text_end,
             PageType::default().executable()),
            (&_kernel_rodata_begin, &_kernel_rodata_end,
             PageType::default()),
            (&_kernel_data_begin, &_kernel_data_end,
             PageType::default().writable()),
            (&_kernel_got_begin, &_kernel_got_end,
             PageType::default().writable()),
            (&_kernel_got_plt_begin, &_kernel_got_plt_end,
             PageType::default().writable()),
            (&_kernel_bss_begin, &_kernel_bss_end,
             PageType::default().writable()),
        ];

        for &(begin, end, page_type) in &sections[..] {
            let bytes = end as usize - begin as usize;
            let pages = bytes / PAGE_SIZE +
                if bytes % PAGE_SIZE != 0 { 1 } else { 0 };

            out.push((
                begin as usize,
                pages,
                Some((begin as usize - KERNEL_OFFSET, page_type))
            ));
        }

        // Add modules, identity mapped, read-only
        if let Some(modules) = self.modules() {
            for module in modules {
                let bytes =
                    module.mod_end as usize - module.mod_start as usize;
                let pages = bytes / PAGE_SIZE +
                    if bytes % PAGE_SIZE > 0 { 1 } else { 0 };

                out.push((
                    KERNEL_OFFSET + module.mod_start as usize,
                    pages,
                    Some((module.mod_start as usize, PageType::default()))
                ));
            }
        }

        debug!("Multiboot identity map: {:08X?}", &out[initial_len..]);
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub union ColorInfo {
    indexed: IndexedColor,
    rgb: RgbColor,
}

impl fmt::Debug for ColorInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            f.debug_struct("ColorInfo")
                .field("indexed", &self.indexed)
                .field("rgb", &self.rgb)
                .finish()
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IndexedColor {
    framebuffer_palette_addr: u32,
    framebuffer_palette_num_colors: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RgbColor {
    framebuffer_red_field_position: u8,
    framebuffer_red_mask_size: u8,
    framebuffer_green_field_position: u8,
    framebuffer_green_mask_size: u8,
    framebuffer_blue_field_position: u8,
    framebuffer_blue_mask_size: u8,
}

pub const MEMORY_AVAILABLE: u32 = 1;
pub const MEMORY_RESERVED: u32 = 2;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
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

    pub fn range(&self) -> (usize, usize) {
        (self.addr as usize, (self.addr + self.len) as usize)
    }
}

pub struct MmapEntryIter<'a> {
    current_mmap: *const u8,
    mmap_end: *const u8,
    phantom: core::marker::PhantomData<&'a u8>,
}

impl<'a> Iterator for MmapEntryIter<'a> {
    type Item = &'a MmapEntry;

    fn next(&mut self) -> Option<&'a MmapEntry> {
        unsafe {
            if self.current_mmap < self.mmap_end {
                let entry_ptr: *const MmapEntry = self.current_mmap.cast();
                let entry = entry_ptr.as_ref().unwrap();

                self.current_mmap = self.current_mmap
                    .wrapping_add(entry.size as usize + 4);

                Some(entry)
            } else {
                None
            }
        }
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
