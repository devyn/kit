/*******************************************************************************
 *
 * kit/kernel/error.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::fmt::{Debug, Display};

/// Mostly compatible with Rust's `std::error::Error`.
pub trait Error: Debug + Display {
    /// A short description of the error.
    ///
    /// The description should not contain newlines or sentence-ending
    /// punctuation, to facilitate embedding in larger user-facing strings.
    fn description(&self) -> &str;

    /// The lower-level cause of this error, if any.
    fn cause(&self) -> Option<&dyn Error> { None }
}
