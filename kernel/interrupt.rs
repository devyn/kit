/*******************************************************************************
 *
 * kit/kernel/interrupt.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! High level interface to processor interrupts.

/// Prepare the interrupt table and load it.
pub unsafe fn initialize() {
    ffi::interrupt_initialize()
}

/// Enable interrupts.
#[inline]
pub unsafe fn enable() {
    asm!("sti");
}

/// Disable interrupts.
#[inline]
pub unsafe fn disable() {
    asm!("cli");
}

/// Wait for an interrupt.
#[inline]
pub unsafe fn wait() {
    asm!("sti; hlt; cli");
}

/// Briefly enable interrupts to allow a pending interrupt to be serviced.
#[inline]
pub unsafe fn accept() {
    enable();
    for _ in 0..10 { 
        core::hint::spin_loop();
    }
    disable();
}

/// C interface. See `kit/kernel/include/interrupt.h`.
pub mod ffi {
    extern {
        pub fn interrupt_initialize();
    }
}
