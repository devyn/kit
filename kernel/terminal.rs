/*******************************************************************************
 *
 * kit/kernel/terminal.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Early text mode 80x25 terminal handler.

use core::fmt;
use core::mem;

/// Colors common to most terminals.
///
/// Numeric values correspond to the VGA text mode palette.
#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Color {
    Black        = 0,
    Blue         = 1,
    Green        = 2,
    Cyan         = 3,
    Red          = 4,
    Magenta      = 5,
    Brown        = 6,
    LightGrey    = 7,
    DarkGrey     = 8,
    LightBlue    = 9,
    LightGreen   = 10,
    LightCyan    = 11,
    LightRed     = 12,
    LightMagenta = 13,
    LightBrown   = 14,
    White        = 15,
}

static LIGHT_COLORS: [Color; 8] = [
    Color::DarkGrey,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightCyan,
    Color::LightRed,
    Color::LightMagenta,
    Color::LightBrown,
    Color::White,
];

static DARK_COLORS: [Color; 8] = [
    Color::Black,
    Color::Blue,
    Color::Green,
    Color::Cyan,
    Color::Red,
    Color::Magenta,
    Color::Brown,
    Color::LightGrey,
];

impl Color {
    pub fn lighten(self) -> Color {
        LIGHT_COLORS[self as usize % 8]
    }

    pub fn darken(self) -> Color {
        DARK_COLORS[self as usize % 8]
    }
}

/// A terminal.
pub trait Terminal: fmt::Write {
    /// Reset the terminal to its initial state.
    fn reset(&mut self) -> fmt::Result;

    /// Clear the terminal buffer.
    fn clear(&mut self) -> fmt::Result;

    /// Get the current `(row, col)` position of the cursor.
    fn get_cursor(&self) -> (usize, usize);

    /// Set the cursor to a given `(row, col)` position.
    fn set_cursor(&mut self, row: usize, col: usize) -> fmt::Result;

    /// Get the current color set of the terminal.
    ///
    /// These colors are used for every method that writes to the terminal except
    /// `put_raw_byte()`, which specifies its own set of foreground and
    /// background colors.
    fn get_color(&self) -> (Color, Color);

    /// Set the current color set of the terminal.
    ///
    /// These colors must be used for any subsequent calls to any methods that
    /// write to the terminal, with the exception of `put_raw_byte()`, which
    /// specifies its own set of foreground and background colors.
    fn set_color(&mut self, fg: Color, bg: Color) -> fmt::Result;

    /// Put a byte at the given `(row, col)` position with the given foreground
    /// and background colors, without changing the cursor or the current color
    /// set of the terminal.
    ///
    /// The results of `get_cursor()` and `get_color()` must not be changed by
    /// this function.
    fn put_raw_byte(&mut self,
                    byte: u8,
                    fg:   Color,
                    bg:   Color,
                    row:  usize,
                    col:  usize) -> fmt::Result;

    /// Write a raw byte at the current cursor position with the current color
    /// set.
    ///
    /// May not `flush()`, if applicable.
    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result;

    /// Write a byte slice at the current cursor position with the current color
    /// set.
    ///
    /// May not `flush()`, if applicable.
    fn write_raw_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        for byte in bytes {
            try!(self.write_raw_byte(*byte));
        }

        Ok(())
    }

    /// Flushes any internally deferred commands or buffers, if applicable.
    ///
    /// The `Write` implementation should automatically call this.
    ///
    /// For example, `Vga` uses this to update the cursor, since writing to IO
    /// ports can be slow.
    fn flush(&mut self) -> fmt::Result;
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
    /// Create a new VGA text-mode terminal controller with the given
    /// dimensions, buffer, and port.
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
            asm!("out %al, %dx" :: "{ax}" (byte), "{dx}" (port) :: "volatile");
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

                // XXX: SSE memory operations fail on memory-mapped I/O in KVM,
                // so inhibit vectorization
                unsafe { asm!("" :::: "volatile"); }
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

                // XXX: SSE memory operations fail on memory-mapped I/O in KVM,
                // so inhibit vectorization
                unsafe { asm!("" :::: "volatile"); }
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

                if self.col >= self.width {
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
        let mut buf = [0u8, 4];

        let size = try!(ch.encode_utf8(&mut buf).ok_or(fmt::Error));

        try!(self.write_raw_bytes(&buf[0..size]));
        try!(self.flush());
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> fmt::Result {
        try!(self.write_raw_bytes(s.as_bytes()));
        try!(self.flush());
        Ok(())
    }
}

/// Wraps a Terminal to make it compatible with ANSI escape sequences.
pub struct Ansi<T> {
    term:  T,
    state: AnsiState,
    bold:  bool,
}

#[derive(Debug, Clone, Copy)]
enum AnsiState {
    Normal,
    StartSeq,
    Csi(u8, bool, [u8; 16])
}

impl AnsiState {
    fn csi_digit(&mut self, digit: u8) {
        match *self {
            AnsiState::Csi(ref mut size, ref mut new, ref mut buf) => {
                if *new {
                    if (*size as usize) >= buf.len() { return }

                    *size += 1;
                    *new   = false;
                }

                let index = (*size as usize) - 1;

                buf[index] = buf[index] * 10 + digit;
            },

            _ => panic!("csi_digit() called on {:?}", self)
        }
    }

    fn csi_push(&mut self) {
        match *self {
            AnsiState::Csi(ref mut size, ref mut new, ref mut buf) => {
                if *new {
                    if (*size as usize) < buf.len() {
                        *size += 1;
                    }
                } else {
                    *new = true;
                }
            },

            _ => panic!("csi_push() called on {:?}", self)
        }
    }

    fn csi_finish(&self) -> &[u8] {
        match *self {
            AnsiState::Csi(size, _, ref buf) => &buf[0..(size as usize)],

            _ => panic!("csi_finish() called on {:?}", self)
        }
    }
}

static ANSI_ATTR_TABLE: [Color; 8] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Brown,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::LightGrey,
];

impl<T: Terminal> Ansi<T> {
    /// Wrap an existing Terminal to make it compatible with ANSI escape
    /// sequences.
    pub fn new(term: T) -> Ansi<T> {
        Ansi { term: term, state: AnsiState::Normal, bold: false }
    }

    fn csi_sgr(&mut self) -> fmt::Result {
        let state = mem::replace(&mut self.state, AnsiState::Normal);

        let (mut fg, mut bg) = self.term.get_color();

        for attr in state.csi_finish() {
            match *attr {
                0 => {
                    self.bold = false;

                    fg = Color::LightGrey;
                    bg = Color::Black;
                },

                1 => {
                    self.bold = true;
                },

                22 => {
                    self.bold = false;
                },

                30...37 => { fg = ANSI_ATTR_TABLE[(attr - 30) as usize]; },
                40...47 => { bg = ANSI_ATTR_TABLE[(attr - 40) as usize]; },

                _ => ()
            }
        }

        if self.bold {
            fg = fg.lighten();
        } else {
            fg = fg.darken();
        }

        self.term.set_color(fg, bg)
    }
}

impl<T: Terminal> Terminal for Ansi<T> {
    fn reset(&mut self) -> fmt::Result {
        self.term.reset().and_then(|_| {
            self.state = AnsiState::Normal;
            self.bold  = false;
            Ok(())
        })
    }

    fn clear(&mut self) -> fmt::Result {
        self.term.clear()
    }

    fn get_cursor(&self) -> (usize, usize) {
        self.term.get_cursor()
    }

    fn set_cursor(&mut self, row: usize, col: usize) -> fmt::Result {
        self.term.set_cursor(row, col)
    }

    fn get_color(&self) -> (Color, Color) {
        self.term.get_color()
    }

    fn set_color(&mut self, fg: Color, bg: Color) -> fmt::Result {
        self.term.set_color(fg, bg)
    }

    fn put_raw_byte(&mut self,
                    byte: u8,
                    fg:   Color,
                    bg:   Color,
                    row:  usize,
                    col:  usize) -> fmt::Result {

        self.term.put_raw_byte(byte, fg, bg, row, col)
    }

    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result {
        match self.state {
            AnsiState::Normal if byte == 0x1b /* escape */ => {
                self.state = AnsiState::StartSeq;
                Ok(())
            },

            AnsiState::Normal =>
                self.term.write_raw_byte(byte),

            AnsiState::StartSeq => match byte {
                b'[' => {
                    self.state = AnsiState::Csi(1, false, [0; 16]);
                    Ok(())
                },

                _ => {
                    self.state = AnsiState::Normal;
                    self.term.write_raw_byte(byte)
                }
            },

            AnsiState::Csi(_,_,_) => match byte {
                b'0'...b'9' => {
                    self.state.csi_digit(byte - b'0');
                    Ok(())
                },

                b';' => {
                    self.state.csi_push();
                    Ok(())
                },

                b'm' => {
                    self.csi_sgr()
                },

                _ => {
                    self.state = AnsiState::Normal;
                    Ok(())
                }
            }
        }
    }

    fn flush(&mut self) -> fmt::Result {
        self.term.flush()
    }
}

impl<T: Terminal> fmt::Write for Ansi<T> {
    fn write_char(&mut self, ch: char) -> fmt::Result {
        let mut buf = [0u8, 4];

        let size = try!(ch.encode_utf8(&mut buf).ok_or(fmt::Error));

        try!(self.write_raw_bytes(&buf[0..size]));
        try!(self.flush());
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> fmt::Result {
        try!(self.write_raw_bytes(s.as_bytes()));
        try!(self.flush());
        Ok(())
    }
}

static mut CONSOLE: Option<Ansi<Vga>> = None;

/// Get the current global console.
pub fn console() -> &'static mut Terminal {
    unsafe {
        if CONSOLE.is_none() {
            CONSOLE = Some(Ansi::new(Vga::new(
                        80, 25, 0xffffffff800b8000 as *mut u16, 0x3d4)));
        }

        CONSOLE.as_mut().unwrap()
    }
}

/// C (legacy) interface. See `kit/kernel/include/terminal.h`.
pub mod ffi {
    use super::*;
    
    use core::mem;
    use core::slice;

    use c_ffi::{c_char, size_t};

    #[no_mangle]
    pub extern fn terminal_initialize() {
        console().reset().unwrap();
    }

    #[no_mangle]
    pub extern fn terminal_clear() {
        console().clear().unwrap();
    }

    #[no_mangle]
    pub extern fn terminal_updatecursor() {
        console().flush().unwrap();
    }

    #[no_mangle]
    pub unsafe extern fn terminal_getcursor(row: *mut size_t, column: *mut size_t) {
        let (row_us, col_us) = console().get_cursor();

        *row    = row_us as size_t;
        *column = col_us as size_t;
    }

    #[no_mangle]
    pub extern fn terminal_setcursor(row: size_t, column: size_t) {
        console().set_cursor(row as usize, column as usize).unwrap();
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub enum VgaColor {
        Black        = 0,
        Blue         = 1,
        Green        = 2,
        Cyan         = 3,
        Red          = 4,
        Magenta      = 5,
        Brown        = 6,
        LightGrey    = 7,
        DarkGrey     = 8,
        LightBlue    = 9,
        LightGreen   = 10,
        LightCyan    = 11,
        LightRed     = 12,
        LightMagenta = 13,
        LightBrown   = 14,
        White        = 15,
    }

    impl VgaColor {
        pub fn from_color(color: Color) -> VgaColor {
            unsafe { mem::transmute(color as i32) }
        }

        pub fn to_color(self) -> Color {
            unsafe { mem::transmute(self as u8) }
        }
    }

    #[no_mangle]
    pub unsafe extern fn terminal_getcolor(fg: *mut VgaColor,
                                           bg: *mut VgaColor) {
        let (fg_c, bg_c) = console().get_color();

        *fg = VgaColor::from_color(fg_c);
        *bg = VgaColor::from_color(bg_c);
    }

    #[no_mangle]
    pub extern fn terminal_setcolor(fg: VgaColor, bg: VgaColor) {
        console().set_color(fg.to_color(), bg.to_color()).unwrap();
    }

    #[no_mangle]
    pub unsafe extern fn terminal_putentryat(c: c_char,
                                      color: u8,
                                      x: size_t,
                                      y: size_t) {

        let fg_v: VgaColor = mem::transmute((color & 0x0f) as i32);
        let bg_v: VgaColor = mem::transmute((color >> 4)   as i32);

        console()
            .put_raw_byte(c as u8,
                          fg_v.to_color(),
                          bg_v.to_color(),
                          y as usize,
                          x as usize)
            .unwrap();
    }

    #[no_mangle]
    pub extern fn terminal_newline() {
        console().write_raw_byte('\n' as u8).unwrap();
        console().flush().unwrap();
    }

    #[no_mangle]
    pub extern fn terminal_writechar_internal(c: c_char) {
        console().write_raw_byte(c as u8).unwrap();
    }

    #[no_mangle]
    pub extern fn terminal_writechar(c: c_char) {
        console().write_raw_byte(c as u8).unwrap();
        console().flush().unwrap();
    }

    #[no_mangle]
    pub unsafe extern fn terminal_writebuf(length: u64, buffer: *const u8) {
        let bytes = slice::from_raw_parts(buffer, length as usize);

        console().write_raw_bytes(bytes).unwrap();
        console().flush().unwrap();
    }

    #[no_mangle]
    pub unsafe extern fn terminal_writestring(data: *const u8) {
        let mut data_len = 0usize;
        let mut data_end = data;

        while *data_end != 0 {
            data_len += 1;
            data_end = data_end.offset(1);
        }

        let bytes = slice::from_raw_parts(data, data_len);

        console().write_raw_bytes(bytes).unwrap();
        console().flush().unwrap();
    }
}
