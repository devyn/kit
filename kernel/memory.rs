/*******************************************************************************
 *
 * kit/kernel/memory.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Kernel memory management.

use core::prelude::*;

use core::fmt;
use core::mem;
use core::ptr::{self, Unique};
use core::ops::{Deref, DerefMut};
use core::cmp::Ordering;

use libc::size_t;

/// Loads the memory map information into the region tree in order to know where
/// in physical memory it's safe to allocate fresh pages.
pub unsafe fn initialize(mmap_buffer: *const u8, mmap_length: u32) {
    ffi::memory_initialize(mmap_buffer, mmap_length)
}

/// Similar to Rust std::boxed::Box, using our kernel memory allocator instead.
pub struct Box<T>(Unique<T>);

impl<T> Box<T> {
    /// Allocates memory on the heap and then moves `x` into it.
    pub fn new(x: T) -> Box<T> {
        unsafe {
            let p = ffi::memory_alloc_aligned(mem::size_of::<T>() as size_t,
                                              mem::align_of::<T>() as size_t);

            let p: *mut T = mem::transmute(p);

            ptr::write(p.as_mut().expect("out of memory"), x);

            Box(Unique::new(p))
        }
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

#[unsafe_destructor]
impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let ptr: *mut T = &mut **self;

            ptr::read(ptr); // Drop

            ffi::memory_free(mem::transmute(ptr));
        }
    }
}

/// C interface. See `kit/kernel/include/memory.h`.
pub mod ffi {
    use libc::{size_t, c_void};

    extern {
        pub fn memory_initialize(mmap_buffer: *const u8, mmap_length: u32);

        pub fn memory_alloc_aligned(size: size_t, alignment: size_t)
                                    -> *mut c_void;

        pub fn memory_free(pointer: *mut c_void);
    }
}
