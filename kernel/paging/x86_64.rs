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
use core::mem;
use memory::Box;

use super::generic;

pub struct Pageset {
    cr3:    u64,
    pml4:   Box<Pml4>,
    kernel: bool,
}
/*
impl generic::Pageset for Pageset {
    pub unsafe fn load(&mut self) {
        asm!("mov $0, %cr3"
             :
             : "r" (self.cr3)
             : "memory"
             : "volatile");
    }
}
*/

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
            Kernel if vaddr.prefix() == 0xffff && index >= 256 =>
                Some(index),
            _ =>
                None
        }
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
}
