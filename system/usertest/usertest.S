.section .rodata

otherdata: .asciz "Test string, testing"

.section .data

.align 8
xorval1: .quad 0xffff

.section .bss

.align 8
xorval2: .skip 8

.section .text

.global _start

_start:
  movq (xorval1), %rbx
  movq %rbx, (xorval2)
  movq (xorval2), %rcx
  xorq %rcx, %rax
  syscall
