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

.include "constants.S"

.section .data

.section .text

# Prepares to go to kernel code.
#
# Requires two arguments passed on the stack: rip (function to call),
# rdi (argument)
#
# The function to call is basically expected to be:
#
#   int procedure(size_t arg)
#
# The return of the function is the exit code for process_exit, which the entry
# point will call automatically after return.
.global process_hw_enter_kernel
process_hw_enter_kernel:
  pop %rcx
  pop %rdi
  call *%rcx
  mov %eax, %edi
  call process_exit
.Lunreachable:
  cli
  hlt
  jmp .Lunreachable

# Prepares to go to user code.
.global process_hw_enter_user
process_hw_enter_user:
  # Save our current stack to the TSS.
  mov %rsp, tss_stack

  # Load the current process's registers' base address into R15.
  mov %gs:CPULOCAL_HWSTATE, %r15
  add $HWSTATE_USER_REGISTERS, %r15

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
  swapgs

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

  # Put return address (RIP) at R9
  pop %r9

  # Save old kernel registers
  #
  # System V ABI requirements dictate this limited set as callee-preserved
  mov %r9,  (HWSTATE_KERN_REGISTERS+0x00)(%rdi)
  mov %rsp, (HWSTATE_KERN_REGISTERS+0x08)(%rdi)
  mov %rbp, (HWSTATE_KERN_REGISTERS+0x10)(%rdi)
  mov %rbx, (HWSTATE_KERN_REGISTERS+0x18)(%rdi)
  mov %r12, (HWSTATE_KERN_REGISTERS+0x20)(%rdi)
  mov %r13, (HWSTATE_KERN_REGISTERS+0x28)(%rdi)
  mov %r14, (HWSTATE_KERN_REGISTERS+0x30)(%rdi)
  mov %r15, (HWSTATE_KERN_REGISTERS+0x38)(%rdi)

  # Load new kernel registers
  mov (HWSTATE_KERN_REGISTERS+0x00)(%rsi), %r9
  mov (HWSTATE_KERN_REGISTERS+0x08)(%rsi), %rsp
  mov (HWSTATE_KERN_REGISTERS+0x10)(%rsi), %rbp
  mov (HWSTATE_KERN_REGISTERS+0x18)(%rsi), %rbx
  mov (HWSTATE_KERN_REGISTERS+0x20)(%rsi), %r12
  mov (HWSTATE_KERN_REGISTERS+0x28)(%rsi), %r13
  mov (HWSTATE_KERN_REGISTERS+0x30)(%rsi), %r14
  mov (HWSTATE_KERN_REGISTERS+0x38)(%rsi), %r15

  # Set current hwstate to new one
  mov %rsi, %gs:CPULOCAL_HWSTATE

  # Jump to previously saved instruction pointer
  jmp *%r9
