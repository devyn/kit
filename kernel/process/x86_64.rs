/*******************************************************************************
 *
 * kit/kernel/process/x86_64.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! x86-64 architecture-specific process logic and hardware state.

use memory;
use core::isize;
use core::ptr;

#[repr(C)]
#[derive(Debug)]
pub struct Registers {
    rax:     usize,
    rcx:     usize,
    rdx:     usize,
    rbx:     usize,
    rsp:     usize,
    rbp:     usize,
    rsi:     usize,
    rdi:     usize,
    r8:      usize,
    r9:      usize,
    r10:     usize,
    r11:     usize,
    r12:     usize,
    r13:     usize,
    r14:     usize,
    r15:     usize,
    rip:     usize,
    eflags:  u32,
}

impl Default for Registers {
    fn default() -> Registers {
        Registers {
            rax: 0, rcx: 0, rdx: 0, rbx: 0,
            rsp: 0, rbp: 0, rsi: 0, rdi: 0,
            r8:  0, r9:  0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0,
            eflags: 0,
        }
    }
}

pub static ARGS_TOP_ADDR:   usize = 0x0000_7fee_ffff_ffff;
pub static STACK_BASE_ADDR: usize = 0x0000_7fff_ffff_f000;
pub static HEAP_BASE_ADDR:  usize = 0x0000_0001_0000_0000;

/// The hardware state of a process. Usually mutated by foreign code.
///
/// On x86_64, includes the kernel stack pointer and base, as well as the
/// registers.
#[allow(drop_with_repr_extern)]
#[repr(C)]
#[derive(Debug)]
#[allow(raw_pointer_derive)]
pub struct HwState {
    kstack_base:    *mut u8,    // offset 0x00
    kstack_pointer: *mut u8,    // offset 0x08
    registers:      Registers,  // offset 0x10
}

static KSTACK_SIZE: usize = 8192;
static KSTACK_ALIGN: usize = 16;

extern {
    static mut process_hwstate: *mut HwState;
}

impl HwState {
    pub fn new() -> HwState {
        unsafe {
            let kstack = memory::allocate(KSTACK_SIZE, KSTACK_ALIGN);

            debug_assert!(KSTACK_SIZE < isize::MAX as usize);

            HwState {
                kstack_base:    kstack,
                kstack_pointer: kstack.offset(KSTACK_SIZE as isize),
                registers:      Registers::default(),
            }
        }
    }

    /// Loads the HwState as the current HwState.
    pub fn load(&mut self) {
        unsafe {
            process_hwstate = self as *mut HwState;
        }
    }

    /// Set the entry point arguments to `(argc, argv)`.
    ///
    /// Pass `None` to set to `(0, NULL)`.
    pub fn set_args(&mut self, args: Option<(i32, usize)>) {
        if let Some((argc, argv)) = args {
            self.registers.rdi = argc as usize;
            self.registers.rsi = argv;
        } else {
            self.registers.rdi = 0;
            self.registers.rsi = ptr::null::<u8>() as usize;
        }
    }

    /// Set the instruction pointer to the given address.
    pub fn set_instruction_pointer(&mut self, vaddr: usize) {
        self.registers.rip = vaddr;
    }

    /// Call this instead of dropping directly.
    ///
    /// Memory for this is completely managed by Rust, and the `repr(C)` is
    /// just so that the asm code can predictably access the fields.
    /// However, Rust doesn't let us just implement `Drop` on a `repr(C)`
    /// without complaining.
    pub fn deallocate(self) {
        unsafe {
            memory::deallocate(self.kstack_base, KSTACK_SIZE, KSTACK_ALIGN);
        }
    }
}
