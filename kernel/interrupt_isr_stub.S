################################################################################
#
# kit/kernel/interrupt_isr_stub.S
# - assembly ISR stubs
#
# vim:ts=2:sw=2:et:tw=80:ft=asm
#
# Copyright (C) 2015, Devyn Cairns
# Redistribution of this file is permitted under the terms of the simplified BSD
# license. See LICENSE for more information.
#
################################################################################

.section .text

.set INTERRUPT_STACK_SIZE, 192

# GDT-related constants
.set KERNEL_CODE_SEL, 0x08
.set KERNEL_DATA_SEL, 0x10

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

ISR_NOERRCODE 0  # Divide by zero error
ISR_NOERRCODE 1  # Debug
ISR_NOERRCODE 2  # Non-maskable interrupt
ISR_NOERRCODE 3  # Breakpoint
ISR_NOERRCODE 4  # Overflow
ISR_NOERRCODE 5  # Bound range exceeded
ISR_NOERRCODE 6  # Invalid opcode
ISR_NOERRCODE 7  # Device not available
ISR_ERRCODE   8  # Double fault
ISR_NOERRCODE 9  # Coprocessor segment overrun
ISR_ERRCODE   10 # Invalid TSS
ISR_ERRCODE   11 # Segment not present
ISR_ERRCODE   12 # Stack segment fault
ISR_ERRCODE   13 # General protection fault
ISR_ERRCODE   14 # Page fault
ISR_NOERRCODE 15
ISR_NOERRCODE 16 # x87 floating point exception
ISR_ERRCODE   17 # Alignment check
ISR_NOERRCODE 18 # Machine check
ISR_NOERRCODE 19 # SIMD floating point exception
ISR_NOERRCODE 20 # Virtualization exception
ISR_NOERRCODE 21
ISR_NOERRCODE 22
ISR_NOERRCODE 23
ISR_NOERRCODE 24
ISR_NOERRCODE 25
ISR_NOERRCODE 26
ISR_NOERRCODE 27
ISR_NOERRCODE 28
ISR_NOERRCODE 29
ISR_ERRCODE   30 # Security exception
ISR_NOERRCODE 31
ISR_NOERRCODE 32
ISR_NOERRCODE 33
ISR_NOERRCODE 34
ISR_NOERRCODE 35
ISR_NOERRCODE 36
ISR_NOERRCODE 37
ISR_NOERRCODE 38
ISR_NOERRCODE 39
ISR_NOERRCODE 40
ISR_NOERRCODE 41
ISR_NOERRCODE 42
ISR_NOERRCODE 43
ISR_NOERRCODE 44
ISR_NOERRCODE 45
ISR_NOERRCODE 46
ISR_NOERRCODE 47
ISR_NOERRCODE 48
ISR_NOERRCODE 49
ISR_NOERRCODE 50
ISR_NOERRCODE 51
ISR_NOERRCODE 52
ISR_NOERRCODE 53
ISR_NOERRCODE 54
ISR_NOERRCODE 55
ISR_NOERRCODE 56
ISR_NOERRCODE 57
ISR_NOERRCODE 58
ISR_NOERRCODE 59
ISR_NOERRCODE 60
ISR_NOERRCODE 61
ISR_NOERRCODE 62
ISR_NOERRCODE 63

interrupt_isr_stub_common:
  # Save general purpose registers
  PUSHAQ

  # Save data selector
  mov %ds, %ax
  push %rax

  # Load kernel data selector
  mov $KERNEL_DATA_SEL, %ax
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs
  mov %ax, %ss # to replace the hacky NULL selector one

  # Align the stack
  mov %rsp, %rbp
  and $-16, %rsp

  # Execute interrupt handler
  call interrupt_handler

  # Reset the stack
  mov %rbp, %rsp

  # Load the old data selector
  pop %rax

  # If the data selector was not the kernel data selector, then we got here from
  # user code, which means we should save the stack as it will be once we return
  # to the TSS and to the current process.
  #
  # Otherwise, skip the following.
  mov $KERNEL_DATA_SEL, %bx
  cmp %ax, %bx
  je  1f

  # This is what the stack pointer will be after iret.
  lea (INTERRUPT_STACK_SIZE - 8)(%rsp), %rbx

  # Store this in the TSS.
  mov %rbx, tss_stack

  # And the process.
  mov process_current,                     %rdi
  add PROCESS_OFFSET_KERNEL_STACK_POINTER, %rdi

  mov %rbx, (%rdi)

1:
  # Restore the data selector
  mov %ax, %ds
  mov %ax, %es
  mov %ax, %fs
  mov %ax, %gs

  # Restore general purpose registers
  POPAQ

  # Clean up space used by interrupt number and error code
  add $16, %rsp

  # Return from interrupt
  iretq
