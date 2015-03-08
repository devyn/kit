/*******************************************************************************
 *
 * kit/kernel/paging/x86_64.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! x86-64 architecture-specific page tables.

use core::prelude::*;
use core::ops::Range;
use core::num::Int;
use core::default::Default;
use core::mem;
use core::fmt;
use core::error;
use core::ptr;
use memory::Box;

use super::generic::{self, Page, PagesetExt, PageType};
use super::kernel_pageset;

static PAGE_SIZE: usize = 4096;

pub struct Pageset {
    cr3:    u64,
    pml4:   Box<Pml4>,
    kernel: bool,
}

impl<'a> generic::Pageset<'a> for Pageset {
    type Paddr = usize;
    type Iter  = Iter<'a>;
    type E     = Error;

    fn new() -> Pageset {
        let pml4 = Pml4::alloc(User);

        let cr3 = kernel_pageset()
            .lookup(&*pml4 as *const Pml4 as usize)
            .unwrap() as u64;

        Pageset { cr3: cr3, pml4: pml4, kernel: false }
    }

    fn new_kernel() -> Pageset {
        let pml4 = Pml4::alloc(Kernel);

        let cr3 = kernel_pageset()
            .lookup(&*pml4 as *const Pml4 as usize)
            .unwrap() as u64;

        Pageset { cr3: cr3, pml4: pml4, kernel: true }
    }

    unsafe fn load(&mut self) {
        asm!("mov $0, %cr3"
             :
             : "r" (self.cr3)
             : "memory"
             : "volatile");
    }

    fn page_size() -> usize { PAGE_SIZE }

    fn from(&'a self, vaddr: usize) -> Iter<'a> {
        Iter {
            pageset: self,
            vaddr:   vaddr,

            pdpt:    ptr::null(),
            pd:      ptr::null(),
            pt:      ptr::null(),
        }
    }

    fn modify_while<F>(&mut self, vaddr: usize, callback: F)
                       -> Result<(), Error>
        where F: FnMut(Page<usize>) -> Option<Page<usize>> {
    
        unimplemented!()
    }

    fn modify<F>(&mut self, vaddr: usize, callback: F) -> Result<(), Error>
        where F: FnOnce(Page<usize>) -> Page<usize> {
    
        unimplemented!()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Error {
    OutOfKernelRange(usize),
    OutOfUserRange(usize),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::OutOfKernelRange(_) =>
                "Tried to modify a page in the kernel pageset outside 
                 the kernel address space.",

            Error::OutOfUserRange(_) =>
                "Tried to modify a page in a user pageset outside
                 the user address space.",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(error::Error::description(self))
    }
}

pub struct Iter<'a> {
    pageset: &'a Pageset,
    vaddr:   usize,

    // So we don't have to do a full page walk every time:
    pdpt:    *const Pdpt,
    pd:      *const Pd,
    pt:      *const Pt,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Page<usize>;

    fn next(&mut self) -> Option<Page<usize>> {
        let vaddr = self.vaddr + PAGE_SIZE;

        if self.walk(vaddr + PAGE_SIZE).is_err() {
            return None;
        }

        unsafe {
            Some(self.pt.as_ref().and_then(|pt| pt.get(vaddr.pt_index())))
        }
    }
}

impl<'a> Iter<'a> {
    fn walk(&mut self, vaddr: usize) -> Result<(), ()> {
        if vaddr.pml4_index() != self.vaddr.pml4_index() {
            if !self.pageset.pml4.index(vaddr).is_none() {
                self.pdpt = ptr::null();
                self.pd   = ptr::null();
                self.pt   = ptr::null();

                self.walk_pdpt(vaddr);
            } else {
                return Err(());
            }
        } else if vaddr.pdpt_index() != self.vaddr.pdpt_index() {
            self.pd = ptr::null();
            self.pt = ptr::null();

            self.walk_pd(vaddr);
        } else if vaddr.pd_index() != self.vaddr.pd_index() {
            self.pt = ptr::null();

            self.walk_pt(vaddr);
        }

        self.vaddr = vaddr;
        Ok(())
    }

    fn walk_pdpt(&mut self, vaddr: usize) {
        self.pdpt = self.pageset.pml4
                        .get(vaddr.pml4_index())
                        .map(|pdpt| pdpt as *const Pdpt)
                        .unwrap_or(ptr::null());

        if !self.pdpt.is_null() {
            self.walk_pd(vaddr);
        }
    }

    fn walk_pd(&mut self, vaddr: usize) {
        let pdpt = unsafe { self.pdpt.as_ref().unwrap() };

        self.pd = pdpt.get(vaddr.pdpt_index())
                      .map(|pd| pd as *const Pd)
                      .unwrap_or(ptr::null());

        if !self.pd.is_null() {
            self.walk_pt(vaddr);
        }
    }

    fn walk_pt(&mut self, vaddr: usize) {
        let pd = unsafe { self.pd.as_ref().unwrap() };

        self.pt = pd.get(vaddr.pd_index())
                    .map(|pt| pt as *const Pt)
                    .unwrap_or(ptr::null());
    }
}

trait Bits<Idx> {
    fn bit(self, index: Idx) -> bool;
    fn set_bit(&mut self, index: Idx, value: bool);

    fn bits(self, range: Range<Idx>) -> Self;
    fn set_bits(&mut self, range: Range<Idx>, value: Self);
}

impl<T: Int> Bits<usize> for T {
    #[inline]
    fn bit(self, index: usize) -> bool {
        (self >> index) & T::one() == T::one()
    }

    #[inline]
    fn set_bit(&mut self, index: usize, value: bool) {
        let value_i = if value { T::one() } else { T::zero() };

        *self = *self & !(T::one() << index);
        *self = *self | (value_i << index);
    }

    #[inline]
    fn bits(self, range: Range<usize>) -> Self {
        (self >> range.start) & !(!T::zero() << (range.end - range.start))
    }

    #[inline]
    fn set_bits(&mut self, range: Range<usize>, value: Self) {
        let mask = !(!T::zero() << (range.end - range.start));

        *self = *self & (mask << range.start);
        *self = *self | ((value & mask) << range.start);
    }
}

trait VAddrExt {
    fn prefix(self)     -> Self;

    fn pml4_index(self) -> Self;
    fn pdpt_index(self) -> Self;
    fn pd_index(self)   -> Self;
    fn pt_index(self)   -> Self;

    fn offset_1g(self)  -> Self;
    fn offset_2m(self)  -> Self;
    fn offset_4k(self)  -> Self;
}

impl VAddrExt for usize {
    fn prefix(self)     -> usize { self.bits(48..64) }

    fn pml4_index(self) -> usize { self.bits(39..48) }
    fn pdpt_index(self) -> usize { self.bits(30..39) }
    fn pd_index(self)   -> usize { self.bits(21..30) }
    fn pt_index(self)   -> usize { self.bits(12..21) }

    fn offset_1g(self)  -> usize { self.bits(0..30) }
    fn offset_2m(self)  -> usize { self.bits(0..21) }
    fn offset_4k(self)  -> usize { self.bits(0..12) }
}

trait PageDirectory {
    type Next;

    fn get<'a>(&'a self, index: usize) -> Option<&'a Self::Next>;
}

#[derive(PartialEq, Eq, Debug, Copy)]
pub enum Pml4Kind {
    User,
    Kernel
}
use self::Pml4Kind::*;

#[repr(packed)]
pub struct Pml4 {
    entries: [u64; 512],
    pdpts:   [Option<Box<Pdpt>>; 256],
    kind:    Pml4Kind,
}

impl Pml4 {
    fn new(kind: Pml4Kind) -> Pml4 {
        Pml4 { entries: [0; 512], pdpts: unsafe { mem::zeroed() }, kind: kind }
    }

    pub fn alloc(kind: Pml4Kind) -> Box<Pml4> {
        Box::with_alignment(4096, Pml4::new(kind))
    }

    /// Returns the PML4 index for the given virtual address if and only if the
    /// address is canonical and within the allowed range for this PML4.
    fn index(&self, vaddr: usize) -> Option<usize> {
        let index = vaddr.pml4_index();

        match self.kind {
            User if vaddr.prefix() == 0 && index < 256 =>
                Some(index),
            Kernel if vaddr.prefix() == 0xffff && index >= 256 && index < 512 =>
                Some(index),
            _ =>
                None
        }
    }
}

impl PageDirectory for Pml4 {
    type Next = Pdpt;

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pdpt> {
        self.index(index).and_then(|index|
            self.pdpts[index % 256].as_ref().map(|pdpt| &**pdpt))
    }
}

#[repr(packed)]
pub struct Pdpt {
    entries: [u64; 512],
    pds:     [Option<Box<Pd>>; 512],
}

impl Pdpt {
    fn new() -> Pdpt {
        Pdpt { entries: [0; 512], pds: unsafe { mem::zeroed() } }
    }

    fn alloc() -> Box<Pdpt> {
        Box::with_alignment(4096, Pdpt::new())
    }
}

impl PageDirectory for Pdpt {
    type Next = Pd;

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pd> {
        self.pds.get(index).and_then(|pd| pd.as_ref()).map(|pd| &**pd)
    }
}

#[repr(packed)]
pub struct Pd {
    entries: [u64; 512],
    pts:     [Option<Box<Pt>>; 512],
}

impl Pd {
    fn new() -> Pd {
        Pd { entries: [0; 512], pts: unsafe { mem::zeroed() } }
    }

    fn alloc() -> Box<Pd> {
        Box::with_alignment(4096, Pd::new())
    }
}

impl PageDirectory for Pd {
    type Next = Pt;

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pt> {
        self.pts.get(index).and_then(|pt| pt.as_ref()).map(|pt| &**pt)
    }
}

#[repr(packed)]
pub struct Pt {
    entries: [u64; 512]
}

impl Pt {
    fn new() -> Pt {
        Pt { entries: [0; 512] }
    }

    fn alloc() -> Box<Pt> {
        Box::with_alignment(4096, Pt::new())
    }

    fn get(&self, index: usize) -> Page<usize> {
        self.entries.get(index).and_then(|entry| {
            let present  = 0;
            let writable = 1;
            let user     = 2;

            if entry.bit(present) {
                let mut page_type = PageType::default();

                if entry.bit(writable) { page_type = page_type.writable(); }
                if entry.bit(user)     { page_type = page_type.user();     }

                Some(((entry.bits(12..48) << 12) as usize, page_type))
            } else {
                None
            }
        })
    }
}
