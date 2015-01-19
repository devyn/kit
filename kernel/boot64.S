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

  # LGDT again to get the full 64 bits
  lgdt gdt64_pointer

  # Set CS and far return
  # Workaround for x86_64, which doesn't have a far jump that sets CS
  # So we set up a fake stack frame
  pushq $0
  pushq $GDT64_CODE_SEL
  pushq $.LafterLGDT
  lretq

.LafterLGDT:
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
