/*******************************************************************************
 *
 * kit/kernel/interrupt.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
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
pub unsafe fn enable() {
    asm!("sti");
}

/// Disable interrupts.
pub unsafe fn disable() {
    asm!("cli");
}

/// Wait for an interrupt.
pub unsafe fn wait() {
    asm!("sti; hlt; cli");
}

/// C interface. See `kit/kernel/include/interrupt.h`.
pub mod ffi {
    extern {
        pub fn interrupt_initialize();
    }
}
