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

use constants::{KERNEL_OFFSET, KERNEL_LOW_START, KERNEL_LOW_END};

use super::generic::{self, Page, PagesetExt, PageType};

static PAGE_SIZE: usize = 4096;

/// This is the only safe way to get the physical address of a kernel pointer
/// during paging operations.
///
/// It's not safe outside of paging operations however, because it assumes that
/// if paging is not initialized yet then the initial identity mapping is in
/// place, and `kernel_pageset_unsafe()` is used so that we can still do lookups
/// while modifying the kernel pageset. This in itself is technically unsafe,
/// but there's really no other option within this module.
fn safe_lookup<T>(ptr: *const T) -> Option<usize> {

    if super::initialized() {
        unsafe {
            super::kernel_pageset_unsafe().lookup(ptr as usize)
        }
    } else {
        let map_start = KERNEL_OFFSET + KERNEL_LOW_START as usize;
        let map_end   = KERNEL_OFFSET + KERNEL_LOW_END as usize;
        let vaddr     = ptr as usize;

        if vaddr >= map_start && vaddr > map_end {
            Some(vaddr - KERNEL_OFFSET)
        } else {
            None
        }
    }
}

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
        let cr3  = safe_lookup(&*pml4).unwrap() as u64;

        Pageset { cr3: cr3, pml4: pml4, kernel: false }
    }

    fn new_kernel() -> Pageset {
        let pml4 = Pml4::alloc(Kernel);
        let cr3  = safe_lookup(&*pml4).unwrap() as u64;

        let mut pageset = Pageset { cr3: cr3, pml4: pml4, kernel: true };

        // Insert the identity map.
        let identity_map_insert_result =
            pageset.map_pages_with_type(
                KERNEL_OFFSET,
                Pageset::range(KERNEL_LOW_START as usize,
                               KERNEL_LOW_END as usize),
                PageType::default().writable().executable().not_user());

        assert!(identity_map_insert_result.is_ok());

        pageset
    }

    unsafe fn load(&mut self) {
        asm!("mov $0, %cr3"
             :
             : "r" (self.cr3)
             : "memory"
             : "volatile");
    }

    fn page_size() -> usize { PAGE_SIZE }

    fn is_kernel_pageset(&self) -> bool { self.kernel }

    fn from(&'a self, vaddr: usize) -> Iter<'a> {
        Iter {
            pageset: self,
            first:   true,
            vaddr:   vaddr,

            pdpt:    ptr::null(),
            pd:      ptr::null(),
            pt:      ptr::null(),
        }
    }

    fn modify_while<F>(&mut self, vaddr: usize, mut callback: F)
                       -> Result<(), Error>
        where F: FnMut(Page<usize>) -> Option<Page<usize>> {

        Pml4::modify_while(&mut *self.pml4, vaddr, &mut callback).into_result()
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
    first:   bool,
    vaddr:   usize,

    // So we don't have to do a full page walk every time:
    pdpt:    *const Pdpt,
    pd:      *const Pd,
    pt:      *const Pt,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Page<usize>;

    fn next(&mut self) -> Option<Page<usize>> {
        let vaddr = if self.first { self.vaddr } else { self.vaddr + PAGE_SIZE };

        if self.walk(vaddr).is_err() {
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
            if !self.pageset.pml4.index_if_ok(vaddr).is_none() {
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

#[must_use]
#[derive(PartialEq, Eq, Debug)]
enum ModifyWhileState {
    Continue(usize),
    Done,
    Error(Error)
}

impl ModifyWhileState {
    fn into_result(self) -> Result<(), Error> {
        use self::ModifyWhileState::*;

        match self {
            Continue(_) =>
                panic!("The final state of modify_while() was Continue!"),

            Done => Ok(()),

            Error(e) => Err(e)
        }
    }
}

trait PageDirectory {
    type Next: ModifyWhile<Hole=Option<Box<Self::Next>>>;

    fn index(vaddr: usize) -> usize;

    fn get<'a>(&'a self, index: usize) -> Option<&'a Self::Next>;
}

trait InnerPageDirectory: PageDirectory {
    fn within_same(vaddr1: usize, vaddr2: usize) -> bool;

    fn alloc() -> Box<Self>;

    fn get_mut_hole<'a>(&'a mut self, index: usize)
                        -> &'a mut Option<Box<Self::Next>>;

    fn update_entry(&mut self, index: usize);
}

trait ModifyWhile {
    type Hole;

    fn modify_while<F>(hole: &mut Self::Hole, vaddr: usize, callback: &mut F)
                       -> ModifyWhileState
        where F: FnMut(Page<usize>) -> Option<Page<usize>>;
}

impl ModifyWhile for Pml4 {
    type Hole = Pml4;

    fn modify_while<F>(pml4: &mut Pml4, vaddr: usize, callback: &mut F)
                       -> ModifyWhileState
        where F: FnMut(Page<usize>) -> Option<Page<usize>> {

        use self::ModifyWhileState::*;
        use self::Error::*;

        // Verify that the address is in range.
        match pml4.kind {
            User if !User.vaddr_ok(vaddr) => {
                return Error(OutOfUserRange(vaddr));
            },

            Kernel if !Kernel.vaddr_ok(vaddr) => {
                return Error(OutOfKernelRange(vaddr));
            },

            _ => ()
        }

        let index = Pml4::index(vaddr);

        let state = Pdpt::modify_while(&mut pml4.pdpts[index % 256],
                                       vaddr,
                                       callback);

        pml4.update_entry(index);

        match state {
            Continue(next_vaddr) =>
                Pml4::modify_while(pml4, next_vaddr, callback),

            _ => state
        }
    }
}

impl<T: InnerPageDirectory> ModifyWhile for T {
    type Hole = Option<Box<T>>;

    fn modify_while<F>(hole: &mut Option<Box<T>>,
                       vaddr: usize,
                       callback: &mut F)
                       -> ModifyWhileState
        where F: FnMut(Page<usize>) -> Option<Page<usize>> {

        use self::ModifyWhileState::*;

        let index = T::index(vaddr);

        let mut state;

        if let Some(ref mut me) = *hole {
            state = T::Next::modify_while(me.get_mut_hole(index),
                                          vaddr,
                                          callback);

            me.update_entry(index)
        } else {
            let mut my_next = None;

            state = T::Next::modify_while(&mut my_next, vaddr, callback);

            if my_next.is_some() {
                let mut me = T::alloc();

                *me.get_mut_hole(index) = my_next;

                me.update_entry(index);

                *hole = Some(me);
            }
        }

        match state {
            Continue(next_vaddr) if T::within_same(vaddr, next_vaddr) =>
                T::modify_while(hole, next_vaddr, callback),

            _ => state
        }
    }
}

impl ModifyWhile for Pt {
    type Hole = Option<Box<Pt>>;

    fn modify_while<F>(hole: &mut Option<Box<Pt>>,
                       vaddr: usize,
                       callback: &mut F)
                       -> ModifyWhileState
        where F: FnMut(Page<usize>) -> Option<Page<usize>> {

        use self::ModifyWhileState::*;

        fn invlpg(vaddr: usize) {
            unsafe {
                asm!("invlpg ($0)"
                     :
                     : "r" (vaddr)
                     : "memory"
                     : "volatile");
            }
        }

        let index = vaddr.pt_index();

        if let Some(ref mut pt) = *hole {
            if let Some(page) = callback(pt.get(index)) {
                pt.set(index, page);

                // FIXME: not necessary if this is neither the current nor
                // kernel pageset
                invlpg(vaddr);

            } else {
                return Done;
            }
        } else {
            if let Some(page) = callback(None) {
                let mut pt = Pt::alloc();

                pt.set(index, page);

                // FIXME: not necessary if this is neither the current nor
                // kernel pageset
                invlpg(vaddr);

                *hole = Some(pt);
            } else {
                return Done;
            }
        }

        let next_vaddr = vaddr + PAGE_SIZE;

        if index == 511 {
            Continue(next_vaddr)
        } else {
            Pt::modify_while(hole, next_vaddr, callback)
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy)]
pub enum Pml4Kind {
    User,
    Kernel
}
use self::Pml4Kind::*;

impl Pml4Kind {
    fn vaddr_ok(self, vaddr: usize) -> bool {
        let index  = vaddr.pml4_index();
        let prefix = vaddr.prefix();

        match self {
            User   => prefix == 0      && index <  256,
            Kernel => prefix == 0xffff && index >= 256 && index < 512
        }
    }
}

#[repr(packed)]
struct Pml4 {
    entries: [u64; 512],
    pdpts:   [Option<Box<Pdpt>>; 256],
    kind:    Pml4Kind,
}

impl Pml4 {
    fn new(kind: Pml4Kind) -> Pml4 {
        Pml4 { entries: [0; 512], pdpts: unsafe { mem::zeroed() }, kind: kind }
    }

    fn alloc(kind: Pml4Kind) -> Box<Pml4> {
        Box::with_alignment(PAGE_SIZE, Pml4::new(kind))
    }

    /// Returns the PML4 index for the given virtual address if and only if the
    /// address is canonical and within the allowed range for this PML4.
    fn index_if_ok(&self, vaddr: usize) -> Option<usize> {
        if self.kind.vaddr_ok(vaddr) {
            Some(vaddr.pml4_index())
        } else {
            None
        }
    }

    fn update_entry(&mut self, index: usize) {
        // We have to use kernel_pageset_unsafe() because this could be the
        // kernel pageset we're updating, and we have no other way to grab the
        // physical address.
        
        if let Some(ref pdpt) = self.pdpts[index] {
            let mut entry: u64 = 0x7; // present, writable, user

            let paddr = safe_lookup(&**pdpt)
                .expect("failed to find pdpt's physical address");

            entry.set_bits(12..48, (paddr << 12) as u64);

            self.entries[index] = entry;
        } else {
            self.entries[index] = 0;
        }
    }
}

impl PageDirectory for Pml4 {
    type Next = Pdpt;

    fn index(vaddr: usize) -> usize { vaddr.pml4_index() }

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pdpt> {
        self.index_if_ok(index).and_then(|index|
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
}

impl PageDirectory for Pdpt {
    type Next = Pd;

    fn index(vaddr: usize) -> usize { vaddr.pdpt_index() }

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pd> {
        self.pds.get(index).and_then(|pd| pd.as_ref()).map(|pd| &**pd)
    }
}

impl InnerPageDirectory for Pdpt {
    fn alloc() -> Box<Pdpt> {
        Box::with_alignment(PAGE_SIZE, Pdpt::new())
    }

    fn within_same(vaddr1: usize, vaddr2: usize) -> bool {
        vaddr1.pml4_index() == vaddr2.pml4_index()
    }

    fn get_mut_hole<'a>(&'a mut self, index: usize)
                        -> &'a mut Option<Box<Pd>> {
        &mut self.pds[index]
    }

    fn update_entry(&mut self, index: usize) {
        // We have to use kernel_pageset_unsafe() because this could be the
        // kernel pageset we're updating, and we have no other way to grab the
        // physical address.
        
        if let Some(ref pd) = self.pds[index] {
            let mut entry: u64 = 0x7; // present, writable, user

            let paddr = safe_lookup(&**pd)
                .expect("failed to find pd's physical address");

            entry.set_bits(12..48, (paddr << 12) as u64);

            self.entries[index] = entry;
        } else {
            self.entries[index] = 0;
        }
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
}

impl PageDirectory for Pd {
    type Next = Pt;

    fn index(vaddr: usize) -> usize { vaddr.pd_index() }

    fn get<'a>(&'a self, index: usize) -> Option<&'a Pt> {
        self.pts.get(index).and_then(|pt| pt.as_ref()).map(|pt| &**pt)
    }
}

impl InnerPageDirectory for Pd {
    fn alloc() -> Box<Pd> {
        Box::with_alignment(PAGE_SIZE, Pd::new())
    }

    fn within_same(vaddr1: usize, vaddr2: usize) -> bool {
        vaddr1.pdpt_index() == vaddr2.pdpt_index()
    }

    fn get_mut_hole<'a>(&'a mut self, index: usize)
                        -> &'a mut Option<Box<Pt>> {
        &mut self.pts[index]
    }

    fn update_entry(&mut self, index: usize) {
        // We have to use kernel_pageset_unsafe() because this could be the
        // kernel pageset we're updating, and we have no other way to grab the
        // physical address.
        
        if let Some(ref pt) = self.pts[index] {
            let mut entry: u64 = 0x7; // present, writable, user

            let paddr = safe_lookup(&**pt)
                .expect("failed to find pt's physical address");

            entry.set_bits(12..48, (paddr << 12) as u64);

            self.entries[index] = entry;
        } else {
            self.entries[index] = 0;
        }
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
        Box::with_alignment(PAGE_SIZE, Pt::new())
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

    fn set(&mut self, index: usize, page: Page<usize>) {
        self.entries[index] =
            if let Some((paddr, page_type)) = page {
                let writable = 1;
                let user     = 2;

                let mut entry: u64 = 1;

                entry.set_bits(12..48, (paddr >> 12) as u64);

                if page_type.is_writable() { entry.set_bit(writable, true); }
                if page_type.is_user()     { entry.set_bit(user,     true); }

                entry
            } else {
                0
            };
    }
}
