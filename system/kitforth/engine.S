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

.macro NEXT
  add $8, %rbp
  jmp *(%rbp)
.endm

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

  NEXT

.global dup
dup: # ( x -- x x )
  mov (%rbx), %rax
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global display
display: # ( x -- )
  mov (%rbx), %rdi
  add $8, %rbx
  call printu64x
  NEXT

.global emit
emit: # ( char -- ); print single char
  mov (%rbx), %rdi
  add $8, %rbx
  call putchar
  NEXT

.global in_char
in_char: # ( -- char); parse next char and put it on the stack
  call read_charword
  and  $0xFF, %rax

  sub $8, %rbx
  mov %rax, (%rbx)

  NEXT

.global compiler_off
compiler_off: # go to immediate/interpreted mode
  movl $0, compiling
  NEXT

.global compiler_on
compiler_on: # go to compiled mode
  movl $1, compiling
  NEXT

.global literal_stub
literal_stub: # ( x -- ); push literal to compiled code
  mov (%rbx), %rdi
  add $8, %rbx
  call literal
  NEXT

.global postpone_stub
postpone_stub: # get a stub and append its compilation semantics
  call postpone
  NEXT

.global immediate_stub
immediate_stub: # make the last defined word immediate
  call immediate
  NEXT

.global defword_stub
defword_stub:
  call defword
  NEXT

.global endword_stub
endword_stub:
  call endword
  NEXT

.global call
call: # push IP+2 on return stack, take new IP from IP+1
  lea 16(%rbp), %rax
  push %rax

  mov 8(%rbp), %rbp
  jmp *(%rbp)

.global ret
ret: # pop IP from return stack
  pop %rbp
  jmp *(%rbp)

.global ret_quit
ret_quit:
  mov %rbx, %rax # return DP
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %rbx
  pop %rbp
  ret
