/*******************************************************************************
 *
 * kit/kernel/constants.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Target constants and related utility functions.

/// The offset of the identity map from zero, which contains the initial kernel
/// image.
pub const KERNEL_OFFSET: usize = 0xffffffff80000000;

/// Start of the initial ('low') identity mapped region.
pub const KERNEL_LOW_START: u32 = 0x00000000;

/// End of the initial ('low') identity mapped region.
pub const KERNEL_LOW_END:   u32 = 0x04000000;

/// Get a usable constant pointer in kernel space from a low address.
///
/// Returns `Some(ptr)` only if `addr` is in the range
/// `KERNEL_LOW_START..KERNEL_LOW_END`, which is the initial identity mapped
/// region.
pub unsafe fn translate_low_addr<T>(addr: u32) -> Option<*const T> {
    if addr >= KERNEL_LOW_START && addr < KERNEL_LOW_END {
        Some((KERNEL_OFFSET + (addr as usize)) as *const T)
    } else {
        None
    }
}

/// Get a usable mutable pointer in kernel space from a low address.
///
/// Returns `Some(ptr)` only if `addr` is in the range
/// `KERNEL_LOW_START..KERNEL_LOW_END`, which is the initial identity mapped
/// region.
pub unsafe fn translate_low_addr_mut<T>(addr: u32) -> Option<*mut T> {
    if addr >= KERNEL_LOW_START && addr < KERNEL_LOW_END {
        Some((KERNEL_OFFSET + (addr as usize)) as *mut T)
    } else {
        None
    }
}
