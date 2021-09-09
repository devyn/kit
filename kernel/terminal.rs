/*******************************************************************************
 *
 * kit/kernel/terminal.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Text mode terminal handler.

use core::fmt;
use core::mem;

use alloc::boxed::Box;

use crate::multiboot;
use crate::constants::translate_low_addr;

mod vga;
pub use vga::{VgaConfig, Vga};

mod graphical;
pub use graphical::Graphical;

/// Colors common to most terminals.
///
/// Numeric values correspond to the VGA text mode palette.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

static COLORS_RGB: [u32; 16] = [
    0x000000, //black
    0x000080, //blue
    0x008000, //green
    0x008080, //cyan
    0x800000, //red
    0x800080, //magenta
    0x808000, //brown
    0xc0c0c0, //light grey
    0x404040, //dark grey,
    0x0000ff, //light blue
    0x00ff00, //light green
    0x00ffff, //light cyan
    0xff0000, //light red
    0xff00ff, //light magenta
    0xffff00, //light brown (yellow)
    0xffffff, //white
];

impl Color {
    pub fn lighten(self) -> Color {
        LIGHT_COLORS[self as usize % 8]
    }

    pub fn darken(self) -> Color {
        DARK_COLORS[self as usize % 8]
    }

    pub fn to_rgb(self) -> u32 {
        COLORS_RGB[self as usize]
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
            self.write_raw_byte(*byte)?;
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

                30..=37 => { fg = ANSI_ATTR_TABLE[(attr - 30) as usize]; },
                40..=47 => { bg = ANSI_ATTR_TABLE[(attr - 40) as usize]; },

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
                b'0'..=b'9' => {
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

/// Ignores all commands and does nothing. Useful initial state.
pub struct Null;

impl Terminal for Null {
    fn reset(&mut self) -> fmt::Result {
        Ok(())
    }

    fn clear(&mut self) -> fmt::Result {
        Ok(())
    }

    fn get_cursor(&self) -> (usize, usize) {
        (0, 0)
    }

    fn set_cursor(&mut self, _row: usize, _col: usize) -> fmt::Result {
        Ok(())
    }

    fn get_color(&self) -> (Color, Color) {
        (Color::White, Color::Black)
    }

    fn set_color(&mut self, _fg: Color, _bg: Color) -> fmt::Result {
        Ok(())
    }

    fn put_raw_byte(&mut self,
                    _byte: u8,
                    _fg:   Color,
                    _bg:   Color,
                    _row:  usize,
                    _col:  usize) -> fmt::Result {

        Ok(())
    }

    fn write_raw_byte(&mut self, _byte: u8) -> fmt::Result {
        Ok(())
    }

    fn flush(&mut self) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Write for Null {
    fn write_char(&mut self, _ch: char) -> fmt::Result {
        Ok(())
    }

    fn write_str(&mut self, _s: &str) -> fmt::Result {
        Ok(())
    }
}

/// Build chars from streaming bytes
#[derive(Debug, Clone, Default)]
struct CharStream {
    char_buf: [u8; 4],
    char_index: u8,
}

impl CharStream {
    fn consume(&mut self) -> Option<char> {
        if self.char_index == 0 {
            return None;
        }

        let utf8 = &self.char_buf[0..self.char_index as usize];

        match core::str::from_utf8(utf8) {
            Ok(s) => {
                self.char_index = 0;
                s.chars().next()
            },
            Err(e) => match e.error_len() {
                None => {
                    // ok, wait for more bytes
                    None
                },
                Some(invalid_end) => {
                    // move invalid end to the beginning
                    self.char_buf.copy_within(invalid_end.., 0);
                    self.char_index -= invalid_end as u8;
                    // produce fffd (replacement char)
                    Some('\u{fffd}')
                },
            }
        }
    }

    /// Call consume() in a loop after calling push
    fn push(&mut self, byte: u8) {
        self.char_buf[self.char_index as usize] = byte;
        self.char_index += 1;
    }

    fn reset(&mut self) {
        *self = CharStream::default();
    }
}

static mut NULL: Null = Null;
static mut CONSOLE: Option<*mut dyn Terminal> = None;

/// Initialize the global console.
pub unsafe fn initialize(info: &multiboot::Info) {
    assert!(CONSOLE.is_none());

    match info.framebuffer_type {
        1 /* Linear framebuffer, rgb */ => {
            use crate::framebuffer::*;

            assert!((info.framebuffer_height as usize *
                info.framebuffer_pitch as usize) < 0x8000_0000);

            assert!(info.framebuffer_addr < usize::MAX as u64);

            let fb = LinearFramebuffer::map(LinearPixelConfig {
                buffer: 0xffff_ffff_0000_0000 as *mut u8,
                width: info.framebuffer_width as usize,
                height: info.framebuffer_height as usize,
                pitch: info.framebuffer_pitch as usize,
                bits_per_pixel: info.framebuffer_bpp,
                color_format: ColorFormat::Rgb {
                    red: MaskShift {
                        mask_bits: info.color_info.rgb.framebuffer_red_mask_size,
                        shift: info.color_info.rgb.framebuffer_red_field_position,
                    },
                    green: MaskShift {
                        mask_bits: info.color_info.rgb.framebuffer_green_mask_size,
                        shift: info.color_info.rgb.framebuffer_green_field_position,
                    },
                    blue: MaskShift {
                        mask_bits: info.color_info.rgb.framebuffer_blue_mask_size,
                        shift: info.color_info.rgb.framebuffer_blue_field_position,
                    },
                },
            }, info.framebuffer_addr as usize);

            let ansi_graphical = Ansi::new(Graphical::new(fb));

            CONSOLE = Some(Box::leak(Box::new(ansi_graphical)));
        },
        2 /* VGA */ => {
            assert!(info.framebuffer_addr < u32::MAX as u64);

            let ansi_vga = Ansi::new(Vga::new(VgaConfig {
                width: info.framebuffer_width as usize,
                height: info.framebuffer_height as usize,
                buffer: translate_low_addr::<u16>(info.framebuffer_addr as u32)
                    .unwrap() as *mut _,
                port: 0x3d4,
            }));

            CONSOLE = Some(Box::leak(Box::new(ansi_vga)));
        },
        _ => {
            warn!("Unknown framebuffer type {}. \
                Terminal will not be initialized", info.framebuffer_type);
        }
    }
}

/// Get the current global console.
pub fn console() -> &'static mut dyn Terminal {
    unsafe {
        if CONSOLE.is_none() {
            &mut NULL
        } else {
            CONSOLE.unwrap().as_mut().unwrap()
        }
    }
}

/// C (legacy) interface. See `kit/kernel/include/terminal.h`.
pub mod ffi {
    use super::*;
    
    use core::mem;
    use core::slice;

    use crate::c_ffi::{c_char, size_t};

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
