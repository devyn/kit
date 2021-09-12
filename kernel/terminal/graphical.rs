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

use core::fmt;
use core::ops::Range;

use alloc::collections::BTreeMap;

use crate::framebuffer::Framebuffer;
use crate::util::align_up;

use super::{Color, CharStream, Terminal};

/// Emulates a terminal on a graphical framebuffer
#[derive(Debug)]
pub struct Graphical<T, U> {
    fb:         T,
    font:       U,
    row:        usize,
    col:        usize,
    rows:       usize,
    cols:       usize,
    font_w:     usize,
    font_h:     usize,
    fg:         Color,
    bg:         Color,
    native_fg:  u32,
    native_bg:  u32,
    char_buf:   CharStream,
}

impl<T, U> Graphical<T, U> where T: Framebuffer, U: Font {
    pub fn new(fb: T, font: U) -> Graphical<T, U> {
        let mut g = Graphical {
            row: 0,
            col: 0,
            rows: 0,
            cols: 0,
            font_w: font.char_width() + 1,
            font_h: font.char_height(),
            fg: Color::White,
            bg: Color::Black,
            native_fg: 0,
            native_bg: 0,
            char_buf: CharStream::default(),
            fb,
            font,
        };

        g.rows = g.fb.height()/g.font_h;
        g.cols = g.fb.width()/g.font_w;

        g.update_colors();

        g
    }

    fn update_colors(&mut self) {
        self.native_fg = self.fb.color_format().format(self.fg.to_rgb());
        self.native_bg = self.fb.color_format().format(self.bg.to_rgb());
    }

    fn position_xy(&self, row: usize, col: usize) -> (usize, usize) {
        (
            self.font_w * col,
            self.font_h * row
        )
    }

    fn render(&self, ch: char, row: usize, col: usize, colors: (u32, u32)) {
        let (x, y) = self.position_xy(row, col);

        let fontdata = self.font.get(ch);
        let c_width = self.font.char_width();

        let get_bit =
            |px, py, _| {
                let fg = if let Some(bits) = fontdata {
                    if px >= c_width {
                        false
                    } else {
                        let index = py * c_width + px;
                        let byte = index >> 3;
                        let bit  = index & 0b111;
                        let mask = 1u8 << (7 - bit);

                        bits[byte] & mask != 0u8
                    }
                } else {
                    false
                };

                if fg { colors.0 } else { colors.1 }
            };

        self.fb.edit(x, y, self.font_w, self.font_h, get_bit);
    }

    fn update_cursor(&mut self) {
        // TODO
    }

    fn put_here(&self, ch: char) {
        self.render(ch, self.row, self.col, (self.native_fg, self.native_bg));
    }

    fn scroll(&self) {
        let (x0, y0) = self.position_xy(1, 0);
        let (x1, y1) = self.position_xy(0, 0);

        // Move the contents of everything but the first row up one row
        self.fb.copy_within(x0, y0, x1, y1,
            self.font_w * self.cols, self.font_h * (self.rows-1));

        // Clear the last row
        let (x2, y2) = self.position_xy(self.rows - 1, 0);
        self.fb.fill(x2, y2,
            self.font_w * self.cols, self.font_h,
            self.native_bg);
    }

    fn new_line(&mut self) {
        let (x, y) = self.position_xy(self.row, self.col);

        // Fill end of line
        let remaining = self.cols - self.col;
        self.fb.fill(x, y, self.font_w * remaining, self.font_h, self.native_bg);

        self.row += 1;
        self.col = 0;

        if self.row >= self.rows {
            self.row = self.rows - 1;
            self.scroll();
        }
    }

    fn write_char(&mut self, ch: char) {
        match ch {
            '\n' => {
                self.new_line();
            },

            '\x08' /* backspace */ => {
                if self.col > 0 {
                    self.col -= 1;
                }

                self.put_here(' ');
            },

            _ => {
                self.put_here(ch);
                self.col += 1;

                if self.col >= self.cols {
                    self.new_line();
                }
            }
        }
    }
}

impl<T, U> Terminal for Graphical<T, U> where T: Framebuffer, U: Font {
    fn reset(&mut self) -> fmt::Result {
        self.fg = Color::LightGrey;
        self.bg = Color::Black;
        self.update_colors();
        self.clear()
    }

    fn clear(&mut self) -> fmt::Result {
        self.row = 0;
        self.col = 0;

        self.fb.fill(0, 0, self.fb.width(), self.fb.height(), self.native_bg);

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
        self.update_colors();
        Ok(())
    }

    fn put_raw_byte(&mut self,
                    byte: u8,
                    fg:   Color,
                    bg:   Color,
                    row:  usize,
                    col:  usize) -> fmt::Result {
        // I think we should get rid of this method, it doesn't handle unicode properly
        self.render(
            char::from(byte),
            row,
            col,
            (self.fb.color_format().format(fg.to_rgb()),
             self.fb.color_format().format(bg.to_rgb())));
        Ok(())
    }

    fn write_raw_byte(&mut self, byte: u8) -> fmt::Result {
        // Build unicode
        self.char_buf.push(byte);

        while let Some(ch) = self.char_buf.consume() {
            self.write_char(ch);
        }

        Ok(())
    }

    fn set_double_buffer(&mut self, enabled: bool) {
        self.fb.set_double_buffer(enabled);
    }

    fn flush(&mut self) -> fmt::Result {
        self.update_cursor();
        Ok(())
    }
}

impl<T, U> fmt::Write for Graphical<T, U> where T: Framebuffer, U: Font {
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

static U_VGA16: &[u8] = include_bytes!("font/u_vga16.psf");

pub fn u_vga16() -> PsfFont<&'static [u8]> {
    PsfFont::new(U_VGA16).unwrap()
}

pub trait Font {
    fn char_width(&self) -> usize;
    fn char_height(&self) -> usize;
    fn char_bytes(&self) -> usize {
        align_up(self.char_width() * self.char_height(), 8) / 8
    }

    fn get(&self, ch: char) -> Option<&[u8]>;
}

#[derive(Debug, Clone)]
pub struct PsfFont<T: AsRef<[u8]>> {
    buf: T,
    chars_range: Range<usize>,
    unicode_table: BTreeMap<char, usize>,
    char_width: usize,
    char_height: usize,
}

const PSF1_MAGIC: u16 = 0x0436; /* little endian */
const PSF1_MODE_512: u8 = 0x01;
const PSF1_MODE_HASTAB: u8 = 0x02;
const PSF1_MODE_HASSEQ: u8 = 0x04;

const PSF1_SEPARATOR: u32 = 0xffff;
const PSF1_STARTSEQ: u32 = 0xfffe;

#[derive(Debug, Clone)]
pub enum PsfError {
    /// this is not a recognized PC Screen Font file
    WrongMagic,
    /// the mode field ({:x}) has unrecognized bits
    BadMode(u8),
    /// file is smaller than it should be
    FileTooSmall,
}

fn read_u16(slice: &[u8]) -> u16 {
    let mut bytes: [u8; 2] = [0; 2];
    bytes.copy_from_slice(&slice[0..2]);

    u16::from_le_bytes(bytes)
}

impl<T> PsfFont<T> where T: AsRef<[u8]> {
    pub fn new(buf: T) -> Result<PsfFont<T>, PsfError> {
        let file: &[u8] = buf.as_ref();

        if read_u16(&file[0..2]) != PSF1_MAGIC {
            return Err(PsfError::WrongMagic);
        }

        let mode: u8 = file[2];

        if mode & 0x07 != mode {
            return Err(PsfError::BadMode(mode));
        }

        let mode_512    = mode & PSF1_MODE_512 != 0;
        let mode_hastab = mode & PSF1_MODE_HASTAB != 0;
        let mode_hasseq = mode & PSF1_MODE_HASSEQ != 0;

        let charsize: usize = file[3] as usize; // height, also number of bytes

        let char_width = 8;
        let char_height = charsize;

        let length = if mode_512 { 512 } else { 256 };

        let chars_range = 4 .. (4 + charsize * length);

        // File too small if chars_range doesn't fit
        if chars_range.end > file.len() {
            return Err(PsfError::FileTooSmall);
        }

        let mut unicode_table = BTreeMap::new();

        // Parse the unicode table, or generate it if it's not there
        if mode_hastab {
            let unicode_table_buf = &file[chars_range.end ..];

            let mut pos = 0;

            'eof: for index in 0..length {
                loop {
                    if pos >= file.len() {
                        break 'eof;
                    }

                    let codepoint = read_u16(&unicode_table_buf[pos..]) as u32;

                    pos += 2;

                    if codepoint == PSF1_SEPARATOR {
                        // End of sequences for this index
                        break;
                    } else if codepoint == PSF1_STARTSEQ && mode_hasseq {
                        // Sequences not supported yet.
                        pos += 4;
                    } else if let Some(c) = char::from_u32(codepoint) {
                        // Insert codepoint.
                        unicode_table.insert(c, index);
                    }
                }
            }
        } else {
            for index in 0..length {
                if let Some(c) = char::from_u32(index as u32) {
                    unicode_table.insert(c, index);
                }
            }
        }

        Ok(PsfFont {
            buf,
            chars_range,
            unicode_table,
            char_width,
            char_height
        })
    }
}

impl<T> Font for PsfFont<T> where T: AsRef<[u8]> {
    fn char_width(&self) -> usize {
        self.char_width
    }

    fn char_height(&self) -> usize {
        self.char_height
    }

    fn get(&self, ch: char) -> Option<&[u8]> {
        self.unicode_table.get(&ch).map(|index| {
            let start = self.char_height * index;
            &self.buf.as_ref()[self.chars_range.clone()][start .. (start + self.char_height)]
        })
    }
}
