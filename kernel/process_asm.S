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

# XXX: this really isn't what we want to do, but whatever
# good enough for the demo ;)
process_asm_stack: .quad 0

# MSRs we need to set
.set IA32_EFER,  0xC0000080
.set IA32_STAR,  0xC0000081
.set IA32_LSTAR, 0xC0000082
.set IA32_FMASK, 0xC0000084

# GDT-related constants
.set KERNEL_CODE_SEL, 0x08
.set KERNEL_DATA_SEL, 0x10
.set   USER_CD32_SEL, 0x1b
.set   USER_DATA_SEL, 0x23
.set   USER_CD64_SEL, 0x2b

.section .text

# C: (process_registers_t *) -> void
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

  # The register set for the process is pointed to by %rdi
  push %rdi

  # Enable system calls
  mov $IA32_EFER, %ecx
  rdmsr
  or $1, %eax
  wrmsr

  # Put our segment selectors into IA32_STAR
  mov $IA32_STAR, %ecx
  rdmsr
  mov $((USER_CD32_SEL << 16) + KERNEL_CODE_SEL), %edx
  wrmsr

  # Put where we intend to be after the syscall from user code into IA32_LSTAR
  mov    $IA32_LSTAR, %ecx
  movabs $.Lreturn,   %rax
  mov    %rax,        %rdx
  shr    $32,         %rdx
  mov    %eax,        %eax
  wrmsr

  # Put zero into the IA32_FMASK
  mov $IA32_FMASK, %ecx
  rdmsr
  xor %eax, %eax
  wrmsr

  # Save our current stack
  mov %rsp, (process_asm_stack)

  # We'll put the register set into R11 instead for now
  mov %rdi, %r11

  # Load the process's register set
  # Ignore RCX and R11 because those are used for RIP and EFLAGS respectively
  # Don't load RSP yet because we still need the stack
  mov 0x00(%r11), %rax

  mov 0x10(%r11), %rdx
  mov 0x18(%r11), %rbx

  mov 0x28(%r11), %rbp
  mov 0x30(%r11), %rsi
  mov 0x38(%r11), %rdi
  mov 0x40(%r11), %r8
  mov 0x48(%r11), %r9
  mov 0x50(%r11), %r10

  mov 0x60(%r11), %r12
  mov 0x68(%r11), %r13
  mov 0x70(%r11), %r14
  mov 0x78(%r11), %r15

  # Now put RSP, RIP, EFLAGS on the stack
  mov 0x20(%r11), %rcx
  push %rcx
  mov 0x80(%r11), %rcx
  push %rcx
  mov 0x88(%r11), %ecx
  push %rcx

  # Set the data segments for the user process
  mov $USER_DATA_SEL, %cx
  mov %cx, %ds
  mov %cx, %es
  mov %cx, %fs
  mov %cx, %gs

  # And take EFLAGS -> R11, RIP -> RCX, RSP -> RSP
  pop %r11
  pop %rcx
  pop %rsp

  # Now we can jump into user code
  sysretq

.Lreturn:
  # Swap RCX and R11 for now
  xchg %rcx, %r11

  # Shift RCX left so we can use CX
  shl $32, %rcx

  # Set the data segments for the kernel
  or  $KERNEL_DATA_SEL, %cx
  mov %cx, %ds
  mov %cx, %es
  mov %cx, %fs
  mov %cx, %gs

  # Now reset RCX/R11 to what they were before
  shr  $32, %rcx
  xchg %r11, %rcx

  # Reload our stack and simultaneously save the user stack.
  xchg (process_asm_stack), %rsp

  # Swap the process's register set address with R11 (EFLAGS)
  xchg (%rsp), %r11

  # Save the process's register set
  # This time, clear RCX and R11
  # Don't save RSP yet
  movq %rax, 0x00(%r11)
  movq $0,   0x08(%r11)
  movq %rdx, 0x10(%r11)
  movq %rbx, 0x18(%r11)

  movq %rbp, 0x28(%r11)
  movq %rsi, 0x30(%r11)
  movq %rdi, 0x38(%r11)
  movq %r8,  0x40(%r11)
  movq %r9,  0x48(%r11)
  movq %r10, 0x50(%r11)
  movq $0,   0x58(%r11)
  movq %r12, 0x60(%r11)
  movq %r13, 0x68(%r11)
  movq %r14, 0x70(%r11)
  movq %r15, 0x78(%r11)

  # Save (process_asm_stack) -> RSP
  mov (process_asm_stack), %rax
  mov %rax, 0x20(%r11)

  # And save RCX -> RIP
  mov %rcx, 0x80(%r11)

  # And pop the old R11 into RAX and save it as EFLAGS
  pop %rax
  mov %eax, 0x88(%r11)

  # Now prepare to return back into C (System V ABI)
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %rbx
  pop %rbp
  xor %rax, %rax

  # Enable interrupts and return
  sti
  retq
