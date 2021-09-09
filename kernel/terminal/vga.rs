/*******************************************************************************
 *
 * kit/kernel/terminal/vga.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use super::{Terminal, Color};

use core::fmt;

/// Configuration for a VGA text-mode terminal.
#[derive(Debug, Clone, Copy)]
pub struct VgaConfig {
    pub width:  usize,
    pub height: usize,
    pub buffer: *mut u16,
    pub port:   u16,
}

/// Controls a VGA text-mode terminal.
#[derive(Debug)]
pub struct Vga {
    config: VgaConfig,
    row:    usize,
    col:    usize,
    fg:     Color,
    bg:     Color,
    attr:   u8,
}

impl Vga {
    /// Create a new VGA text-mode terminal controller with the given
    /// dimensions, buffer, and port.
    pub unsafe fn new(config: VgaConfig) -> Vga {
        let mut vga = Vga {
            config,
            row:    0,
            col:    0,
            fg:     Color::LightGrey,
            bg:     Color::Black,
            attr:   Vga::attr(Color::LightGrey, Color::Black),
        };

        vga.reset().unwrap();

        vga
    }

    pub fn config(&self) -> VgaConfig {
        self.config
    }

    pub fn color(c: Color) -> u8 {
        c as u8
    }

    pub fn attr(fg: Color, bg: Color) -> u8 {
        Vga::color(fg) | (Vga::color(bg) << 4)
    }

    fn update_attr(&mut self) {
        self.attr = Vga::attr(self.fg, self.bg);
    }

    fn update_cursor(&mut self) {
        unsafe fn outb(byte: u8, port: u16) {
            asm!("out %al, %dx", in("al") byte, in("dx") port, options(att_syntax));
        }

        let pos: u16 = ((self.row * self.config.width) + self.col) as u16;

        unsafe {
            outb(0x0F,             self.config.port);
            outb(pos as u8,        self.config.port + 1);

            outb(0x0E,             self.config.port);
            outb((pos >> 8) as u8, self.config.port + 1);
        }
    }

    pub fn put(&mut self, byte: u8, attr: u8, row: usize, col: usize) {
        unsafe {
            *self.config.buffer.offset((row * self.config.width + col) as isize) =
                (byte as u16) | ((attr as u16) << 8);
        }
    }

    pub fn put_here(&mut self, byte: u8) {
        let (attr, row, col) = (self.attr, self.row, self.col);

        self.put(byte, attr, row, col)
    }

    fn new_line(&mut self) {
        // Clear to the end of the line.
        while self.col < self.config.width {
            self.put_here(' ' as u8);
            self.col += 1;
        }

        // Go to the next line, scrolling if necessary.
        self.col  = 0;
        self.row += 1;

        while self.row >= self.config.height {
            self.scroll();
            self.row -= 1;
        }

        self.update_cursor();
    }

    fn scroll(&mut self) {
        // Shift everything one line back.
        for row in 1..self.config.height {
            for col in 0..self.config.width {
                let index = (row * self.config.width + col) as isize;

                unsafe {
                    *self.config.buffer.offset(index - self.config.width as isize) =
                        *self.config.buffer.offset(index);
                }

                // XXX: SSE memory operations fail on memory-mapped I/O in KVM,
                // so inhibit vectorization
                unsafe { asm!("nop"); }
            }
        }

        // Clear last line.
        let (attr, height) = (self.attr, self.config.height);

        for col in 0..self.config.width {
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

        for row in 0..self.config.height {
            for col in 0..self.config.width {
                self.put(' ' as u8, attr, row, col);

                // XXX: SSE memory operations fail on memory-mapped I/O in KVM,
                // so inhibit vectorization
                unsafe { asm!("nop"); }
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

    fn put_raw_byte(&mut self,
                    byte: u8,
                    fg:   Color,
                    bg:   Color,
                    row:  usize,
                    col:  usize) -> fmt::Result {

        self.put(byte, Vga::attr(fg, bg), row, col);
        Ok(())
    }

    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result {
        match byte {
            b'\n' => {
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
                self.col += 1;

                if self.col >= self.config.width {
                    self.new_line();
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
    fn write_char(&mut self, ch: char) -> fmt::Result {
        let mut buf = [0; 4];
        self.write_raw_bytes(ch.encode_utf8(&mut buf).as_bytes())?;
        self.flush()?;
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_raw_bytes(s.as_bytes())?;
        self.flush()?;
        Ok(())
    }
}
