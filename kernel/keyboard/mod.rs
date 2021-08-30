/*******************************************************************************
 *
 * kit/kernel/keyboard/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Generic keyboard input handler.

use displaydoc::Display;

use crate::error::Error;

pub mod ps2;

pub unsafe fn initialize() -> Result<(), KeyboardInitError> {
    ffi::keyboard_initialize();

    ps2::initialize();

    if !ps2::i8042::initialize() {
        return Err(KeyboardInitError::ControllerInitFailed("i8042"));
    }

    Ok(())
}

#[derive(Debug, Display)]
pub enum KeyboardInitError {
    /// Initialization of the '{0}' keyboard controller failed.
    ControllerInitFailed(&'static str),
}

impl Error for KeyboardInitError { }

/// C interface. See `kit/kernel/include/keyboard.h`.
pub mod ffi {
    extern {
        pub fn keyboard_initialize();
    }
}
