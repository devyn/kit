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

use crate::framebuffer::Framebuffer;

use super::{Color, CharStream, Terminal};

/// Emulates a terminal on a graphical framebuffer
#[derive(Debug)]
pub struct Graphical<T> {
    fb:         T,
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

impl<T> Graphical<T> where T: Framebuffer {
    pub fn new(fb: T) -> Graphical<T> {
        let mut g = Graphical {
            fb,
            row: 0,
            col: 0,
            rows: 0,
            cols: 0,
            font_w: 9,
            font_h: 14,
            fg: Color::White,
            bg: Color::Black,
            native_fg: 0,
            native_bg: 0,
            char_buf: CharStream::default(),
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

        // TODO: use font
        self.fb.edit(x, y, self.font_w, self.font_h,
            |px, py, _| if (px+py)%2 == 0 { colors.0 } else { colors.1 });
    }

    fn update_cursor(&mut self) {
        // TODO
    }

    fn put_here(&self, ch: char) {
        self.render(ch, self.row, self.col, (self.native_fg, self.native_bg));
    }

    fn new_line(&mut self) {
        let (x, y) = self.position_xy(self.row, self.col);

        // Fill end of line
        let remaining = self.cols - self.col;
        self.fb.fill(x, y, self.font_w * remaining, self.font_h, self.native_bg);

        self.row += 1;
        self.col = 0;

        if self.row >= self.rows {
            // TODO: scroll
            self.row = self.rows - 1;
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

impl<T: Framebuffer> Terminal for Graphical<T> {
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
        debug!("{}", char::from(byte));
        // Build unicode
        self.char_buf.push(byte);

        while let Some(ch) = self.char_buf.consume() {
            self.write_char(ch);
        }

        Ok(())
    }

    fn flush(&mut self) -> fmt::Result {
        self.update_cursor();
        Ok(())
    }
}

impl<T: Framebuffer> fmt::Write for Graphical<T> {
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
