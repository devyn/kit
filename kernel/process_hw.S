################################################################################
#
# kit/kernel/process_hw.S
# - user process hardware helpers
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .data

# GDT-related constants
.set USER_CD32_SEL, 0x1b
.set USER_DATA_SEL, 0x23
.set USER_CD64_SEL, 0x2b

# Current process HwState pointer
.global process_hwstate
process_hwstate: .quad 0

# HwState offsets
.set HWSTATE_KSTACK_BASE,    0x00
.set HWSTATE_KSTACK_POINTER, 0x08
.set HWSTATE_REGISTERS,      0x10

.section .text

# Rust: (stack_pointer: *mut u8) -> *mut u8
.global process_hw_prepare
process_hw_prepare:
  xchg %rdi, %rsp
  pushq $.Lbootstrap # return address
  pushq $0
  pushq $0
  pushq $0
  pushq $0
  pushq $0
  pushq $0
  xchg %rdi, %rsp
  mov  %rdi, %rax
  retq

.Lbootstrap:
  # Save our current stack to the TSS.
  mov %rsp, tss_stack

  # Load the current process's registers' base address into R15.
  mov (process_hwstate), %r15
  add $HWSTATE_REGISTERS, %r15

  # Put SS, RSP, EFLAGS, CS, RIP on the stack
  pushq $USER_DATA_SEL # SS

  mov   0x20(%r15), %rcx # RSP
  push  %rcx

  mov   0x88(%r15), %ecx # EFLAGS
  or    $(1 << 9),  %ecx # make sure interrupts are enabled
  push  %rcx

  pushq $USER_CD64_SEL # CS

  mov   0x80(%r15), %rcx # RIP
  push  %rcx

  # Load the FXSAVE data
  fxrstor64 0x90(%r15) # FXSAVE

  # Load the remainder of the general purpose registers
  mov 0x00(%r15), %rax
  mov 0x08(%r15), %rcx
  mov 0x10(%r15), %rdx
  mov 0x18(%r15), %rbx
  #- omit rsp -#
  mov 0x28(%r15), %rbp
  mov 0x30(%r15), %rsi
  mov 0x38(%r15), %rdi
  mov 0x40(%r15), %r8
  mov 0x48(%r15), %r9
  mov 0x50(%r15), %r10
  mov 0x58(%r15), %r11
  mov 0x60(%r15), %r12
  mov 0x68(%r15), %r13
  mov 0x70(%r15), %r14
  mov 0x78(%r15), %r15

  # Set the data segments for the user process
  push %rax

  mov $USER_DATA_SEL, %ax
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs

  pop %rax

  # Now we can jump into user code
  iretq

# Halt if something goes wrong.
.LsomethingWentWrong:
  cli
  hlt
  jmp .LsomethingWentWrong

# Rust: (old: *mut HwState, new: *mut HwState) -> ()
# This function is a bit tricky because it actually returns out of a different
# invocation of this function on a different stack.
#
# (Which is why we don't do it in Rust.)
.global process_hw_switch
process_hw_switch:
  # Disable interrupts, though they probably should be disabled already anyway.
  cli

  # System V ABI requirements
  push %rbp
  push %rbx
  push %r12
  push %r13
  push %r14
  push %r15

  # Save stack
  mov %rsp,             HWSTATE_KSTACK_POINTER(%rdi)
  mov rust_stack_limit, %rbp
  mov %rbp,             HWSTATE_KSTACK_BASE(%rdi)

  # Switch stacks
  mov HWSTATE_KSTACK_POINTER(%rsi), %rsp
  mov HWSTATE_KSTACK_BASE(%rsi),    %rbp
  mov %rbp,                         rust_stack_limit

  # Set process_hwstate
  mov %rsi, process_hwstate

  # Return into process_switch() on other stack
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %rbx
  pop %rbp
  retq
