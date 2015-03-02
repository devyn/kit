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

#![crate_type="lib"]
#![feature(core)]
#![feature(libc)]
#![feature(asm)]
#![feature(no_std)]
#![feature(lang_items)]
#![no_std]

#[macro_use]
extern crate core;

// No linkage. Mostly for types.
extern crate libc;

use core::prelude::*;
use core::fmt::Write;

use terminal::*;

use shell::shell;

pub mod terminal;
pub mod constants;
pub mod multiboot;
pub mod memory;
pub mod interrupt;
pub mod paging;
pub mod keyboard;
pub mod archive;
pub mod process;
pub mod shell;

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

            console().write_raw_bytes(cmdline).unwrap();
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
            unimplemented!();
        } else {
            write!(console(), "W: No initial program specified on kernel \
                               command line; dropping into kernel\n   \
                               shell.\n").unwrap();

            unsafe {
                interrupt::enable();
                shell();
            }
        }
    }

    unreachable!();
}

#[lang = "stack_exhausted"]
extern fn stack_exhausted() {
}

#[lang = "eh_personality"]
extern fn eh_personality() {
}

#[lang = "panic_fmt"]
#[allow(unused_must_use)]
extern fn panic_fmt(fmt: core::fmt::Arguments,
                    file: &'static str,
                    line: usize) -> ! {

    console().set_color(Color::White, Color::Red);

    write!(console(), "\nKernel panic in {}:{}:\n  {}\n\n", file, line, fmt);

    unsafe {
        asm!("cli" :::: "volatile");

        loop {
            asm!("hlt" :::: "volatile");
        }
    }
}
