/*******************************************************************************
 *
 * kit/kernel/util.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Assorted utilities.

use core::cmp::min;
use core::intrinsics::{write_bytes, copy_nonoverlapping};

pub fn set_memory(dest: &mut [u8], value: u8) {
    unsafe {
        write_bytes(dest.as_mut_ptr(), value, dest.len());
    }
}

pub fn copy_memory(src: &[u8], dest: &mut [u8]) {
    unsafe {
        copy_nonoverlapping(
            src.as_ptr(), dest.as_mut_ptr(), min(src.len(), dest.len()));
    }
}

pub fn zero_memory(buf: &mut [u8]) {
    set_memory(buf, 0);
}

#[inline(always)]
pub const fn align_up(number: usize, align: usize) -> usize {
    if align & (align - 1) == 0 {
        // power of two
        (number + (align - 1)) & !(align - 1)
    } else {
        if number % align != 0 {
            number + align - (number % align)
        } else {
            number
        }
    }
}

#[test]
fn align_up_power_of_two() {
    assert_eq!(align_up(990, 8), 992);
}

#[test]
fn align_up_large_power_of_two() {
    assert_eq!(align_up(2990, 1024), 3072);
}

#[test]
fn align_up_non_power_of_two() {
    assert_eq!(align_up(1003, 5), 1005);
}

#[inline(always)]
pub const fn align_down(number: usize, align: usize) -> usize {
    if align & (align - 1) == 0 {
        // power of two
        number & !(align - 1)
    } else {
        number - (number % align)
    }
}

#[test]
fn align_down_power_of_two() {
    assert_eq!(align_down(990, 8), 984);
}

#[test]
fn align_down_large_power_of_two() {
    assert_eq!(align_down(2990, 1024), 2048);
}

#[test]
fn align_down_non_power_of_two() {
    assert_eq!(align_down(1003, 5), 1000);
}
