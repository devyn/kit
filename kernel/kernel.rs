/*******************************************************************************
 *
 * kit/kernel/kernel.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Based on OSDev Bare Bones tutorial
 * http://wiki.osdev.org/Bare_Bones
 *
 ******************************************************************************/

//! The Kit kernel.

#![crate_name="kernel"]
#![crate_type="lib"]

#![feature(lang_items, asm, box_syntax, step_by)]
#![feature(unicode, box_patterns, alloc, collections)]
#![feature(iter_arith, repr_simd, drop_types_in_const)]

#![allow(improper_ctypes)]

#![no_std]

// These rust libs are specifically configured for Kit.
extern crate alloc;
extern crate rustc_unicode;
#[macro_use] extern crate collections;

pub mod terminal;
pub mod constants;
pub mod multiboot;
pub mod memory;
pub mod interrupt;
pub mod paging;
pub mod keyboard;
pub mod archive;
pub mod process;
pub mod elf;
pub mod scheduler;
pub mod syscall;
pub mod c_ffi;
pub mod error;
pub mod util;

use terminal::*;

/// Main kernel entry point.
#[no_mangle]
pub extern fn kernel_main() -> ! {

    console().reset().unwrap();
    console().set_color(Color::Red, Color::White).unwrap();

    console().write_str("+ Hello, I'm Kit.\n").unwrap();

    console().set_color(Color::White, Color::Red).unwrap();
    console().write_char('\n').unwrap();

    let mb_info = unsafe { multiboot::get_info() };

    match mb_info.mem_sizes() {
        Some((lower, upper)) => {
            write!(console(),
                   "{:<20} {:<10} KiB\n\
                    {:<20} {:<10} KiB\n",
                   "Lower memory:", lower,
                   "Upper memory:", upper).unwrap();
        },
        None => {
            write!(console(),
                   "W: Bootloader did not provide valid memory information!")
                .unwrap();
        }
    }

    match unsafe { mb_info.cmdline() } {
        Some(cmdline) => {
            write!(console(), "{:<20} ", "Kernel command line:").unwrap();

            console().write_raw_bytes(cmdline.as_bytes()).unwrap();
            console().write_char('\n').unwrap();
        },
        None => {
            write!(console(), "Kernel command line:\n").unwrap();
        }
    }

    console().write_char('\n').unwrap();
    console().set_color(Color::LightGrey, Color::Black).unwrap();
    console().write_char('\n').unwrap();

    if mb_info.flags & multiboot::info_flags::MEM_MAP != 0 {
        unsafe {
            let mmap = constants::translate_low_addr(mb_info.mmap_addr)
                .expect("mmap pointer outside low region");

            memory::initialize(mmap, mb_info.mmap_length);
        }
    } else {
        panic!("Bootloader did not provide memory map!");
    }

    unsafe {
        interrupt::initialize();
        paging::initialize();
        memory::enable_large_heap();
        keyboard::initialize().unwrap();
    }

    if mb_info.flags & multiboot::info_flags::MODS != 0 {
        unsafe {
            let mods = constants::translate_low_addr(mb_info.mods_addr)
                .expect("mods pointer outside low region");

            if !archive::initialize(mods, mb_info.mods_count) {
                panic!("Archive initialization failed. Are you sure \
                        system.kit was provided?");
            }
        }
    } else {
        panic!("Bootloader did not provide modules!");
    }

    unsafe {
        process::initialize();
    }

    {
        let cmdline = unsafe { mb_info.cmdline().unwrap() };

        if !cmdline.is_empty() {
            archive::utils::spawn(cmdline, &[cmdline.as_bytes()]).unwrap();
            unsafe { scheduler::enter(); }
        } else {
            panic!("No initial program specified on kernel command line!");
        }
    }

    unreachable!();
}

#[lang = "eh_personality"]
extern fn eh_personality() {
}

#[lang = "panic_fmt"]
extern fn panic_fmt(fmt: core::fmt::Arguments,
                    file: &'static str,
                    line: usize) -> ! {

    let _ = console().set_color(Color::White, Color::Red);

    let _ = write!(console(), "\nKernel panic in {}:{}:\n  {}\n\n",
                   file, line, fmt);

    unsafe {
        loop { asm!("cli; hlt" :::: "volatile"); }
    }
}
