/*******************************************************************************
 *
 * kit/kernel/sync/rcu.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use alloc::sync::Arc;

use core::sync::atomic::{
    AtomicPtr,
    Ordering::*,
    fence,
};

use core::fmt;

/// Read-copy-update.
pub struct Rcu<T> {
    state: AtomicPtr<T>,
}

impl<T> Rcu<T> {
    pub fn new(value: Arc<T>) -> Rcu<T> {
        fence(Release);

        Rcu { state: AtomicPtr::new(Arc::into_raw(value) as *mut T) }
    }

    pub fn read(&self) -> Arc<T> {
        unsafe {
            let ptr = self.state.load(Acquire);

            assert!(!ptr.is_null());

            Arc::increment_strong_count(ptr);
            Arc::from_raw(ptr)
        }
    }

    /// Write a new value without verifying the existing value.
    pub fn put(&self, value: Arc<T>) {
        unsafe {
            let old = self.state.swap(Arc::into_raw(value) as *mut T, AcqRel);
            drop(Arc::from_raw(old))
        }
    }

    pub fn update(
        &self,
        original: &Arc<T>,
        value: Arc<T>,
    ) -> Result<(), Arc<T>> {
        unsafe {
            let raw_ptr = Arc::into_raw(value);
            let original_ptr = Arc::as_ptr(original);

            self.state
                .compare_exchange(
                    original_ptr as *mut T,
                    raw_ptr as *mut T,
                    Release,
                    Relaxed,
                )
                .map(|_| ())
                // Give it back on error.
                .map_err(|_| Arc::from_raw(raw_ptr))
        }
    }

    pub fn update_with<F>(&self, mut mapper: F) -> Option<Arc<T>>
    where
        F: FnMut(&Arc<T>) -> Option<Arc<T>>,
    {
        loop {
            let original = self.read();
            if let Some(new) = mapper(&original) {
                if self.update(&original, new.clone()).is_ok() {
                    return Some(new);
                } else {
                    continue;
                }
            } else {
                return None;
            }
        } 
    }
}

impl<T> From<Arc<T>> for Rcu<T> {
    fn from(arc: Arc<T>) -> Rcu<T> {
        Rcu::new(arc)
    }
}

impl<T> From<T> for Rcu<T> {
    fn from(value: T) -> Rcu<T> {
        Rcu::new(Arc::new(value))
    }
}

impl<T: fmt::Debug> fmt::Debug for Rcu<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = self.read();
        write!(f, "Rcu({0:p} = {0:?})", value)
    }
}

impl<T> fmt::Pointer for Rcu<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = self.read();
        write!(f, "{:p}", value)
    }
}
