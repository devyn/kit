/*******************************************************************************
 *
 * kit/kernel/kernel.rs
 * - main kernel entry point and top level management
 *
 * vim:ts=4:sw=4:et:tw=80:ft=rust
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Based on OSDev Bare Bones tutorial
 * http://wiki.osdev.org/Bare_Bones
 *
 ******************************************************************************/

#![crate_type="lib"]
#![feature(core)]
#![feature(asm)]
#![feature(no_std)]
#![feature(lang_items)]
#![no_std]

#[macro_use]
extern crate core;

use core::prelude::*;
use core::fmt::Write;

use terminal::color;
use terminal::Terminal;

pub mod terminal;

#[no_mangle]
pub extern fn kernel_main() -> ! {

    terminal::initialize();
    terminal::set_color(color::RED, color::WHITE);

    terminal::write("+ Hello, I'm Kit.\n");

    let result: Result<(), &str> = Err("foo");
    
    result.unwrap();

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

    terminal::set_color(color::WHITE, color::RED);

    write!(&mut Terminal, "\nKernel panic in {}:{}:\n  {}\n\n", file, line, fmt);

    unsafe {
        asm!("cli");

        loop {
            asm!("hlt");
        }
    }
}
