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

  # Reload our old RAX into RDI now that we've cleared it up, and then save it.
  mov rax_temp, %rdi
  mov %rdi,     0x00(%rax)

  # Load the syscall stack.
  mov syscall_stack, %rsp

  # Make sure RAX (relocated to RDI) is zero (SYSCALL_EXIT).
  # TODO: more syscalls, and sysret
  test %rdi, %rdi
  jnz .Lnotzero

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

.Lnotzero:
  cli
  hlt
  jmp .Lnotzero
