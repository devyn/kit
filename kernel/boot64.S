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

  # Call kernel_main
  movabs $kernel_main, %rbx
  callq *%rbx

.Lhalt:
  # Halt once kernel_main returns.
  cli
  hlt
  jmp .Lhalt
