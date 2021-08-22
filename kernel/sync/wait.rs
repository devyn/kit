/*******************************************************************************
 *
 * kit/kernel/sync/wait.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use alloc::rc::Rc;
use alloc::collections::VecDeque;

use core::cell::RefCell;

use crate::process;
use crate::scheduler;

#[derive(Clone)]
pub struct WaitQueue {
    q: Rc<RefCell<VecDeque<process::Id>>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue { q: Rc::new(RefCell::new(VecDeque::new())) }
    }

    /// Awaken all processes waiting on this wait queue.
    pub fn awaken_all(&self) {
        for &pid in self.q.borrow().iter() {
            if let Some(process) = process::by_id(pid) {
                if process.borrow().is_alive() {
                    scheduler::awaken(process);
                }
            }
        }
    }

    /// Awaken a single process waiting on this wait queue.
    ///
    /// The process that we awaken will be moved to the end of the queue.
    ///
    /// Returns true if a process was awakened, otherwise false if the queue was
    /// empty.
    pub fn awaken_one(&self) -> bool {
        let mut q = self.q.borrow_mut();

        // This loop will discard dead processes
        while let Some(pid) = q.pop_front() {
            if let Some(process) = process::by_id(pid) {
                if scheduler::awaken(process).is_ok() {
                    // Since we woke this process up, it should be the last to
                    // wake up next time (assuming it doesn't remove itself from
                    // the queue before then)
                    q.push_back(pid);

                    // We were able to wake a process
                    return true;
                }
            }
        }

        // No processes waiting
        false
    }

    /// Insert a process to be awakened on this wait queue.
    ///
    /// The process should be in a suitable state to be woken up.
    pub fn insert(&self, pid: process::Id) {
        let mut q = self.q.borrow_mut();

        // Don't insert duplicates.
        if !q.iter().any(|&x| x == pid) {
            q.push_back(pid);
        }
    }

    /// Remove a process from the wait queue.
    pub fn remove(&self, pid: process::Id) -> bool {
        let mut q = self.q.borrow_mut();

        // There's a significant chance that the process requesting itself to be
        // removed is the process previously awakened by awaken_one, and
        // therefore at the back of the queue, so we should iterate in reverse
        let found = q.iter().enumerate().rev().find(|&(_, &x)| x == pid);

        if let Some((index, _)) = found {
            q.remove(index);
            true
        } else {
            false
        }
    }
}

#[macro_export]
macro_rules! wait {
    ($condition:expr, [$($queue:expr),+]) => {
        // Before doing anything, just test the condition once
        if !$condition {
            let current_pid = $crate::process::current().borrow().id;

            // Add us to the queues
            $(
                $queue.insert(current_pid);
            )+

            loop {
                $crate::process::sleep();
                if $condition { break; }
            }

            // Condition evaluated to true, remove us
            $(
                $queue.remove(current_pid);
            )+
        }
    }
}
