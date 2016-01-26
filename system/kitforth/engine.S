global:
################################################################################
#
# kit/system/kitforth/engine.S
# - kitFORTH low-level operations & execution engine
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .text

# Across jumps:
#
# %rsp = return (stack) pointer
# %rbp = instruction (array) pointer
# %rbx = data (stack) pointer

.global execute
execute:
  push %rbp
  push %rbx
  push %r12
  push %r13
  push %r14
  push %r15

  mov %rdi, %rbp # IP - instruction pointer
  mov %rsi, %rbx # DP - data pointer
  jmp *(%rbp)

.global push
push: # ( -- x ); ip+2 -> [PUSH, x, <ip> ...]
  mov 8(%rbp), %rax
  sub $8, %rbx
  mov %rax, (%rbx)

  add $16, %rbp
  jmp *(%rbp)

.global add
add: # ( n m -- n+m )
  mov 0(%rbx), %rax
  mov 8(%rbx), %rcx
  add %rax, %rcx

  add $8, %rbx
  mov %rcx, (%rbx)

  add $8, %rbp
  jmp *(%rbp)

.global dup
dup: # ( x -- x x )
  mov (%rbx), %rax
  sub $8, %rbx
  mov %rax, (%rbx)

  add $8, %rbp
  jmp *(%rbp)

.global display
display: # ( x -- )
  mov (%rbx), %rdi
  add $8, %rbx
  call printu64x

  add $8, %rbp
  jmp *(%rbp)

.global cr
cr: # ( -- ); print newline
  mov $0xA, %rdi
  call putchar

  add $8, %rbp
  jmp *(%rbp)

.global ret
ret:
  mov %rbx, %rax # return DP
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %rbx
  pop %rbp
  ret
