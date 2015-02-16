################################################################################
#
# kit/system/crt0.S
# - C runtime entry point
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .text

.set SYSCALL_EXIT, 0x0

# %rdi: argc, %rsi: argv (SysV)
.global _start
_start:
  # Initialize the standard library.
  push %rdi
  push %rsi
  call libc_init
  pop  %rsi
  pop  %rdi

  # int main(int argc, char **argv)
  movabs $main, %rcx
  callq *%rcx

  # Exit implicitly.
  mov %rax, %rdi # exit code
  call exit

# In case the exit doesn't work, for some reason.
1:
  hlt
  jmp 1b

.section .note.GNU-stack,"",%progbits
