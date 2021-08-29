/*******************************************************************************
 *
 * kit/kernel/sync/lock_free_list.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::sync::atomic::*;
use core::sync::atomic::Ordering::*;

use core::ptr;
use core::mem;
use core::mem::MaybeUninit;
use core::ops::Deref;

use alloc::sync::Arc;

#[derive(Debug)]
pub struct LockFreeList<T> {
    head: AtomicPtr<InnerNode<T>>, // pointer is Arc-wrapped, may be null
}

#[derive(Debug)]
struct InnerNode<T> {
    tail: AtomicPtr<InnerNode<T>>, // pointer is Arc-wrapped, may be null
    value: T
}

impl<T> InnerNode<T> {
    fn into_value(self) -> T {
        unsafe {
            if let Some(p) = ptr::NonNull::new(self.tail.load(SeqCst)) {
                // If tail was set, decrease the reference count.
                Arc::decrement_strong_count(p.as_ptr());
            }

            // Copy the value out to the stack
            let mut out = MaybeUninit::uninit();

            ptr::copy_nonoverlapping(&self.value, out.as_mut_ptr(), 1);

            // Don't run destructor (everything's been moved / handled another
            // way)
            mem::forget(self);

            // Return value copied onto stack
            out.assume_init()
        }
    }
}

impl<T> Drop for InnerNode<T> {
    fn drop(&mut self) {
        if let Some(p) = ptr::NonNull::new(self.tail.load(SeqCst)) {
            // If tail was set, decrease the reference count and clear it.
            unsafe { Arc::decrement_strong_count(p.as_ptr()); }
            self.tail.store(ptr::null_mut(), SeqCst);
        }
    }
}

#[derive(Debug)]
pub struct Node<T> {
    arc: Arc<InnerNode<T>>,
}

impl<T> Node<T> {
    pub fn new(value: T) -> Node<T> {
        Node {
            arc: Arc::new(InnerNode {
                tail: AtomicPtr::new(ptr::null_mut()),
                value,
            }),
        }
    }

    pub fn try_unwrap(this: Node<T>) -> Result<T, Node<T>> {
        Arc::try_unwrap(this.arc)
            .map(|inner_node| inner_node.into_value())
            .map_err(|arc| Node { arc })
    }

    pub fn tail(this: &Node<T>) -> Option<Node<T>> {
        ptr::NonNull::new(this.arc.tail.load(Relaxed))
            .map(|ptr| unsafe {
                Arc::increment_strong_count(ptr.as_ptr());
                Node { arc: Arc::from_raw(ptr.as_ptr()) }
            })
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &T { &self.arc.value }
}

impl<T> LockFreeList<T> {
    pub fn new() -> LockFreeList<T> {
        LockFreeList { head: AtomicPtr::new(ptr::null_mut()) }
    }

    pub fn push(&self, node: Node<T>) {
        let raw_arc = Arc::into_raw(node.arc);

        // Release semantics because we probably want to make sure anything done
        // to the Node makes it to the list
        //
        // Arc reference becomes owned by the list
        self.head.fetch_update(Release, Relaxed, |ptr| {
            unsafe {
                (*raw_arc).tail.store(ptr, Relaxed);
                Some(raw_arc as *mut InnerNode<T>)
            }
        }).expect("fetch_update that always returns Some didn't.");
    }

    pub fn pop(&self) -> Option<Node<T>> {
        pop_if(&self.head, |_| true)
    }

    pub fn head(&self) -> Option<Node<T>> {
        ptr::NonNull::new(self.head.load(Relaxed))
            .map(|ptr| unsafe {
                Arc::increment_strong_count(ptr.as_ptr());
                Node { arc: Arc::from_raw(ptr.as_ptr()) }
            })
    }

    pub fn remove(&self, target: &Node<T>) -> bool {
        let mut pred = |node: &Node<T>| Arc::ptr_eq(&node.arc, &target.arc);

        // If the head node matches, remove that
        if let Some(_) = pop_if(&self.head, &mut pred) {
            return true;
        } else {
            // Step through the iterator and try pop off each
            for node in self.iter() {
                if let Some(_) = pop_if(&node.arc.tail, &mut pred) {
                    return true;
                }
            }

            false
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { next: self.head() }
    }
}

pub struct Iter<T> {
    next: Option<Node<T>>,
}

impl<T> Iterator for Iter<T> {
    type Item = Node<T>;

    fn next(&mut self) -> Option<Node<T>> {
        let after = self.next.as_ref().and_then(|n| Node::tail(n));

        mem::replace(&mut self.next, after)
    }
}

fn pop_if<T, F>(head: &AtomicPtr<InnerNode<T>>, mut pred: F) 
    -> Option<Node<T>>
    where F: FnMut(&Node<T>) -> bool {

    let mut out;

    // Acquire semantics because the popped node may be modified after taking it
    loop {
        out = ptr::NonNull::new(head.load(Acquire));

        // Set the previous node to the next node of the node we're taking, but
        // don't update a null.
        if let Some(taken) = out {
            // Take a reference to taken while we work on it
            let taken_ref = unsafe {
                Arc::increment_strong_count(taken.as_ptr());
                Node { arc: Arc::from_raw(taken.as_ptr()) }
            };

            // Return early if the predicate doesn't match
            if !pred(&taken_ref) {
                return None;
            }

            let tail = taken_ref.arc.tail.load(Relaxed);

            // Add a reference to tail, because the node we pop will still
            // have a reference to it.
            if let Some(tail_p) = ptr::NonNull::new(tail) {
                unsafe { Arc::increment_strong_count(tail_p.as_ptr()); }
            }

            // Try to compare_exchange head -> tail
            let cas_res = head.compare_exchange(
                taken.as_ptr(), tail, Acquire, Relaxed);

            if cas_res.is_ok() {
                break;
            } else {
                // Need to clean up the extra reference we added to tail
                if let Some(tail_p) = ptr::NonNull::new(tail) {
                    unsafe { Arc::decrement_strong_count(tail_p.as_ptr()); }
                }
            }
        } else {
            break;
        }
    }

    // Convert the output to Node.
    out.map(|taken| Node { arc: unsafe { Arc::from_raw(taken.as_ptr()) } })
}
