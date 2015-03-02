/*******************************************************************************
 *
 * kit/kernel/c_ffi.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! C FFI helpers.

use libc::c_char;
use core::prelude::*;
use core::mem;
use core::slice;
use core::marker::PhantomData;
use core::fmt::{self, Display, Debug};
use core::str;

/// Safe C strings.
pub struct CStr<'a> {
    ptr:    *const c_char,
    marker: PhantomData<&'a c_char>,
}

impl<'a> Copy for CStr<'a> { }

impl<'a> CStr<'a> {
    pub fn new(slice: &'a [u8]) -> CStr<'a> {
        if slice.len() > 0 && *slice.last().unwrap() == 0 {
            CStr {
                ptr: unsafe { mem::transmute(slice.as_ptr()) },
                marker: PhantomData
            }
        } else {
            panic!("CStr::new() called with non-zero-terminated byte slice!");
        }
    }

    pub unsafe fn from_ptr(ptr: *const c_char) -> CStr<'a> {
        CStr { ptr: ptr, marker: PhantomData }
    }

    pub fn as_ptr(&self) -> *const c_char {
        self.ptr
    }

    pub fn len(&self) -> usize {
       unsafe { 
            let mut str_len = 0usize;
            let mut str_end = self.ptr;

            while *str_end != 0 {
                str_len += 1;
                str_end  = str_end.offset(1);
            }

            str_len
       }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(mem::transmute(self.ptr), self.len())
        }
    }
}

impl<'a> Display for CStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match str::from_utf8(self.as_bytes()) {
            Ok(s)  => f.pad(s),
            Err(_) => Debug::fmt(self.as_bytes(), f),
        }
    }
}
