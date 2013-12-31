################################################################################
#
# kit/kernel/boot64.S
# - post-boot32 long mode stub which calls the kernel
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
################################################################################

.section .rodata

FPU_INIT_CONTROL: .word 0x37e # enable invalid operand exceptions

.section .text

# GDT-related constants
.set GDT64_CODE_SEL, 0x08
.set GDT64_DATA_SEL, 0x10

.global _bootstrap64
.type _bootstrap64,@function
_bootstrap64:
  # Make sure interrupts are cleared
  cli

  # Load data selector into segment registers.
  movw $GDT64_DATA_SEL, %ax
  movw %ax, %ds
  movw %ax, %es
  movw %ax, %fs
  movw %ax, %gs
  movw %ax, %ss

  # Initialize x87 FPU and SSE.
  movq %cr0, %rax
  andq $((-1) - ((1 << 2) + (1 << 1))), %rax # Clear CR0.TS and CR0.EM
  movq %rax, %cr0

  movq %cr4, %rax
  orq  $((1 << 8) + (1 << 9)), %rax # Set CR4.OSFXSR and CR4.OSXMMEXCPT
  movq %rax, %cr4

  fninit
  fldcw (FPU_INIT_CONTROL)

  # Call kernel_main
  movabs $kernel_main, %rbx
  callq *%rbx

.Lhalt:
  # Halt once kernel_main returns.
  cli
  hlt
  jmp .Lhalt
