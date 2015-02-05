################################################################################
#
# kit/system/stub.S
# - includes an entry point which calls C main() and then exits
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

.global _start
_start:
  mov %r8, %rdi # argc
  mov %r9, %rsi # argv

  # int main(int argc, char **argv)
  movabs $main, %rcx
  callq *%rcx

  mov %rax, %rdi # exit code
  mov $SYSCALL_EXIT, %rax
  syscall

  # should not return
  hlt

.section .note.GNU-stack,"",%progbits
