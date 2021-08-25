################################################################################
#
# kit/kernel/syscall_handler.S
# - handles 'syscall' instructions, saving/restoring process state
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.include "constants.S"

.section .rodata

# FPU init word (div by zero + invalid operand cause exceptions)
FPU_INIT: .word 0x37a

# SSE init dword (same config)
SSE_INIT: .long 0x1d00

.section .text

.global syscall_handler
syscall_handler:
  # Immediately swap to kernel GS in order to get per-CPU data
  swapgs

  # Temporarily save instruction pointer (rcx) in our temp space.
  mov %rcx, %gs:CPULOCAL_SCRATCH

  # Set the data segments for the kernel (SS is already set).
  mov $KERNEL_DATA_SEL, %cx
  mov %cx, %ds
  mov %cx, %es

  # Load the current process's registers' base address into RCX.
  mov %gs:CPULOCAL_HWSTATE, %rcx
  add $HWSTATE_USER_REGISTERS, %rcx

  # Save the process's register set.
  # Clear RCX and R11 (they are clobbered by syscalls).
  mov  %rax,  0x00(%rcx)
  movq $0,    0x08(%rcx) # rcx
  mov  %rdx,  0x10(%rcx)
  mov  %rbx,  0x18(%rcx)
  mov  %rsp,  0x20(%rcx)
  mov  %rbp,  0x28(%rcx)
  mov  %rsi,  0x30(%rcx)
  mov  %rdi,  0x38(%rcx)
  mov  %r8,   0x40(%rcx)
  mov  %r9,   0x48(%rcx)
  mov  %r10,  0x50(%rcx)
  movq $0,    0x58(%rcx) # r11
  mov  %r12,  0x60(%rcx)
  mov  %r13,  0x68(%rcx)
  mov  %r14,  0x70(%rcx)
  mov  %r15,  0x78(%rcx)
  mov  %r11d, 0x88(%rcx) # R11D -> EFLAGS

  # Store the saved instruction pointer (RCX)
  mov  %gs:CPULOCAL_SCRATCH, %r12
  mov  %r12,  0x80(%rcx)

  # Save the FXSAVE data (x87/SSE regs etc.)
  fxsave64 0x90(%rcx)

  # Restore the kernel stack.
  mov %gs:CPULOCAL_HWSTATE, %rsp
  mov HWSTATE_KERN_RBP(%rsp), %rbp
  mov HWSTATE_KERN_RSP(%rsp), %rsp

  # Initialize FS.base. (preserve rax, rdx)
  mov %rax, %r12
  mov %rdx, %r13
  mov    $IA32_FS_BASE, %ecx
  movabs $(rust_stack_limit - 0x70), %rdx
  mov    %edx, %eax
  shr    $32, %rdx
  wrmsr
  mov %r13, %rdx
  mov %r12, %rax

  # Restore kernel FP settings
  fldcw (FPU_INIT)
  ldmxcsr (SSE_INIT)

  # Let's see if the syscall is within the valid range:
  mov syscall_table_size, %r12
  cmp %r12, %rax
  jae .LoutOfRange

  # It is, so let's make sure that the entry exists in the table:
  mov  $syscall_table,  %r12
  mov  (%r12, %rax, 8), %r12
  test %r12, %r12
  jz   .LnotInTable

  # Excellent.
  #
  # RCX is clobbered by syscall, but the ABI dictates the calling sequence as:
  #   RDI, RSI, RDX, RCX, R8, R9
  # which means that we need to use something else. So we'll use R10, because
  # that's what Linux uses.
  mov %r10, %rcx

  # Now, let's prepare the stack to call:
  mov %rsp, %rbp
  and $-16, %rsp

  # And do the call.
  call *%r12

  # Reset the stack.
  mov %rbp, %rsp

.Lsysret:
  # Now we can return to the system. The result should be in RAX, so don't touch
  # that. First, let's save the stack:
  mov %rsp, tss_stack

  mov %gs:CPULOCAL_HWSTATE, %r15
  mov %rbp, HWSTATE_KERN_RBP(%r15)
  mov %rsp, HWSTATE_KERN_RSP(%r15)

  # Add the registers offset to R15 for when we save those.
  add $HWSTATE_USER_REGISTERS, %r15

  # Restore the FXSAVE data.
  fxrstor64 0x90(%r15)

  # Set up the segment registers.
  mov $USER_DATA_SEL, %bx
  mov %bx, %ds
  mov %bx, %es
  swapgs

  # From now on we have to use SS. Now other registers:
  mov %ss:0x80(%r15), %rcx  # RIP
  mov %ss:0x88(%r15), %r11d # EFLAGS

  mov %ss:0x10(%r15), %rdx
  mov %ss:0x18(%r15), %rbx
  mov %ss:0x20(%r15), %rsp
  mov %ss:0x28(%r15), %rbp
  mov %ss:0x30(%r15), %rsi
  mov %ss:0x38(%r15), %rdi
  mov %ss:0x40(%r15), %r8
  mov %ss:0x48(%r15), %r9
  mov %ss:0x50(%r15), %r10

  mov %ss:0x60(%r15), %r12
  mov %ss:0x68(%r15), %r13
  mov %ss:0x70(%r15), %r14
  mov %ss:0x78(%r15), %r15

  # Make sure interrupts are enabled in the user process's EFLAGS.
  or  $(1 << 9),      %r11d

  # Everything should be in order now. Return to the user process.
  sysretq

.LoutOfRange:
  mov %rax,  %r15
  mov $-256, %rax
  jmp .Lsysret

.LnotInTable:
  mov %rax,  %r15
  mov $-257, %rax
  jmp .Lsysret
