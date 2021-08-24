################################################################################
#
# kit/kernel/boot64.S
# - post-boot32 long mode stub which calls the kernel
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

# Must be pointed to by %fs:0x70 (still used?)
.global rust_stack_limit
rust_stack_limit: .quad (KERNEL_VMA + stack_bottom)

.section .text

.global _bootstrap64
.type _bootstrap64,@function
_bootstrap64:
  # Make sure interrupts are cleared
  cli

  # Load data selector into segment registers.
  movw $KERNEL_DATA_SEL, %ax
  movw %ax, %ds
  movw %ax, %es
  movw %ax, %ss

  # Add KERNEL_VMA to the GDT's base
  movabs $KERNEL_VMA, %rax
  mov (gdt64_pointer + 2), %rbx
  add %rax, %rbx
  mov %rbx, (gdt64_pointer + 2)

  # LGDT again to get the full 64 bits
  lgdt gdt64_pointer

  # Set CS and far return
  # Workaround for x86_64, which doesn't have a far jump that sets CS
  # So we set up a fake stack frame
  pushq $0
  pushq $KERNEL_CODE_SEL
  pushq $.LafterLGDT
  lretq

.LafterLGDT:
  # Initialize TSS segment.
  mov $(KERNEL_VMA + gdt64 + KERNEL_TSS_SEL), %rbx
  mov $(KERNEL_VMA + tss64),                  %rax

  mov %ax,   2(%rbx) # Base (15:0)
  shr $16,     %rax
  mov %al,   4(%rbx) # Base (23:16)
  shr $8,      %rax
  mov %al,   7(%rbx) # Base (31:24)
  shr $8,      %rax
  mov %eax,  8(%rbx) # Base (63:32)

  mov $tss64_limit, %rax
  mov %ax,         (%rbx) # Limit (15:0)
  shr $16,          %rax
  and $0x0f,        %al
  mov 6(%rbx),      %dl
  and $0xf0,        %dl
  or  %dl,          %al
  mov %al,        6(%rbx) # Limit (19:16)

  # Load task register.
  xor %eax, %eax
  mov $KERNEL_TSS_SEL, %ax
  ltr %ax

  # Initialize FPU.
  movq %cr0, %rax
  andq $0xfffffffffffffff3, %rax # Clear CR0.TS and CR0.EM
  orq  $0x2,  %rax # Set CR0.MP
  movq %rax, %cr0

  fldcw (FPU_INIT)

  # Initialize SSE.
  movq %cr4, %rax
  orq  $0x600, %rax # Set CR4.OSFXSR and CR4.OSXMMEXCPT
  movq %rax, %cr4

  ldmxcsr (SSE_INIT)

  # Initialize FS.base.
  mov    $IA32_FS_BASE, %ecx
  movabs $(rust_stack_limit - 0x70), %rdx
  mov    %edx, %eax
  shr    $32, %rdx
  wrmsr

  # Initialize GS.base
  mov    $IA32_GS_BASE, %ecx
  movabs $CPU0_LOCAL_DATA, %rdx
  mov    %edx, %eax
  shr    $32, %rdx
  wrmsr

  # Align the stack to 16 bytes (calling convention).
  andq $-16, %rsp

  # Add the new offset to the stack.
  movabs $KERNEL_VMA, %rax
  addq %rax, %rbp
  addq %rax, %rsp

  # Call kernel_main
  movabs $kernel_main, %rbx
  callq *%rbx

.Lhalt:
  # Halt once kernel_main returns.
  cli
  hlt
  jmp .Lhalt
