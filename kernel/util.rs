/*******************************************************************************
 *
 * kit/kernel/util.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
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
