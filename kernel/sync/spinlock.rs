/*******************************************************************************
 *
 * kit/kernel/sync/spinlock.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::*;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::fmt;

pub struct Spinlock<T: ?Sized> {
    locked: AtomicBool,
    cell: UnsafeCell<T>,
}

impl<T> Spinlock<T> {
    pub const fn new(value: T) -> Spinlock<T> {
        Spinlock {
            locked: AtomicBool::new(false),
            cell: UnsafeCell::new(value)
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    /// Spin in a busy loop until we can get the lock.
    pub fn lock(&self) -> SpinlockGuard<T> {
        // Loop until we get the lock
        while !self.acquire() {
            core::hint::spin_loop();
        }

        SpinlockGuard { spinlock: self }
    }

    /// Try once to get the lock without spinning.
    pub fn try_lock(&self) -> Option<SpinlockGuard<T>> {
        if self.acquire() {
            Some(SpinlockGuard { spinlock: self })
        } else {
            None
        }
    }

    fn acquire(&self) -> bool {
        self.locked.compare_exchange(false, true, Acquire, Relaxed).is_ok()
    }

    fn release(&self) -> bool {
        self.locked.compare_exchange(true, false, Release, Relaxed).is_ok()
    }
}

unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Spinlock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Spinlock");
        match self.try_lock() {
            Some(guard) => {
                d.field("data", &&*guard);
            },
            None => {
                d.field("data", &"<locked>");
            }
        }
        d.finish_non_exhaustive()
    }
}

pub struct SpinlockGuard<'a, T: ?Sized> {
    spinlock: &'a Spinlock<T>,
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safe while the guard is held
        unsafe { &*self.spinlock.cell.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safe while the guard is held
        unsafe { &mut *self.spinlock.cell.get() }
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        assert!(self.spinlock.release(), "failed to release spinlock");
    }
}
