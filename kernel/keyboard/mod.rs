/*******************************************************************************
 *
 * kit/kernel/keyboard/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Generic keyboard input handler.

use core::prelude::*;
use core::error::Error;
use core::fmt;

pub mod ps2;

pub unsafe fn initialize() -> Result<(), KeyboardInitError> {
    ffi::keyboard_initialize();

    ps2::initialize();

    if !ps2::i8042::initialize() {
        return Err(KeyboardInitError::ControllerInitFailed("i8042"));
    }

    Ok(())
}

#[derive(Debug)]
pub enum KeyboardInitError {
    /// Initialization of a keyboard controller failed.
    ///
    /// The string is the name of the relevant keyboard controller.
    ControllerInitFailed(&'static str),
}

impl Error for KeyboardInitError {
    fn description(&self) -> &str {
        match *self {
            KeyboardInitError::ControllerInitFailed(_) =>
                "Initialization of a keyboard controller failed."
        }
    }
}

impl fmt::Display for KeyboardInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            KeyboardInitError::ControllerInitFailed(name) =>
               write!(f,
                      "Initialization of the '{}' keyboard controller failed.",
                      name ),
        }
    }
}

/// C interface. See `kit/kernel/include/keyboard.h`.
pub mod ffi {
    extern {
        pub fn keyboard_initialize();
    }
}
