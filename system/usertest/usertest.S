.global _start

_start:
  xorq $0xffff, %rax
  syscall
