################################################################################
#
# kit/kernel/boot64.S
# - post-boot32 long mode stub which calls the kernel
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
################################################################################

.section .rodata

# FPU init word (div by zero + invalid operand cause exceptions)
FPU_INIT: .word 0x37a

# SSE init dword (same config)
SSE_INIT: .long 0x1d00

# GDT-related constants
.set KERNEL_CODE_SEL, 0x08
.set KERNEL_DATA_SEL, 0x10

.section .text

.global _bootstrap64
.type _bootstrap64,@function
_bootstrap64:
  # Make sure interrupts are cleared
  cli

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
  # Load data selector into segment registers.
  movw $KERNEL_DATA_SEL, %ax
  movw %ax, %ds
  movw %ax, %es
  movw %ax, %fs
  movw %ax, %gs
  movw %ax, %ss

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
