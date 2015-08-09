/*******************************************************************************
 *
 * kit/kernel/collections/treemap.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! A TreeMap implementation based on [AA
//! trees](http://en.wikipedia.org/wiki/AA_tree).

use core::mem::{swap, replace};
use core::cmp;
use core::cmp::Ordering::*;
use core::fmt;

use alloc::boxed::Box;

pub struct TreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
    len:  usize,
}

impl<K: Ord, V> TreeMap<K, V> {
    /// Create a new empty `TreeMap`.
    pub fn new() -> TreeMap<K, V> {
        TreeMap { root: None, len: 0 }
    }

    /// Get a reference to the value corresponding to the given key, if found.
    pub fn lookup<'a>(&'a self, key: &K) -> Option<&'a V> {
        lookup(&self.root, key)
    }

    /// Get a mutable reference to the value corresponding to the given key, if
    /// found.
    pub fn lookup_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        lookup_mut(&mut self.root, key)
    }

    /// Insert a key and value into the `TreeMap`.
    ///
    /// Replaces any pre-existing value and returns the original value, if
    /// present.
    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        let r = insert(&mut self.root, key, val);

        if r.is_none() {
            self.len += 1;
        }

        r
    }

    /// Delete a key from a `TreeMap`. If found, returns the removed value.
    pub fn delete(&mut self, key: &K) -> Option<V> {
        let r = delete(&mut self.root, key);

        if r.is_some() {
            self.len -= 1;
        }

        r
    }

    /// The number of nodes in the `TreeMap`.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<K, V> fmt::Debug for TreeMap<K, V>
        where K: fmt::Debug, V: fmt::Debug {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TreeMap {{ root: {:?} }}", self.root)
    }
}

// 1. The level of every leaf node is one.
// 2. The level of every left child is exactly one less than that of its parent.
// 3. The level of every right child is equal to or one less than that of its
//    parent. 
// 4. The level of every right grandchild is strictly less than that of its
//    grandparent.
// 5. Every node of level greater than one has two children.

struct Node<K, V> {
    key:   K,
    val:   V,
    level: usize,
    left:  Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, val: V) -> Node<K, V> {
        Node {
            key:   key,
            val:   val,
            level: 1,
            left:  None,
            right: None,
        }
    }
}

fn lookup<'a, K: Ord, V>(t: &'a Option<Box<Node<K, V>>>, key: &K)
          -> Option<&'a V> {

    t.as_ref().and_then(|node| match key.cmp(&node.key) {
        Less    => lookup(&node.left, key),
        Greater => lookup(&node.right, key),
        Equal   => Some(&node.val)
    })
}

fn lookup_mut<'a, K: Ord, V>(t: &'a mut Option<Box<Node<K, V>>>, key: &K)
              -> Option<&'a mut V> {

    t.as_mut().and_then(|node| match key.cmp(&node.key) {
        Less    => lookup_mut(&mut node.left, key),
        Greater => lookup_mut(&mut node.right, key),
        Equal   => Some(&mut node.val)
    })
}

/// Right rotation to replace a subtree contaning a left horizontal link with one
/// containing a right horizontal link instead.
fn skew<K, V>(t: &mut Box<Node<K, V>>) {
    if t.left.as_ref().map_or(false, |x| x.level == t.level) {
        // Swap the pointers of horizontal left links.
        let mut l = t.left.take().unwrap();
        swap(&mut t.left, &mut l.right);
        swap(t, &mut l);
        t.right = Some(l);
    }
}

/// Left rotation and level increase to replace a subtree containing two or more
/// consecutive right links with one containing two fewer consecutive right
/// horizontal links.
fn split<K, V>(t: &mut Box<Node<K, V>>) {
    if t.right.as_ref().map_or(false,
            |x| x.right.as_ref().map_or(false, |y| y.level == t.level)) {

        // We have two horizontal right links. Take the middle node, elevate it,
        // and return it.
        let mut r = t.right.take().unwrap();
        swap(&mut t.right, &mut r.left);
        swap(t, &mut r);
        t.level += 1;
        t.left = Some(r);
    }
}

fn insert<K: Ord, V>(t: &mut Option<Box<Node<K, V>>>, key: K, val: V)
          -> Option<V> {

    match *t {
        Some(ref mut node) => match key.cmp(&node.key) {
            Less => {
                let r = insert(&mut node.left, key, val);
                skew(node);
                split(node);
                r
            },

            Greater => {
                let r = insert(&mut node.right, key, val);
                skew(node);
                split(node);
                r
            },

            Equal => Some(replace(&mut node.val, val))
        },

        None => {
            *t = Some(box Node::new(key, val));
            None
        }
    }
}

fn take_closest<K, V>(node: &mut Node<K, V>) -> Option<Box<Node<K, V>>> {
    fn leftmost<K, V>(t: &mut Option<Box<Node<K, V>>>) -> Box<Node<K, V>> {
        if t.as_ref().unwrap().left.is_some() {
            leftmost(&mut t.as_mut().unwrap().left)
        } else {
            t.take().unwrap()
        }
    }

    fn rightmost<K, V>(t: &mut Option<Box<Node<K, V>>>) -> Box<Node<K, V>> {
        if t.as_ref().unwrap().right.is_some() {
            rightmost(&mut t.as_mut().unwrap().right)
        } else {
            t.take().unwrap()
        }
    }

    if node.left.is_some() {
        Some(rightmost(&mut node.left))
    } else if node.right.is_some() {
        Some(leftmost(&mut node.right))
    } else {
        None
    }
}

fn decrease_level<K, V>(node: &mut Node<K, V>) {
    fn level<K, V>(t: &Option<Box<Node<K, V>>>) -> usize {
        t.as_ref().map(|x| x.level).unwrap_or(0)
    }

    let should_be = cmp::min(level(&node.left), level(&node.right)) + 1;

    if should_be < node.level {
        node.level = should_be;

        node.right.as_mut().map(|r| {
            if should_be < r.level {
                r.level = should_be;
            }
        });
    }
}

fn delete<K: Ord, V>(t: &mut Option<Box<Node<K, V>>>, key: &K) -> Option<V> {
    t.take().and_then(|mut node| {
        let r = match key.cmp(&node.key) {
            Less    => delete(&mut node.left, key),
            Greater => delete(&mut node.right, key),
            Equal   => match take_closest(&mut node) {
                Some(box c) => {
                    node.key = c.key;
                    Some(replace(&mut node.val, c.val))
                },
                None => return Some(node.val) // Leaf
            }
        };

        // Rebalance the tree. Decrease the level of all nodes in this level if
        // necessary, and then skew and split all nodes in the new level.
        decrease_level(&mut node);
        skew(&mut node);
        node.right.as_mut().map(|right| {
            skew(right);
            right.right.as_mut().map(|r_right| skew(r_right));
        });
        split(&mut node);
        node.right.as_mut().map(|right| split(right));

        *t = Some(node);
        r
    })
}

impl<K, V> fmt::Debug for Node<K, V>
        where K: fmt::Debug, V: fmt::Debug {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Node {{ key: {:?}, val: {:?}, level: {:?}, \
                left: {:?}, right: {:?} }}",
               self.key, self.val, self.level, self.left, self.right)
    }
}
