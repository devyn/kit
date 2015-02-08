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

.section .bss

.set KERNEL_DATA_SEL, 0x10
.set   USER_DATA_SEL, 0x23

rax_temp: .skip 8

.section .text

.global syscall_handler
syscall_handler:
  # Temporarily save RAX in our temp space. We have to use SS for now.
  mov %rax, %ss:rax_temp

  # Set the data segments for the kernel (SS is already set).
  mov $KERNEL_DATA_SEL, %ax
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs

  # Load the current process's registers' base address into RAX.
  mov process_current,                  %rax
  add SYSCALL_OFFSET_PROCESS_REGISTERS, %rax

  # Save the process's register set.
  # Clear RCX and R11 (they are clobbered by syscalls).
  movq $0,    0x08(%rax)
  mov  %rdx,  0x10(%rax)
  mov  %rbx,  0x18(%rax)
  mov  %rsp,  0x20(%rax)
  mov  %rbp,  0x28(%rax)
  mov  %rsi,  0x30(%rax)
  mov  %rdi,  0x38(%rax)
  mov  %r8,   0x40(%rax)
  mov  %r9,   0x48(%rax)
  mov  %r10,  0x50(%rax)
  movq $0,    0x58(%rax)
  mov  %r12,  0x60(%rax)
  mov  %r13,  0x68(%rax)
  mov  %r14,  0x70(%rax)
  mov  %r15,  0x78(%rax)
  mov  %rcx,  0x80(%rax) # RCX  -> RIP
  mov  %r11d, 0x88(%rax) # R11D -> EFLAGS

  # Reload our old RAX into R12 now that we've cleared it up, and then save it.
  mov rax_temp, %r12
  mov %r12,     0x00(%rax)

  # Load the stack for ring 0 from the TSS.
  mov tss_stack, %rsp

  # If R12 (syscall no.) is zero, this is SYSCALL_EXIT which we emulate right
  # here.
  test %r12, %r12
  jz syscall_handler_exit

  # Otherwise, let's see if the syscall is within the valid range:
  mov syscall_table_size, %r13
  cmp %r13, %r12
  jae .LoutOfRange

  # It is, so let's make sure that the entry exists in the table:
  mov  $syscall_table,  %r13
  mov  (%r13, %r12, 8), %r13
  test %r13, %r13
  jz   .LnotInTable

  # Excellent.
  #
  # RCX is clobbered by syscall, but the ABI dictates the calling sequence as:
  #   RDI, RSI, RDX, RCX, R8, R9
  # which means that we need to use something else. So we'll use R10, because
  # that's what Linux uses.
  mov %r10, %rcx

  # Let's save RAX (process registers) because we'll want it after.
  push %rax

  # Now, let's prepare the stack to call:
  mov %rsp, %rbp
  and $-16, %rsp

  # And do the call.
  call *%r13

  # Reset the stack.
  mov %rbp, %rsp

  # Load process registers into R15.
  pop %r15

.Lsysret:
  # Now we can return to the system. The result should be in RAX, so don't touch
  # that. First, let's save the stack:
  mov %rsp, tss_stack

  # Set up the segment registers.
  mov $USER_DATA_SEL, %bx
  mov %bx, %ds
  mov %bx, %es
  mov %bx, %fs
  mov %bx, %gs

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

syscall_handler_exit:
  # Return to kernel.
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %rbx
  pop %rbp
  xor %rax, %rax
  sti
  retq
