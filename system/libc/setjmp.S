################################################################################
#
# kit/system/libc/setjmp.S
# - non-local gotos for C
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .text

.global setjmp
setjmp:
  # Save relevant registers (see System V ABI)
  mov %rbx, 0x00(%rdi)
  mov %rsp, 0x08(%rdi)
  mov %rbp, 0x10(%rdi)
  mov %r12, 0x18(%rdi)
  mov %r13, 0x20(%rdi)
  mov %r14, 0x28(%rdi)
  mov %r15, 0x30(%rdi)

  # Return zero
  xor %rax, %rax
  ret

.global longjmp
longjmp:
  # Restore relevant registers (see System V ABI)
  mov 0x00(%rdi), %rbx
  mov 0x08(%rdi), %rsp
  mov 0x10(%rdi), %rbp
  mov 0x18(%rdi), %r12
  mov 0x20(%rdi), %r13
  mov 0x28(%rdi), %r14
  mov 0x30(%rdi), %r15

  # Return value unless it is zero
  mov %rsi, %rax
  test %rax, %rax
  jnz 1f

  # If it is zero, return one
  mov $1, %rax
1:
  ret

.section .note.GNU-stack,"",%progbits
