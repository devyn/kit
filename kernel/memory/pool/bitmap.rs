/*******************************************************************************
 *
 * kit/kernel/memory/pool/bitmap.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Bitmap for tracking free space in the pool.
//!
//! Submodule to enforce unsafe constructor
use core::fmt;
use core::sync::atomic::Ordering::*;
use core::sync::atomic::*;

#[derive(Clone)]
pub struct FreeBitmap<'a> {
    bits: usize,
    slice: &'a [AtomicU8],
}

pub const fn byte_size(bits: usize) -> usize {
    if bits % 8 == 0 {
        bits / 8
    } else {
        bits / 8 + 1
    }
}

impl<'a> FreeBitmap<'a> {
    pub unsafe fn new(bits: usize, ptr: usize) -> FreeBitmap<'a> {
        FreeBitmap {
            bits,
            slice: core::slice::from_raw_parts(
                ptr as *const AtomicU8,
                byte_size(bits)
            ),
        }
    }

    pub fn clear(&self) {
        for (index, byte) in self.slice.iter().enumerate() {
            let desired_state =
                if self.bits % 8 != 0 && index == byte_size(self.bits) - 1 {
                    // Last byte should pad out of range bits to 1
                    0xFF << self.bits % 8
                } else {
                    0x00
                };

            byte.store(desired_state, Relaxed);
        }
    }

    /// Atomically finds the first free bit and sets it to used, then
    /// returns the index of the acquired bit.
    pub fn acquire_bit(&self) -> Option<usize> {
        for (index, byte) in self.slice.iter().enumerate() {
            let mut acquired_bit = None;

            // We don't care about success/failure because that's
            // actually captured by acquired_bit.
            //
            // We shouldn't need to order other memory - we are just
            // concerned with this byte - so Relaxed is appropriate.
            let _ = byte.fetch_update(Relaxed, Relaxed, |val| {
                for bit in 0..8 {
                    if val & (1 << bit) == 0 {
                        // We may have acquired it, but this can run
                        // again if we fail to CAS.
                        acquired_bit = Some(bit);
                        return Some(val | (1 << bit));
                    }
                }
                // Don't change.
                acquired_bit = None;
                None
            });

            if let Some(bit) = acquired_bit {
                return Some(index * 8 + bit);
            }
        }

        // None of the bytes had a free value
        None
    }

    /// Atomically sets the bit at the given index to free. Returns true if
    /// the bit was used, false if the bit was already free.
    pub fn release_bit(&self, index: usize) -> bool {
        let byte = {
            assert!(
                index < self.bits,
                "Index out of range: {} >= {}",
                index,
                self.bits
            );

            &self.slice[index / 8]
        };

        let bit = index % 8;

        // Use the fetch_update result - if a modification was made, we
        // freed the bit.
        byte.fetch_update(Relaxed, Relaxed, |val| {
            if val & (1 << bit) != 0 {
                // Clear the bit
                Some(val & !(1 << bit))
            } else {
                None
            }
        })
        .is_ok()
    }

    /// Checks if the region is empty. Only can be trusted if there are no
    /// other references to the region, as the whole operation is not
    /// atomic.
    pub fn is_empty(&self) -> bool {
        for (index, byte) in self.slice.iter().enumerate() {
            let desired_state =
                if self.bits % 8 != 0 && index == byte_size(self.bits) - 1 {
                    // Last byte should pad out of range bits to 1
                    0xFF << (8 - self.bits % 8)
                } else {
                    0x00
                };

            if byte.load(Relaxed) != desired_state {
                return false;
            }
        }

        true
    }

    /// Checks if the region is full. Only can be trusted if there are no
    /// other references to the region, as the whole operation is not
    /// atomic.
    pub fn is_full(&self) -> bool {
        self.slice.iter().all(|byte| byte.load(Relaxed) == 0xFF)
    }

    /// Counts the number of full bits. Non-atomic.
    #[allow(dead_code)]
    pub fn count_full(&self) -> usize {
        let mut count = 0;
        let mut total_bit = 0;

        for byte in self.slice {
            let value = byte.load(Relaxed);

            for bit in 0..8 {
                if value & (1 << bit) != 0 && total_bit < self.bits {
                    count += 1;
                }
                total_bit += 1;
            }
        }

        count
    }
}

impl<'a> fmt::Debug for FreeBitmap<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FreeBitmap({}, \"", self.bits)?;

        for byte in self.slice {
            write!(f, "{:02X}", byte.load(Relaxed))?;
        }

        write!(f, "\")")?;
        Ok(())
    }
}
