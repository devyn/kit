/*******************************************************************************
 *
 * kit/kernel/c_ffi.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! C FFI helpers.

#![allow(non_camel_case_types)]

use core::mem;
use core::slice;
use core::marker::PhantomData;
use core::fmt::{self, Display, Debug};
use core::str;

use alloc::string::String;

pub type size_t = usize;

pub type c_char  = i8;
pub type c_short = i16;
pub type c_int   = i32;
pub type c_long  = i64;

pub type uint8_t  = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type  int8_t  = i8;
pub type  int16_t = i16;
pub type  int32_t = i32;
pub type  int64_t = i64;

#[repr(u8)]
pub enum c_void {
    __variant1,
    __variant2,
}

/// Safe C strings.
pub struct CStr<'a> {
    ptr:    *const c_char,
    marker: PhantomData<&'a c_char>,
}

impl<'a> Clone for CStr<'a> {
    fn clone(&self) -> CStr<'a> {
        CStr {
            ptr: self.ptr,
            marker: PhantomData
        }
    }
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

/// Warning: lossy implementation
impl<'a> Into<String> for CStr<'a> {
    fn into(self) -> String {
        String::from_utf8_lossy(self.as_bytes()).into_owned()
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
