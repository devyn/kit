################################################################################
#
# kit/kernel/process_asm.S
# - user process trampoline
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

.section .text

# C: (void) -> void
.global process_asm_call
process_asm_call:
  # Disable interrupts
  cli

  # System V ABI requirements
  push %rbp
  push %rbx
  push %r12
  push %r13
  push %r14
  push %r15

  # Save our current stack to the TSS.
  mov %rsp, tss_stack

  # Load the current process's registers' base address into R15.
  mov process_current,                  %r15
  add SYSCALL_OFFSET_PROCESS_REGISTERS, %r15

  # Put SS, RSP, EFLAGS, CS, RIP on the stack
  pushq $USER_DATA_SEL # SS

  mov   0x20(%r15), %rcx # RSP
  push  %rcx

  mov   0x88(%r15), %ecx # EFLAGS
  push  %rcx

  pushq $USER_CD64_SEL # CS

  mov   0x80(%r15), %rcx # RIP
  push  %rcx

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
