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

//! Reference-counted, automatically releasing boxes.

use core::prelude::*;
use core::ops::Deref;
use core::cmp::Ordering;
use core::fmt;

use memory::Box;

pub struct Rc<T>(*mut Contents<T>);

struct Contents<T> {
    refs: isize, // signed in order to be able to detect errors
    data: T,
}

impl<T> Rc<T> {
    /// Constructs a new reference-counted box.
    pub fn new(value: T) -> Rc<T> {
        unsafe { Rc(Box::new(Contents { refs: 1, data: value }).into_raw()) }
    }
}

impl<T> Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let Rc(ptr) = *self;

        unsafe { &(*ptr).data }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Rc<T> {
        let Rc(ptr) = *self;

        unsafe {
            assert!((*ptr).refs > 0);

            (*ptr).refs += 1;
        }

        Rc(ptr)
    }
}

#[unsafe_destructor]
impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let Rc(ptr) = *self;

        unsafe {
            if (*ptr).refs == 1 {
                drop(Box::from_raw(ptr)); // free the memory
            } else {
                assert!((*ptr).refs > 1);

                (*ptr).refs -= 1;
            }
        }
    }
}

impl<T, U> PartialEq<Rc<U>> for Rc<T> where T: PartialEq<U> {
    fn eq(&self, other: &Rc<U>) -> bool {
        PartialEq::eq(&**self, &**other)
    }

    fn ne(&self, other: &Rc<U>) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: Eq> Eq for Rc<T> { }

impl<T, U> PartialOrd<Rc<U>> for Rc<T> where T: PartialOrd<U> {
    fn partial_cmp(&self, other: &Rc<U>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }

    fn lt(&self, other: &Rc<U>) -> bool {
        PartialOrd::lt(&**self, &**other)
    }

    fn le(&self, other: &Rc<U>) -> bool {
        PartialOrd::le(&**self, &**other)
    }

    fn gt(&self, other: &Rc<U>) -> bool {
        PartialOrd::gt(&**self, &**other)
    }

    fn ge(&self, other: &Rc<U>) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
}

impl<T: Ord> Ord for Rc<T> {
    fn cmp(&self, other: &Rc<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
