/*******************************************************************************
 *
 * kit/kernel/include/terminal.rs
 * - early text mode 80x25 terminal handler
 *
 * vim:ts=4:sw=4:et:tw=80:ft=rust
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::prelude::*;

#[allow(dead_code)]
pub mod color {
    pub static BLACK: u8 = 0;
    pub static BLUE: u8 = 1;
    pub static GREEN: u8 = 2;
    pub static CYAN: u8 = 3;
    pub static RED: u8 = 4;
    pub static MAGENTA: u8 = 5;
    pub static BROWN: u8 = 6;
    pub static LIGHT_GREY: u8 = 7;
    pub static DARK_GREY: u8 = 8;
    pub static LIGHT_BLUE: u8 = 9;
    pub static LIGHT_GREEN: u8 = 10;
    pub static LIGHT_CYAN: u8 = 11;
    pub static LIGHT_RED: u8 = 12;
    pub static LIGHT_MAGENTA: u8 = 13;
    pub static LIGHT_BROWN: u8 = 14;
    pub static WHITE: u8 = 15;
}

mod internal {
    extern {
        pub fn terminal_initialize();
        pub fn terminal_clear();
        pub fn terminal_setcolor(fg: u8, bg: u8);
        pub fn terminal_writestring(string: *const u8);
    }
}

pub fn initialize() {
    unsafe { internal::terminal_initialize() }
}

pub fn clear() {
    unsafe { internal::terminal_clear() }
}

pub fn setcolor(fg: u8, bg: u8) {
    unsafe { internal::terminal_setcolor(fg, bg) }
}

pub fn writestring(string: &str) {
    unsafe { internal::terminal_writestring(string.as_ptr()) }
}
