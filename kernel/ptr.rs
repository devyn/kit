/*******************************************************************************
 *
 * kit/kernel/ptr.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Safe pointer manipulation.

use core::mem::MaybeUninit;
use core::slice;

use crate::paging::{current_pageset, GenericPageset, PAGE_SIZE};
use crate::c_ffi::CStr;

use alloc::vec::Vec;

use displaydoc::Display;

/// Assert that dereferencing this type from a valid user pointer can not cause
/// undefined behavior.
pub unsafe trait AlwaysUserSafe { }

unsafe impl AlwaysUserSafe for u8 { }
unsafe impl AlwaysUserSafe for u16 { }
unsafe impl AlwaysUserSafe for u32 { }
unsafe impl AlwaysUserSafe for u64 { }
unsafe impl AlwaysUserSafe for u128 { }
unsafe impl AlwaysUserSafe for usize { }
unsafe impl AlwaysUserSafe for i8 { }
unsafe impl AlwaysUserSafe for i16 { }
unsafe impl AlwaysUserSafe for i32 { }
unsafe impl AlwaysUserSafe for i64 { }
unsafe impl AlwaysUserSafe for i128 { }
unsafe impl AlwaysUserSafe for isize { }

// A user safe pointer is user safe.
unsafe impl<T> AlwaysUserSafe for UserPtr<T> where T: UserSafe { }

/// Assert that dereferencing this type from a valid user pointer can not cause
/// undefined behavior if and only if `is_user_safe()` returns true for the
/// pointer.
pub unsafe trait UserSafe {
    unsafe fn is_user_safe(valid_ptr: *const Self) -> bool;
}

unsafe impl<T> UserSafe for T where T: AlwaysUserSafe {
    #[inline]
    unsafe fn is_user_safe(_valid_ptr: *const Self) -> bool {
        true
    }
}

unsafe impl UserSafe for bool {
    #[inline]
    unsafe fn is_user_safe(valid_ptr: *const bool) -> bool {
        // zero and one are valid values
        *valid_ptr.cast::<u8>() < 2
    }
}

#[derive(Debug, Display, Clone)]
pub enum Error {
    /// Encountered an inaccessible page while reading from a user pointer
    InaccessiblePage,
    /// Encountered invalid data while reading from a user pointer
    UnsafeData,
    /// The destination buffer is too small to hold the user-provided data
    BufferTooSmall,
}

impl crate::error::Error for Error { }

/// A C-compatible pointer type with safe accessors that validate data before
/// returning anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct UserPtr<T>(*mut T);

impl<T: UserSafe + Copy> UserPtr<T> {
    /// A maximally safe option for reading user data.
    ///
    /// The value will be put on the stack, so this may be unsuitable for
    /// particularly large structures.
    pub fn read(self) -> Result<T, Error> {
        let mut value: MaybeUninit<T> = MaybeUninit::uninit();

        self.read_into(&mut value)?;

        // SAFETY: If read_into returns Ok, this is initialized.
        unsafe { Ok(value.assume_init()) }
    }

    /// A maximally safe option for reading user data.
    ///
    /// The reference returned in the `Ok` case is guaranteed to have been
    /// initialized, and is the same as the reference passed in `out`, just
    /// appropriately assumed initialized. It is also safe to assume that the
    /// `out` reference itself has been initialized if this function returns
    /// `Ok`.
    pub fn read_into(self, out: &mut MaybeUninit<T>) -> Result<&mut T, Error> {
        self.read_into_slice(slice::from_mut(out))
            .map(|mut_ref| &mut mut_ref[0])
    }

    /// Read a slice of user data safely.
    ///
    /// Each page will be checked for user accessibility before reading from it.
    ///
    /// Each object will be checked for validity (according to
    /// [UserSafe::is_user_safe]) during the read operation.
    ///
    /// The reference returned in the `Ok` case is guaranteed to have been
    /// initialized, and is the same as the slice passed in `out`, just
    /// appropriately assumed initialized. It is also safe to assume that the
    /// `out` slice itself has been initialized if this function returns `Ok`.
    ///
    /// If the function returns `Err`, the contents of the slice must be assumed
    /// uninitialized.
    pub fn read_into_slice(self, out: &mut [MaybeUninit<T>])
        -> Result<&mut [T], Error> {

        // Easy out if slice is empty.
        if out.is_empty() {
            // SAFETY: slice is empty
            unsafe {
                return Ok(MaybeUninit::slice_assume_init_mut(out));
            }
        }

        // Ensure pages can be accessed.
        let min_vaddr = self.0 as usize;
        let max_vaddr = self.0.wrapping_offset(out.len() as isize) as usize;

        assert!(max_vaddr >= min_vaddr);

        let min_vaddr_page = min_vaddr & !(PAGE_SIZE - 1);
        let max_vaddr_page = max_vaddr & !(PAGE_SIZE - 1);
        let expected_pages = (max_vaddr_page - min_vaddr_page) / PAGE_SIZE;

        let mut found_pages = 0;

        // SAFETY: we are only reading the pageset, this is always ok
        let pageset_ref = unsafe {
            current_pageset().expect("paging not initialized")
        };
        let pageset = pageset_ref.lock();

        // Walk pageset, ensure all mapped and accessible by user
        for page in pageset.from(min_vaddr_page).take(expected_pages) {
            if let Some((_, page_type)) = page {
                if page_type.is_user() {
                    found_pages += 1;
                } else {
                    return Err(Error::InaccessiblePage);
                }
            } else {
                return Err(Error::InaccessiblePage);
            }
        }

        if found_pages != expected_pages {
            return Err(Error::InaccessiblePage);
        }

        // Copy is safe now.
        unsafe {
            out.as_mut_ptr().cast::<T>().copy_from(self.0, out.len());
        }

        // Check each element
        for element in out.iter() {
            // SAFETY: bounds checked by iterator
            if !unsafe { UserSafe::is_user_safe(element.as_ptr()) } {
                return Err(Error::UnsafeData);
            }
        }

        // We had to hold the pageset lock until now to make sure it doesn't get
        // modified.
        drop(pageset);

        // All ok. Return the reference
        //
        // SAFETY: we initialized all of the elements and they're safe
        unsafe {
            Ok(MaybeUninit::slice_assume_init_mut(out))
        }
    }

    pub fn read_to_vec(self, count: usize) -> Result<Vec<T>, Error> {
        let mut buffer = Vec::with_capacity(count);

        let len = self.read_into_slice(buffer.spare_capacity_mut())?.len();

        // read_into_slice guarantees that portion was initialized
        unsafe {
            buffer.set_len(len);
        }

        Ok(buffer)
    }
}

impl UserPtr<u8> {
    /// Read a C string from user memory safely.
    pub fn read_c_string(self, buffer: &mut [u8]) -> Result<CStr, Error> {

        debug!("read_c_string({:?}, {:p} x {})", self, buffer, buffer.len());

        let min_vaddr = self.0 as usize;
        let min_vaddr_page = min_vaddr & !(PAGE_SIZE - 1);

        let mut size = 0;

        // SAFETY: reading the current pageset is always safe
        let pageset_ref = unsafe {
            current_pageset().expect("paging not initialized")
        };
        let pageset = pageset_ref.lock();

        let mut iter = pageset.from(min_vaddr_page);

        let mut page_base = min_vaddr_page;

        while let Some(Some((_, page_type))) = iter.next() {
            // Check that page is accessible.
            if page_type.is_user() {
                for vaddr in min_vaddr + size .. page_base + PAGE_SIZE {
                    if buffer.len() <= size {
                        return Err(Error::BufferTooSmall);
                    }

                    // SAFETY: we validated that the page is mapped, u8 does not
                    // have invalid data or alignment restrictions, and we hold
                    // the exclusive lock on the pageset so it shouldn't change
                    // from under us.
                    let byte = unsafe {
                        *(vaddr as *const u8)
                    };

                    buffer[size] = byte;

                    size += 1;

                    if byte == 0 {
                        // End of C string
                        return Ok(CStr::new(&buffer[0..size]));
                    }
                }
            } else {
                return Err(Error::InaccessiblePage);
            }

            page_base += PAGE_SIZE;
        }

        // Fall-through to inaccessible page error.
        Err(Error::InaccessiblePage)
    }
}
