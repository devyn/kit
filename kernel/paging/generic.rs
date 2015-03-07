/*******************************************************************************
 *
 * kit/kernel/paging/generic.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Page management functions and traits applicable to all targets.

use core::prelude::*;

pub trait Pageset: PageView {
    /// Create a new pageset.
    fn new() -> Self;

    /// Create a new kernel pageset.
    fn new_kernel() -> Self;

    /// Set the current system pageset to this one.
    ///
    /// # Unsafety
    ///
    /// Changing the pageset can result in system instability, data loss,
    /// and/or information leaks. Use with care.
    unsafe fn load(&mut self);
}

pub trait PageView {
    type P:  PresentCursor;
    type Pm: PresentCursorMut<Nm=Self::Nm>;
    type Nm: NotPresentCursorMut<Pm=Self::Pm>;

    type I:  Iterator<Item=Cursor<Self::P, ()>>;
    type Im: Iterator<Item=Cursor<Self::Pm, Self::Nm>>;

    /// The size in bytes of one page in this view.
    fn page_size() -> usize;

    /// Get a cursor to the page that maps the given virtual address.
    fn get(&self, vaddr: usize) -> Cursor<Self::P, ()>;

    /// Get a mutable cursor to the page that maps the given virtual address.
    fn get_mut(&mut self, vaddr: usize) -> Cursor<Self::Pm, Self::Nm>;

    /// Get an iterator that starts at the page that maps the given virtual
    /// address.
    fn iter(&self, from_vaddr: usize) -> Self::I;

    /// Get a mutable iterator that starts at the page that maps the given
    /// virtual address.
    fn iter_mut(&mut self, from_vaddr: usize) -> Self::Im;
}

pub trait PagesetExt {
    /// Look up a virtual address to get a physical address.
    fn lookup(&self, vaddr: usize) -> Option<usize>;
}

impl<T: Pageset> PagesetExt for T {
    fn lookup(&self, vaddr: usize) -> Option<usize> {
        self.get(vaddr).present().map(|p| p.paddr() + vaddr % T::page_size())
    }
}

pub enum Cursor<P, N> {
    Present(P),
    NotPresent(N),
    Reserved
}

impl<P, N> Cursor<P, N> {
    fn present(self) -> Option<P> {
        match self {
            Cursor::Present(p) => Some(p),
            _                  => None
        }
    }

    fn not_present(self) -> Option<N> {
        match self {
            Cursor::NotPresent(n) => Some(n),
            _                     => None
        }
    }

    fn is_present(&self) -> bool {
        match *self {
            Cursor::Present(_) => true,
            _                  => false
        }
    }

    fn is_not_present(&self) -> bool {
        match *self {
            Cursor::NotPresent(_) => true,
            _                     => false
        }
    }

    fn is_reserved(&self) -> bool {
        match *self {
            Cursor::Reserved => true,
            _                => false
        }
    }
}

pub trait PresentCursor {
    fn paddr(&self) -> usize;
}

pub trait PresentCursorMut: PresentCursor {
    type Nm: NotPresentCursorMut;

    fn unmap(self) -> Self::Nm;
}

pub trait NotPresentCursorMut {
    type Pm: PresentCursorMut;

    fn map(self) -> Self::Pm;
}
