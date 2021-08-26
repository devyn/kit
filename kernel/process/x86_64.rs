/*******************************************************************************
 *
 * kit/kernel/process/x86_64.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! x86-64 architecture-specific process logic and hardware state.

use crate::memory;

use core::ptr;
use core::mem;

/// A complete set of registers
#[repr(C, align(16))]
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
    // 0x290
}

assert_eq_size!(Registers, [u8; 0x290]);

impl Default for Registers {
    fn default() -> Registers {
        Registers {
            rax: 0, rcx: 0, rdx: 0, rbx: 0,
            rsp: 0, rbp: 0, rsi: 0, rdi: 0,
            r8:  0, r9:  0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0,
            eflags: 0,
            fxsave: [SSEReg::default(); 32]
        }
    }
}

#[repr(simd, align(16))]
#[derive(Debug, Default, Clone, Copy)]
pub struct SSEReg(u32, u32, u32, u32);

pub const ARGS_TOP_ADDR:   usize = 0x0000_7fee_ffff_ffff;
pub const STACK_BASE_ADDR: usize = 0x0000_7fff_ffff_f000;
pub const HEAP_BASE_ADDR:  usize = 0x0000_0001_0000_0000;

pub const STACK_SIZE:      usize = 8192;

/// The hardware state of a process. Usually mutated by foreign code.
#[repr(C, align(16))]
#[derive(Debug)]
pub struct HwState {
    kernel: KernelHwState, // offset 0x00
    user: UserHwState,     // offset 0x50
}

impl HwState {
    pub fn new() -> HwState {
        HwState {
            kernel: KernelHwState::new(),
            user: UserHwState::new(),
        }
    }

    pub fn kernel(&self) -> &KernelHwState {
        &self.kernel
    }

    pub fn kernel_mut(&mut self) -> &mut KernelHwState {
        &mut self.kernel
    }

    pub fn user(&self) -> &UserHwState {
        &self.user
    }

    pub fn user_mut(&mut self) -> &mut UserHwState {
        &mut self.user
    }
}

/// The kernel part of the hardware state
#[repr(C, align(16))]
#[derive(Debug)]
pub struct KernelHwState {
    kstack: *mut u8,            // 0x00
    registers: KernelRegisters, // 0x10
}

assert_eq_size!(KernelHwState, [u8; 0x50]);

impl KernelHwState {
    fn new() -> KernelHwState {
        let kstack = memory::allocate_kernel_stack();

        KernelHwState {
            kstack,
            registers: KernelRegisters {
                rsp: kstack as usize,
                rbp: kstack as usize,
                ..KernelRegisters::default()
            }
        }
    }

    /// Set the address to jump to on process switch.
    ///
    /// # Unsafety
    ///
    /// This can result in any code (safe or unsafe) being executed at kernel
    /// level, and dereference of invalid addresses, so it's up to the caller to
    /// ensure the value provided will do something safe.
    pub unsafe fn set_instruction_pointer(&mut self, ip: usize) {
        self.registers.rip = ip;
    }

    /// Push a value onto the stack. The stack pointer advanced by the size of
    /// `T` and then aligned as required for `T`.
    ///
    /// Because the value will be sent to another thread, it must be `Send`.
    ///
    /// # Unsafety
    ///
    /// As this modifies the kernel stack directly, this could cause a violation
    /// of expectations. It's up to the caller to ensure the instruction pointer
    /// is set to a place that can handle the argument passed.
    ///
    /// The kernel stack set must also be mapped to valid, nonoverlapping
    /// memory.
    pub unsafe fn push_stack<T>(&mut self, arg: T) 
        where T: Send {

        // Alignment must be power of two. This should be guaranteed, but I
        // didn't see that the Rust docs guarantee it.
        let align = mem::align_of::<T>();
        debug_assert_eq!(align & (align - 1), 0);

        // Subtract size
        self.registers.rsp -= mem::size_of::<T>();

        // Align downward
        self.registers.rsp &= !(align - 1);

        // Put the value in
        *(self.registers.rsp as *mut T) = arg;
    }
}

impl Drop for KernelHwState {
    fn drop(&mut self) {
        // TODO: free the kernel stack
    }
}

/// The registers here are chosen because they are those required to be
/// preserved between calls by the ABI. A process switch just looks like a call.
#[repr(C, align(16))]
#[derive(Debug, Default, Clone)]
struct KernelRegisters {
    rip: usize, // 0x00
    rsp: usize, // 0x08
    rbp: usize, // 0x10
    rbx: usize, // 0x18
    r12: usize, // 0x20
    r13: usize, // 0x28
    r14: usize, // 0x30
    r15: usize, // 0x38
    // 0x40
}

assert_eq_size!(KernelRegisters, [u8; 0x40]);

/// The userland part of the hardware state.
#[repr(C, align(16))]
#[derive(Debug)]
pub struct UserHwState {
    registers: Registers,
}

assert_eq_size!(UserHwState, [u8; 0x290]);

impl UserHwState {
    fn new() -> UserHwState {
        UserHwState {
            registers: Registers::default(),
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

    /// Set the stack pointer to the given address.
    pub fn set_stack_pointer(&mut self, vaddr: usize) {
        self.registers.rsp = vaddr;
        self.registers.rbp = vaddr;
    }
}
