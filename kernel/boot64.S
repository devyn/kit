.section .text

.global _bootstrap64
.type _bootstrap64,@function
_bootstrap64:
  # Make sure interrupts are cleared
  cli

  # Call kernel_main
  movabs $kernel_main, %rbx
  callq *%rbx

.Lhalt:
  # Halt once kernel_main returns.
  cli
  hlt
  jmp .Lhalt
