/*******************************************************************************
 *
 * kit/kernel/kernel.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::fmt;

/// Command line parsing
#[derive(Debug)]
pub struct Cmdline<'a> {
    string: &'a str
}

impl fmt::Display for Cmdline<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl Cmdline<'_> {
    pub fn new(string: &str) -> Cmdline {
        Cmdline { string }
    }

    /// Parse the command line into an iterator of key-value pairs.
    pub fn iter(&self) -> Iter {
        Iter { split: self.string.split(' ') }
    }
}

pub struct Iter<'a> {
    split: core::str::Split<'a, char>
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        self.split.next().map(|next_item| {
            if let Some((key, value)) = next_item.split_once("=") {
                (key, value)
            } else {
                (next_item, "")
            }
        })
    }
}
