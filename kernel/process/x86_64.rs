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
    rax:     usize,       // 0x00
    rcx:     usize,       // 0x08
    rdx:     usize,       // 0x10
    rbx:     usize,       // 0x18
    rsp:     usize,       // 0x20
    rbp:     usize,       // 0x28
    rsi:     usize,       // 0x30
    rdi:     usize,       // 0x38
    r8:      usize,       // 0x40
    r9:      usize,       // 0x48
    r10:     usize,       // 0x50
    r11:     usize,       // 0x58
    r12:     usize,       // 0x60
    r13:     usize,       // 0x68
    r14:     usize,       // 0x70
    r15:     usize,       // 0x78
    rip:     usize,       // 0x80
    eflags:  u32,         // 0x88
    fxsave:  [SSEReg; 32] // 0x90
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
            fxsave: [
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
                SSEReg(0,0,0,0), SSEReg(0,0,0,0),
            ]
        }
    }
}

#[repr(simd)]
#[derive(Debug)]
pub struct SSEReg(u32, u32, u32, u32);

pub static ARGS_TOP_ADDR:   usize = 0x0000_7fee_ffff_ffff;
pub static STACK_BASE_ADDR: usize = 0x0000_7fff_ffff_f000;
pub static HEAP_BASE_ADDR:  usize = 0x0000_0001_0000_0000;

pub static STACK_SIZE:      usize = 8192;

/// The hardware state of a process. Usually mutated by foreign code.
///
/// On x86_64, includes the kernel stack pointer and base, as well as the
/// registers.
#[repr(C)]
#[derive(Debug)]
pub struct HwState {
    kstack_base:    *mut u8,    // offset 0x00
    kstack_pointer: *mut u8,    // offset 0x08
    registers:      Registers,  // offset 0x10
}

static KSTACK_SIZE: usize = 8192;
static KSTACK_ALIGN: usize = 16;

extern {
    fn process_hw_prepare(stack_pointer: *mut u8) -> *mut u8;
}

impl HwState {
    pub fn new() -> HwState {
        unsafe {
            let kstack = memory::allocate(KSTACK_SIZE, KSTACK_ALIGN);

            debug_assert!(KSTACK_SIZE < isize::MAX as usize);

            HwState {
                kstack_base:    kstack,
                kstack_pointer: process_hw_prepare(
                                    kstack.offset(KSTACK_SIZE as isize)),
                registers:      Registers {
                    rsp: STACK_BASE_ADDR,
                    ..Registers::default()
                },
            }
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
