################################################################################
#
# kit/kernel/interrupt_isr_stub.S
# - assembly ISR stubs
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
################################################################################

.section .text

# GDT-related constants
.set GDT64_CODE_SEL, 0x08
.set GDT64_DATA_SEL, 0x10

# Emulates pusha on x86_64
.macro PUSHAQ
  push %rax
  push %rbx
  push %rcx
  push %rdx

  push %rsi
  push %rdi
  push %rbp
  push %rsp

  push %r8
  push %r9
  push %r10
  push %r11
  push %r12
  push %r13
  push %r14
  push %r15
.endm

# Emulates popa on x86_64
.macro POPAQ
  pop %r15
  pop %r14
  pop %r13
  pop %r12
  pop %r11
  pop %r10
  pop %r9
  pop %r8

  pop %rsp
  pop %rbp
  pop %rdi
  pop %rsi

  pop %rdx
  pop %rcx
  pop %rbx
  pop %rax
.endm

# Defines an interrupt stub, which just jumps to the common stub
# This variant expects that an error code is present
.macro ISR_ERRCODE index
.global interrupt_isr_stub_\index
interrupt_isr_stub_\index:
  # Disable interrupts
  cli

  # Put interrupt number on the stack
  pushq $\index

  # Go to common code
  jmp interrupt_isr_stub_common
.endm

# Defines an interrupt stub, which just jumps to the common stub
# This variant expects that an error code is NOT present (most common)
.macro ISR_NOERRCODE index
.global interrupt_isr_stub_\index
interrupt_isr_stub_\index:
  # Disable interrupts
  cli

  # Put dummy error code on the stack
  pushq $0

  # Put interrupt number on the stack
  pushq $\index

  # Go to common code
  jmp interrupt_isr_stub_common
.endm

ISR_NOERRCODE 0
ISR_NOERRCODE 1
ISR_NOERRCODE 2
ISR_NOERRCODE 3
ISR_NOERRCODE 4
ISR_NOERRCODE 5
ISR_NOERRCODE 6
ISR_NOERRCODE 7
ISR_ERRCODE   8  # Double fault
ISR_NOERRCODE 9
ISR_ERRCODE   10 # Invalid TSS
ISR_NOERRCODE 11
ISR_NOERRCODE 12
ISR_NOERRCODE 13
ISR_ERRCODE   14 # Page fault
ISR_NOERRCODE 15
ISR_NOERRCODE 16
ISR_NOERRCODE 17
ISR_NOERRCODE 18
ISR_NOERRCODE 19
ISR_NOERRCODE 20
ISR_NOERRCODE 21
ISR_NOERRCODE 22
ISR_NOERRCODE 23
ISR_NOERRCODE 24
ISR_NOERRCODE 25
ISR_NOERRCODE 26
ISR_NOERRCODE 27
ISR_NOERRCODE 28
ISR_NOERRCODE 29
ISR_NOERRCODE 30
ISR_NOERRCODE 31

interrupt_isr_stub_common:
  # Save general purpose registers
  PUSHAQ

  # Save data selector
  mov %ds, %ax
  push %rax

  # Load kernel data selector
  mov $GDT64_DATA_SEL, %ax
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs

  call interrupt_handler

  # Reload data selector
  pop %rax
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs

  # Restore general purpose registers
  POPAQ

  # Clean up space used by interrupt number and error code
  add $16, %rsp

  # (future) Re-enable interrupts
  # TODO: handle IRQ
  #sti

  # Return from interrupt
  iretq
