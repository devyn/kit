/*******************************************************************************
 *
 * kit/kernel/memory/boxed.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Owned boxes.

use core::prelude::*;

use core::fmt;
use core::mem;
use core::ptr::{self, Unique};
use core::ops::{Deref, DerefMut};
use core::cmp::Ordering;
use core::intrinsics;

use libc::size_t;

use super::ffi;

/// Similar to Rust `std::boxed::Box`, using our kernel memory allocator
/// instead.
pub struct Box<T>(Unique<T>);

impl<T> Box<T> {
    /// Allocates memory on the heap and then moves `x` into it.
    pub fn new(x: T) -> Box<T> {
        Box::with_alignment(mem::align_of::<T>(), x)
    }

    /// Allocates memory on the heap aligned to the given alignment and then
    /// moves `x` into it.
    ///
    /// # Panics
    ///
    /// Panics if the given alignment is not divisible by the type's alignment.
    /// This ensures safety.
    pub fn with_alignment(alignment: usize, x: T) -> Box<T> {
        if alignment % mem::align_of::<T>() != 0 {
            panic!("invalid alignment for type ({} into {})",
                   mem::align_of::<T>(), alignment);
        }

        unsafe {
            let p = ffi::memory_alloc_aligned(mem::size_of::<T>() as size_t,
                                              alignment as size_t);

            let p: *mut T = mem::transmute(p);

            ptr::write(p.as_mut().expect("out of memory"), x);

            Box(Unique::new(p))
        }
    }

    /// Allocates zeroed memory on the heap aligned to the given alignment.
    ///
    /// # Safety
    ///
    /// The box must be properly initialized before returning to safe code, if
    /// zeroes are not an appropriate representation for the type.
    ///
    /// In particular, the box must be properly initialized such that it is safe
    /// to run any applicable destructors for the type.
    ///
    /// # Panics
    ///
    /// Panics if the given alignment is not divisible by the type's alignment.
    pub unsafe fn zeroed_with_alignment(alignment: usize) -> Box<T> {
        if alignment % mem::align_of::<T>() != 0 {
            panic!("invalid alignment for type ({} into {})",
                   mem::align_of::<T>(), alignment);
        }

        let p = ffi::memory_alloc_aligned(mem::size_of::<T>() as size_t,
                                          alignment as size_t);

        let p: *mut T = mem::transmute(p);

        intrinsics::set_memory(p, 0, 1);

        Box(Unique::new(p))
    }

    /// Consumes the `Box` and returns the stored value.
    pub fn unwrap(self) -> T {
        unsafe {
            let ptr: *mut T = { let Box(ref u) = self; **u };
            let x = ptr::read(ptr);

            ffi::memory_free(mem::transmute(ptr));
            mem::forget(self);
            x
        }
    }

    /// Creates a `Box` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The only safe way to use this function is to pass a pointer that was
    /// previously returned by `Box::into_raw()`. Anything else is unsafe.
    pub unsafe fn from_raw(ptr: *mut T) -> Box<T> {
        Box(Unique::new(ptr))
    }

    /// Consumes the `Box`, returning the raw pointer.
    ///
    /// # Safety
    ///
    /// Box is no longer managed and may be leaked. Use `Box::from_raw()` to
    /// release.
    pub unsafe fn into_raw(self) -> *mut T {
        let ptr: *mut T = { let Box(ref u) = self; **u };
        mem::forget(self);
        ptr
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let &Box(ref u) = self;

        unsafe { u.get() }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        let &mut Box(ref mut u) = self;

        unsafe { u.get_mut() }
    }
}

#[unsafe_destructor]
impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let ptr: *mut T = &mut **self;

            // XXX HACK, ew.
            ((*intrinsics::get_tydesc::<T>()).drop_glue)(mem::transmute(ptr));

            ffi::memory_free(mem::transmute(ptr));
        }
    }
}

impl<T, U> PartialEq<Box<U>> for Box<T> where T: PartialEq<U> {
    fn eq(&self, other: &Box<U>) -> bool {
        PartialEq::eq(&**self, &**other)
    }

    fn ne(&self, other: &Box<U>) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: Eq> Eq for Box<T> { }

impl<T, U> PartialOrd<Box<U>> for Box<T> where T: PartialOrd<U> {
    fn partial_cmp(&self, other: &Box<U>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }

    fn lt(&self, other: &Box<U>) -> bool {
        PartialOrd::lt(&**self, &**other)
    }

    fn le(&self, other: &Box<U>) -> bool {
        PartialOrd::le(&**self, &**other)
    }

    fn gt(&self, other: &Box<U>) -> bool {
        PartialOrd::gt(&**self, &**other)
    }

    fn ge(&self, other: &Box<U>) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
}

impl<T: Ord> Ord for Box<T> {
    fn cmp(&self, other: &Box<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: fmt::Display> fmt::Display for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
