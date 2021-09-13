/*******************************************************************************
 *
 * kit/kernel/framebuffer.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::slice;

use alloc::vec::Vec;

use crate::terminal::VgaConfig;
use crate::paging::{PAGE_SIZE, PagesetExt, kernel_pageset, PageType};
use crate::util::align_up;
use crate::sync::Spinlock;

/// Configuration for a framebuffer, in either text mode or graphics mode.
#[derive(Debug, Clone)]
pub enum FramebufferConfig {
    VgaTextMode(VgaConfig),
    LinearPixel(LinearPixelConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearPixelConfig {
    pub buffer: *mut u8,
    pub pitch: usize,
    pub width: usize,
    pub height: usize,
    pub bits_per_pixel: u8,
    pub color_format: ColorFormat,
}

impl LinearPixelConfig {
    pub fn size(&self) -> usize {
        self.size_in_bytes() / (self.bits_per_pixel / 8) as usize
    }

    pub fn size_in_bytes(&self) -> usize {
        self.pitch * self.height
    }

    pub fn size_in_pages(&self) -> usize {
        align_up(self.size_in_bytes(), PAGE_SIZE) / PAGE_SIZE
    }

    pub fn bytes_per_pixel(&self) -> usize {
        align_up(self.bits_per_pixel as usize, 8) / 8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorFormat {
    Rgb {
        red: MaskShift,
        green: MaskShift,
        blue: MaskShift,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskShift {
    pub mask_bits: u8,
    pub shift: u8,
}

impl MaskShift {
    #[inline]
    pub const fn shift(self, value: u8) -> u32 {
        ((value >> (8 - self.mask_bits)) as u32) << self.shift
    }
}

const RGB24: ColorFormat = ColorFormat::Rgb {
    red: MaskShift { mask_bits: 8, shift: 0 },
    green: MaskShift { mask_bits: 8, shift: 8 },
    blue: MaskShift { mask_bits: 8, shift: 16 },
};

const BGR24: ColorFormat = ColorFormat::Rgb {
    red: MaskShift { mask_bits: 8, shift: 16 },
    green: MaskShift { mask_bits: 8, shift: 8 },
    blue: MaskShift { mask_bits: 8, shift: 0 },
};

impl ColorFormat {
    /// Convert an rgb color to the native color format
    #[inline]
    pub fn format(&self, rgb: u32) -> u32 {
        match *self {
            _ if *self == RGB24 => {
                u32::from_be(rgb << 8)
            },
            _ if *self == BGR24 => {
                u32::from_le(rgb)
            },
            ColorFormat::Rgb { red, green, blue } => {
                red.shift((rgb >> 16) as u8) |
                green.shift((rgb >> 8) as u8) |
                blue.shift(rgb as u8)
            }
        }
    }
}

#[test]
fn test_color_format_rgb24() {
    let color_format = RGB24;

    let b = u32::from_ne_bytes;

    assert_eq!(color_format.format(0x000000), b([0x00, 0x00, 0x00, 0x00]));
    assert_eq!(color_format.format(0xffffff), b([0xff, 0xff, 0xff, 0x00]));
    assert_eq!(color_format.format(0x380a28), b([0x38, 0x0a, 0x28, 0x00]));
}

#[test]
fn test_color_format_bgr24() {
    let color_format = BGR24;

    let b = u32::from_ne_bytes;

    assert_eq!(color_format.format(0x000000), b([0x00, 0x00, 0x00, 0x00]));
    assert_eq!(color_format.format(0xffffff), b([0xff, 0xff, 0xff, 0x00]));
    assert_eq!(color_format.format(0x38f128), b([0x28, 0xf1, 0x38, 0x00]));
}

#[test]
fn test_color_format_rgb8() {
    let color_format = ColorFormat::Rgb {
        red: MaskShift { mask_bits: 2, shift: 0 },
        green: MaskShift { mask_bits: 3, shift: 2 },
        blue: MaskShift { mask_bits: 3, shift: 5 },
    };

    assert_eq!(color_format.format(0x000000), 0b00000000);
    assert_eq!(color_format.format(0xffffff), 0b11111111);
    assert_eq!(color_format.format(0xff9922), 0b00110011);
    assert_eq!(color_format.format(0x8038cc), 0b11000110);
}

pub trait Framebuffer {
    fn color_format(&self) -> &ColorFormat;

    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn double_buffer_enabled(&self) -> bool;
    fn set_double_buffer(&self, enabled: bool);

    /// Edit the pixels in the specified rectanglar region according to the
    /// given `mapper` function, which takes `(x, y, old_native_color)`.
    fn edit<F>(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        mapper: F,
    ) where
        F: FnMut(usize, usize, u32) -> u32;

    /// Fill rectangular region with the specified native color.
    fn fill(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: u32,
    ) {
        self.edit(x, y, width, height, |_, _, _| color);
    }

    /// Copy rectangle within the framebuffer, from the coordinates specified by
    /// `(sx, sy)` to `(dx, dy)`.
    fn copy_within(
        &self,
        sx: usize,
        sy: usize,
        dx: usize,
        dy: usize,
        width: usize,
        height: usize,
    );
}

#[derive(Debug)]
pub struct LinearFramebuffer {
    config: LinearPixelConfig,
    buffer: Spinlock<GuardedBuffer>,
}

#[derive(Debug)]
struct GuardedBuffer {
    slice: &'static mut [u8],
    shadow: Option<Vec<u8>>,
}

impl GuardedBuffer {
    fn slice(&mut self) -> &mut [u8] {
        if let Some(ref mut shadow) = self.shadow {
            &mut shadow[..]
        } else {
            &mut self.slice[..]
        }
    }

    fn dirty(
        &mut self,
        config: &LinearPixelConfig,
        x: usize,
        y: usize,
        width: usize,
        height: usize
    ) {
        // If double buffering is enabled
        if let Some(ref shadow) = self.shadow {
            let pitch = config.pitch;

            let bpp = config.bytes_per_pixel();

            // Starting position and width expressed in terms of bytes
            let x_offset = x * bpp;
            let width_bytes = width * bpp;

            for py in y..(y + height) {
                // Copy the bytes from the shadow buffer to the real fb, taking
                // the x offset and width into consideration
                let start = py * pitch + x_offset;
                let end = start + width_bytes;
                self.slice[start..end].copy_from_slice(&shadow[start..end]);
            };
        }
    }
}

impl LinearFramebuffer {
    pub unsafe fn new(config: LinearPixelConfig) -> LinearFramebuffer {
        LinearFramebuffer {
            buffer: Spinlock::new(GuardedBuffer {
                slice: slice::from_raw_parts_mut(
                    config.buffer,
                    config.size_in_bytes(),
                ),
                shadow: None,
            }),
            config,
        }
    }

    /// Map the framebuffer to the pointer specified in the config and then
    /// create it.
    pub unsafe fn map(config: LinearPixelConfig, paddr: usize)
        -> LinearFramebuffer {

        kernel_pageset().map_pages_with_type(
            config.buffer as usize,
            (paddr..).step_by(PAGE_SIZE).take(config.size_in_pages()),
            PageType::default().writable()).unwrap();

        LinearFramebuffer::new(config)
    }

    /// Generic version of edit. Transmutes the byte buffer to the specified
    /// size.
    #[inline]
    fn edit_gen<F, T>(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        mut mapper: F,
    )
    where
        F: FnMut(usize, usize, T) -> T,
        T: Copy,
    {
        let mut buf = self.buffer.lock();

        {
            let (_, slice_gen, _) = unsafe { buf.slice().align_to_mut::<T>() };

            let pitch = self.config.pitch / core::mem::size_of::<T>();
            let row_width = self.config.width as usize;

            for cy in 0..height {
                let row = &mut slice_gen[
                    (y + cy) * pitch .. (y + cy) * pitch + row_width];

                for cx in 0..width {
                    row[x + cx] = mapper(cx, cy, row[x + cx]);
                }
            }
        }

        buf.dirty(&self.config, x, y, width, height);
    }
}

impl Framebuffer for LinearFramebuffer {
    fn color_format(&self) -> &ColorFormat {
        &self.config.color_format
    }

    fn width(&self) -> usize {
        self.config.width
    }

    fn height(&self) -> usize {
        self.config.height
    }

    fn double_buffer_enabled(&self) -> bool {
        self.buffer.lock().shadow.is_some()
    }

    fn set_double_buffer(&self, enabled: bool) {
        let mut buf = self.buffer.lock();

        if enabled {
            if buf.shadow.is_none() {
                let mut new_vec: Vec<u8> = Vec::with_capacity(buf.slice.len());

                debug!("Double buffering on {:p} x {} -> {:p}",
                    buf.slice, buf.slice.len(), &new_vec[..]);

                // Copy from underlying framebuffer. Likely very slow, so try
                // not to do this too often.
                new_vec.extend(buf.slice.iter().cloned());

                buf.shadow = Some(new_vec);
            }
        } else {
            buf.shadow = None;
        }
    }

    #[inline]
    fn edit<F>(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        mut mapper: F,
    ) where
        F: FnMut(usize, usize, u32) -> u32,
    {
        macro_rules! adapt {
            ($t:ty) => {
                |x, y, old_value| mapper(x, y, old_value as u32) as $t
            };
        }
        match self.config.bits_per_pixel {
            32 => {
                self.edit_gen::<_, u32>(x, y, width, height, mapper)
            }
            15 | 16 => {
                self.edit_gen::<_, u16>(x, y, width, height, adapt!(u16))
            }
            1 | 4 | 8 => {
                self.edit_gen::<_, u8>(x, y, width, height, adapt!(u8))
            }
            other => panic!("Can't handle bits_per_pixel = {}", other),
        }
    }

    fn copy_within(
        &self,
        sx: usize,
        sy: usize,
        dx: usize,
        dy: usize,
        width: usize,
        height: usize
    ) {
        let mut buf = self.buffer.lock();

        let pitch = self.config.pitch;

        let bpp = self.config.bytes_per_pixel();

        // Starting positions and width expressed in terms of bytes
        let sx_offset = sx * bpp;
        let dx_offset = dx * bpp;
        let width_bytes = width * bpp;

        let copy_row = |slice: &mut [u8], psy: usize, pdy: usize| {
            // Copy the bytes from the source row to the dest row, taking the x
            // offset and width into consideration
            let start = psy * pitch + sx_offset;
            let end = start + width_bytes;
            slice.copy_within(start .. end, pdy * pitch + dx_offset);
        };

        for index in 0..height {
            let py = if sy >= dy {
                // copy downward
                index
            } else {
                // copy upward
                height - 1 - index
            };

            copy_row(buf.slice(), sy + py, dy + py);
        }

        buf.dirty(&self.config, dx, dy, width, height);
    }
}
