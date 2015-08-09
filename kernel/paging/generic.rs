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

use core::iter::{repeat, StepBy};
use core::cell::RefCell;
use core::ops;

use error::Error;

use alloc::rc::Rc;

pub type Page<Paddr> = Option<(Paddr, PageType)>;

pub trait PhysicalAddress: Copy + Clone {
    /// Offset the physical address by the given amount.
    ///
    /// # Panics
    ///
    /// Only required to be able to offset within a single page. Any
    /// unacceptable offsets may panic.
    fn page_offset(self, amount: usize) -> Self;
}

impl<T> PhysicalAddress for T
    where T: Copy + Clone + ops::Add<usize, Output=T> {

    fn page_offset(self, amount: usize) -> Self {
        self + amount
    }
}

pub trait Pageset<'a>: Sized {
    type Paddr: PhysicalAddress;
    type Iter:  Iterator<Item=Option<(Self::Paddr, PageType)>>;
    type E:     Error;

    /// Create a new pageset.
    fn new() -> Self;

    /// Create a new kernel pageset.
    fn new_kernel() -> Self;

    /// Load the pageset's page tables into the appropriate control registers.
    ///
    /// # Safety
    ///
    /// This method does not take measures to ensure that the pageset outlives
    /// the duration of which the hardware control registers point to its page
    /// tables.
    ///
    /// Changing the page tables can result in system instability, data loss,
    /// and/or information leaks. Use with care. Doing so with this method may
    /// also conflict with `paging::current_pageset()`.
    unsafe fn load_into_hw(&'a mut self);

    fn page_size() -> usize;

    fn is_kernel_pageset(&self) -> bool;

    fn from(&'a self, vaddr: usize) -> Self::Iter;

    fn get(&'a self, vaddr: usize) -> Page<Self::Paddr> {
        self.from(vaddr).next().and_then(|x| x)
    }

    fn modify_while<F>(&'a mut self, vaddr: usize, callback: F)
                       -> Result<(), Self::E>
        where F: FnMut(Page<Self::Paddr>) -> Option<Page<Self::Paddr>>;

    fn modify<F>(&'a mut self, vaddr: usize, callback: F) -> Result<(), Self::E>
        where F: FnOnce(Page<Self::Paddr>) -> Page<Self::Paddr> {

        let mut callback = Some(callback);

        self.modify_while(vaddr, |page| callback.take().map(|c| c(page)))
    }
}

pub trait PagesetExt<'a>: Pageset<'a> {
    fn alloc() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }

    fn range(vaddr_start: usize, vaddr_end: usize)
             -> StepBy<usize, ops::Range<usize>> {
        (vaddr_start..vaddr_end).step_by(Self::page_size())
    }

    fn lookup(&'a self, vaddr: usize) -> Option<Self::Paddr> {
        self.get(vaddr).map(|(paddr, _)|
            paddr.page_offset(vaddr % Self::page_size()))
    }

    fn modify_pages<F>(&'a mut self,
                       vaddr: usize,
                       pages: usize,
                       mut callback: F)
                       -> Result<(), Self::E>
        where F: FnMut(Page<Self::Paddr>) -> Page<Self::Paddr> {

        let mut remaining = pages;

        self.modify_while(vaddr, |page| {
            if remaining > 0 {
                remaining -= 1;
                Some(callback(page))
            } else {
                None
            }
        })
    }

    fn set_pages<I>(&'a mut self, vaddr: usize, mut pages: I)
                    -> Result<(), Self::E>
        where I: Iterator<Item=Page<Self::Paddr>> {

        self.modify_while(vaddr, |_| pages.next())
    }

    fn set_page_types<I>(&'a mut self, vaddr: usize, mut page_types: I)
                        -> Result<(), Self::E>
        where I: Iterator<Item=PageType> {

        self.modify_while(vaddr, |page| {
            page_types.next().map(|page_type| {
                page.map(|(paddr, _)| (paddr, page_type))
            })
        })
    }

    fn set(&'a mut self, vaddr: usize, page: Page<Self::Paddr>)
           -> Result<(), Self::E> {

        self.modify(vaddr, |_| page)
    }

    fn set_page_type(&'a mut self, vaddr: usize, page_type: PageType)
                     -> Result<(), Self::E> {

        self.modify(vaddr, |page| page.map(|(paddr, _)| (paddr, page_type)))
    }

    fn map_pages<I>(&'a mut self, vaddr: usize, pages: I)
                    -> Result<(), Self::E>
        where I: Iterator<Item=(Self::Paddr, PageType)> {

        self.set_pages(vaddr, pages.map(|page| Some(page)))
    }

    fn map_pages_with_type<I>(&'a mut self,
                              vaddr:     usize,
                              paddrs:    I,
                              page_type: PageType)
                              -> Result<(), Self::E>
        where I: Iterator<Item=Self::Paddr> {

        self.map_pages(vaddr, paddrs.map(|paddr| (paddr, page_type)))
    }

    fn map<I>(&'a mut self,
              vaddr:     usize,
              paddr:     Self::Paddr,
              page_type: PageType)
              -> Result<(), Self::E> {

        self.set(vaddr, Some((paddr, page_type)))
    }

    fn unmap_pages(&'a mut self, vaddr: usize, pages: usize)
                   -> Result<(), Self::E> {

        self.set_pages(vaddr, repeat(None).take(pages))
    }

    fn unmap(&'a mut self, vaddr: usize) -> Result<(), Self::E> {
        self.set(vaddr, None)
    }
}

impl<'a, T: Pageset<'a>> PagesetExt<'a> for T { }

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct PageType(u8);

impl Default for PageType {
    fn default() -> PageType { PageType(0) }
}

impl PageType {
    fn set(self, bit: u8) -> PageType {
        let PageType(n) = self;
        PageType(n | (1 << bit))
    }

    fn clear(self, bit: u8) -> PageType {
        let PageType(n) = self;
        PageType(n & !(1 << bit))
    }

    fn is(self, bit: u8) -> bool {
        let PageType(n) = self;
        ((n >> bit) & 1) == 1
    }

    pub fn executable(self)     -> PageType { self.set(0) }
    pub fn writable(self)       -> PageType { self.set(1) }
    pub fn user(self)           -> PageType { self.set(2) }

    pub fn not_executable(self) -> PageType { self.clear(0) }
    pub fn not_writable(self)   -> PageType { self.clear(1) }
    pub fn not_user(self)       -> PageType { self.clear(2) }

    pub fn is_executable(self)  -> bool { self.is(0) }
    pub fn is_writable(self)    -> bool { self.is(1) }
    pub fn is_user(self)        -> bool { self.is(2) }
}
