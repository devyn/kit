.section .text

.global _bootstrap64
.type _bootstrap64,@function
_bootstrap64:
  # Make sure interrupts are cleared
  cli

  # Meh, let's just call kernel_main
  call kernel_main

.Lhalt:
  # Halt once kernel_main returns.
  cli
  hlt
  jmp .Lhalt
