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

.global bit_xor
bit_xor: # ( n m -- n^m )
  mov 0(%rbx), %rax
  mov 8(%rbx), %rcx
  xor %rax, %rcx

  add $8, %rbx
  mov %rcx, (%rbx)

  NEXT

.global equal
equal: # ( x y -- flag )
  mov 0(%rbx), %rax
  mov 8(%rbx), %rcx
  xor %rdx, %rdx # false
  cmp %rax, %rcx
  jne .Lne
  not %rdx # true
.Lne:
  add $8, %rbx
  mov %rdx, (%rbx)
  NEXT

.global fetch
fetch: # ( addr -- value ); fetch value in addr
  mov (%rbx), %rsi
  mov (%rsi), %rax
  mov %rax, (%rbx)
  NEXT

.global store
store: # ( value addr -- ); store value in addr
  mov 0(%rbx), %rdi
  mov 8(%rbx), %rax
  add $16, %rbx
  mov %rax, (%rdi)
  NEXT

.global dup
dup: # ( x -- x x )
  mov (%rbx), %rax
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global swap
swap: # ( x y -- y x )
  mov 0(%rbx), %rax
  mov 8(%rbx), %rcx
  mov %rcx, 0(%rbx)
  mov %rax, 8(%rbx)
  NEXT

.global over
over: # ( x y -- x y x )
  mov 8(%rbx), %rax
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global rot
rot: # ( x y z -- y z x )
  mov 0x10(%rbx), %rax # x
  mov 0x08(%rbx), %rcx # y
  mov 0x00(%rbx), %rdx # z
  mov %rcx, 0x10(%rbx) # y
  mov %rdx, 0x08(%rbx) # z
  mov %rax, 0x00(%rbx) # x
  NEXT

.global drop
drop: # ( x -- )
  add $8, %rbx
  NEXT

.global to_rstack
to_rstack: # ( x -- ); push value to return stack
  mov (%rbx), %rax
  add $8, %rbx
  push %rax
  NEXT

.global from_rstack
from_rstack: # ( -- x ); pop value from return stack
  pop %rax
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global fetch_rstack
fetch_rstack: # ( -- x ); copy value from return stack
  mov (%rsp), %rax
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global cp_stub
cp_stub: # ( -- addr ); push the end of the current compilation
  call cp
  sub $8, %rbx
  mov %rax, (%rbx)
  NEXT

.global cp_comma_stub
cp_comma_stub: # ( x -- ); write to the current compilation, advancing CP
  mov (%rbx), %rdi
  add $8, %rbx
  call cp_comma
  NEXT

.global branch
branch: # ( -- ); set ip to following number
  mov 8(%rbp), %rbp
  jmp *(%rbp)

.global branch_if_zero
branch_if_zero: # ( flag -- ); set ip to following number if flag is zero
  mov 8(%rbp), %rax
  mov (%rbx), %rcx
  add $8, %rbx

  test %rcx, %rcx
  jz .Lzero

  add $16, %rbp
  jmp *(%rbp)
.Lzero:
  mov %rax, %rbp
  jmp *(%rbp)

.global display
display: # ( x -- )
  mov (%rbx), %rdi
  add $8, %rbx
  call printi64
  NEXT

.global emit
emit: # ( char -- ); print single char
  mov (%rbx), %rdi
  add $8, %rbx
  call putchar
  NEXT

.global in_char
in_char: # ( -- char ); parse next char and put it on the stack
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
postpone_stub: # compile the compilation semantics of a word
  call postpone
  NEXT

.global postponed
postponed: # compile the compilation semantics of a postponed word
  mov 8(%rbp), %rdi
  mov %rbx, dp
  call compile_dict_entry
  mov dp, %rbx
  add $16, %rbp
  jmp *(%rbp)

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

.global parse_stub
parse_stub:
  mov (%rbx), %rdi
  mov %rbx, %rsi
  call parse
  sub $8, %rbx
  mov %rax, (%rbx)
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
