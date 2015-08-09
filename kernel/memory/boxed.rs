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

use core::fmt;
use core::mem;
use core::slice;
use core::slice::bytes::MutableByteVector;
use core::ptr::{self, Unique};
use core::ops::{Deref, DerefMut};
use core::cmp::Ordering;

/// Similar to Rust `std::boxed::Box`, using our kernel memory allocator
/// instead.
#[lang = "owned_box"]
pub struct Box<T>(Unique<T>);

impl<T> Box<T> {
    /// Allocates memory on the heap and then moves `x` into it.
    pub fn new(x: T) -> Box<T> {
        box x
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
            let p = super::allocate(mem::size_of::<T>(), alignment) as *mut T;

            ptr::write(&mut *p, x);

            mem::transmute(p)
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

        let p = super::allocate(mem::size_of::<T>(), alignment);

        slice::from_raw_parts_mut(p, mem::size_of::<T>()).set_memory(0);

        mem::transmute(p)
    }

    /// Consumes the `Box` and returns the stored value.
    pub fn unwrap(self) -> T {
        *self
    }

    /// Creates a `Box` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The only safe way to use this function is to pass a pointer that was
    /// previously returned by `Box::into_raw()`. Anything else is unsafe.
    pub unsafe fn from_raw(ptr: *mut T) -> Box<T> {
        mem::transmute(ptr)
    }

    /// Consumes the `Box`, returning the raw pointer.
    ///
    /// # Safety
    ///
    /// Box is no longer managed and may be leaked. Use `Box::from_raw()` to
    /// release.
    pub unsafe fn into_raw(self) -> *mut T {
        mem::transmute(self)
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &**self
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut **self
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
