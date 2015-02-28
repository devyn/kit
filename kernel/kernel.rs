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

mod terminal;

#[no_mangle]
pub extern fn kernel_main() -> ! {

    terminal::initialize();
    terminal::set_color(color::RED, color::WHITE);

    write!(&mut Terminal, "+ Hello. I'm {}.\n", "Kit").ok();

    loop {
        unsafe { asm!("hlt"); }
    }
}

#[lang = "stack_exhausted"]
extern fn stack_exhausted() {
}

#[lang = "eh_personality"]
extern fn eh_personality() {
}

#[lang = "panic_fmt"]
#[allow(unused_variables)]
extern fn panic_fmt(args: &core::fmt::Arguments,
                    file: &str,
                    line: u32) -> ! {
    loop {
        unsafe { asm!("hlt"); }
    }
}
