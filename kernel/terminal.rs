/*******************************************************************************
 *
 * kit/kernel/include/terminal.rs
 * - early text mode 80x25 terminal handler
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::prelude::*;

use core::fmt;

#[derive(Copy)]
pub enum Color {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Brown,
    LightGrey,
    DarkGrey,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    LightMagenta,
    LightBrown,
    White,
}

/// A terminal.
pub trait Terminal: fmt::Write {
    fn reset(&mut self) -> fmt::Result;
    fn clear(&mut self) -> fmt::Result;

    fn get_cursor(&self) -> (usize, usize);
    fn set_cursor(&mut self, row: usize, col: usize) -> fmt::Result;

    fn get_color(&self) -> (Color, Color);
    fn set_color(&mut self, fg: Color, bg: Color) -> fmt::Result;

    /// Does not flush.
    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result;

    /// Does not flush.
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        for byte in bytes {
            try!(self.write_raw_byte(*byte));
        }

        Ok(())
    }

    fn flush(&mut self) -> fmt::Result;

    fn write_char(&mut self, ch: char) -> fmt::Result {
        let mut buf = [0u8, 4];

        try!(ch.encode_utf8(&mut buf).ok_or(fmt::Error));

        try!(self.write_raw_bytes(&buf));
        try!(self.flush());
        Ok(())
    }
}

/// Controls a VGA text-mode terminal.
pub struct Vga {
    width:  usize,
    height: usize,
    row:    usize,
    col:    usize,
    fg:     Color,
    bg:     Color,
    attr:   u8,
    buffer: *mut u16,
    port:   u16
}

impl Vga {
    pub unsafe fn new(width:  usize,
                      height: usize,
                      buffer: *mut u16,
                      port:   u16)
                      -> Vga {

        let mut vga = Vga {
            width:  width,
            height: height,
            row:    0,
            col:    0,
            fg:     Color::LightGrey,
            bg:     Color::Black,
            attr:   Vga::attr(Color::LightGrey, Color::Black),
            buffer: buffer,
            port:   port
        };

        vga.reset().unwrap();

        vga
    }

    pub fn color(c: Color) -> u8 {
        match c {
            Color::Black        => 0,
            Color::Blue         => 1,
            Color::Green        => 2,
            Color::Cyan         => 3,
            Color::Red          => 4,
            Color::Magenta      => 5,
            Color::Brown        => 6,
            Color::LightGrey    => 7,
            Color::DarkGrey     => 8,
            Color::LightBlue    => 9,
            Color::LightGreen   => 10,
            Color::LightCyan    => 11,
            Color::LightRed     => 12,
            Color::LightMagenta => 13,
            Color::LightBrown   => 14,
            Color::White        => 15,
        }
    }

    pub fn attr(fg: Color, bg: Color) -> u8 {
        Vga::color(fg) | (Vga::color(bg) << 4)
    }

    fn update_attr(&mut self) {
        self.attr = Vga::attr(self.fg, self.bg);
    }

    fn update_cursor(&mut self) {
        unsafe fn outb(byte: u8, port: u16) {
            // FIXME: It seems like we have to do this due to a Rust bug where
            // the "a" and "d" constraints cause nothing to be generated. I
            // should file a bug report.
            asm!(concat!("mov $0, %al;\n",
                         "mov $1, %dx;\n",
                         "out %al, %dx")
                :
                : "r" (byte), "r" (port)
                : "rax", "rdx"
                : "volatile");
        }

        let pos: u16 = ((self.row * self.width) + self.col) as u16;

        unsafe {
            outb(0x0F,             self.port);
            outb(pos as u8,        self.port + 1);

            outb(0x0E,             self.port);
            outb((pos >> 8) as u8, self.port + 1);
        }
    }

    pub fn put(&mut self, byte: u8, attr: u8, row: usize, col: usize) {
        unsafe {
            *self.buffer.offset((row * self.width + col) as isize) =
                (byte as u16) | ((attr as u16) << 8);
        }
    }

    pub fn put_here(&mut self, byte: u8) {
        let (attr, row, col) = (self.attr, self.row, self.col);

        self.put(byte, attr, row, col)
    }

    fn new_line(&mut self) {
        // Clear to the end of the line.
        while self.col < self.width {
            self.put_here(' ' as u8);
            self.col += 1;
        }

        // Go to the next line, scrolling if necessary.
        self.col  = 0;
        self.row += 1;

        while self.row >= self.height {
            self.scroll();
            self.row -= 1;
        }

        self.update_cursor();
    }

    fn scroll(&mut self) {
        // Shift everything one line back.
        for row in 1..self.height {
            for col in 0..self.width {
                let index = (row * self.width + col) as isize;

                unsafe {
                    *self.buffer.offset(index - self.width as isize) =
                        *self.buffer.offset(index);
                }
            }
        }

        // Clear last line.
        let (attr, height) = (self.attr, self.height);

        for col in 0..self.width {
            self.put(' ' as u8, attr, height - 1, col);
        }
    }
}

impl Terminal for Vga {
    fn reset(&mut self) -> fmt::Result {
        self.fg = Color::LightGrey;
        self.bg = Color::Black;
        self.update_attr();
        self.clear()
    }

    fn clear(&mut self) -> fmt::Result {
        self.row = 0;
        self.col = 0;

        let attr = self.attr;

        for row in 0..self.height {
            for col in 0..self.width {
                self.put(' ' as u8, attr, row, col);
            }
        }

        Ok(())
    }

    fn get_cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    fn set_cursor(&mut self, row: usize, col: usize) -> fmt::Result {
        self.row = row;
        self.col = col;

        self.update_cursor();
        Ok(())
    }

    fn get_color(&self) -> (Color, Color) {
        (self.fg, self.bg)
    }

    fn set_color(&mut self, fg: Color, bg: Color) -> fmt::Result {
        self.fg = fg;
        self.bg = bg;
        self.update_attr();
        Ok(())
    }

    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result {
        match byte {
            0x0A /* newline */ => {
                self.new_line();
            },

            0x08 /* backspace */ => {
                if self.col > 0 {
                    self.col -= 1;
                }

                self.put_here(' ' as u8);
            },

            _ => {
                self.put_here(byte);

                if self.col + 1 >= self.width {
                    self.new_line();
                } else {
                    self.col += 1;
                }
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> fmt::Result {
        self.update_cursor();
        Ok(())
    }
}

impl fmt::Write for Vga {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        try!(self.write_raw_bytes(s.as_bytes()));
        try!(self.flush());
        Ok(())
    }
}

static mut CONSOLE: Option<Vga> = None;

pub fn console() -> &'static mut Terminal {
    unsafe {
        if CONSOLE.is_none() {
            CONSOLE = Some(Vga::new(80, 25,
                                    0xffffffff800b8000 as *mut u16, 0x3d4));
        }

        CONSOLE.as_mut().unwrap()
    }
}

pub mod ffi {
}
