/*******************************************************************************
 *
 * kit/kernel/interrupt.c
 * - high level interface to processor interrupts
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "interrupt.h"
#include "memory.h"
#include "x86_64.h"
#include "terminal.h"

interrupt_gate_t *interrupt_table;

#define EXTERN_INTERRUPT_ISR_STUB(n) \
  extern void interrupt_isr_stub_##n();

EXTERN_INTERRUPT_ISR_STUB(0)
EXTERN_INTERRUPT_ISR_STUB(1)
EXTERN_INTERRUPT_ISR_STUB(2)
EXTERN_INTERRUPT_ISR_STUB(3)
EXTERN_INTERRUPT_ISR_STUB(4)
EXTERN_INTERRUPT_ISR_STUB(5)
EXTERN_INTERRUPT_ISR_STUB(6)
EXTERN_INTERRUPT_ISR_STUB(7)
EXTERN_INTERRUPT_ISR_STUB(8)
EXTERN_INTERRUPT_ISR_STUB(9)
EXTERN_INTERRUPT_ISR_STUB(10)
EXTERN_INTERRUPT_ISR_STUB(11)
EXTERN_INTERRUPT_ISR_STUB(12)
EXTERN_INTERRUPT_ISR_STUB(13)
EXTERN_INTERRUPT_ISR_STUB(14)
EXTERN_INTERRUPT_ISR_STUB(15)
EXTERN_INTERRUPT_ISR_STUB(16)
EXTERN_INTERRUPT_ISR_STUB(17)
EXTERN_INTERRUPT_ISR_STUB(18)
EXTERN_INTERRUPT_ISR_STUB(19)
EXTERN_INTERRUPT_ISR_STUB(20)
EXTERN_INTERRUPT_ISR_STUB(21)
EXTERN_INTERRUPT_ISR_STUB(22)
EXTERN_INTERRUPT_ISR_STUB(23)
EXTERN_INTERRUPT_ISR_STUB(24)
EXTERN_INTERRUPT_ISR_STUB(25)
EXTERN_INTERRUPT_ISR_STUB(26)
EXTERN_INTERRUPT_ISR_STUB(27)
EXTERN_INTERRUPT_ISR_STUB(28)
EXTERN_INTERRUPT_ISR_STUB(29)
EXTERN_INTERRUPT_ISR_STUB(30)
EXTERN_INTERRUPT_ISR_STUB(31)

void interrupt_initialize()
{
  // Initialize memory for IDT.
  interrupt_table = memory_alloc_aligned(INTERRUPT_TABLE_SIZE, 4096);

  memory_clear(interrupt_table, INTERRUPT_TABLE_SIZE);

  // Initialize IDT by setting mapping the first 32 gates.
  // This should be changed if INTERRUPT_TABLE_ENTRIES changes.
  // We use a macro to do the heavy lifting.

#define MAP_INTERRUPT(n) \
  interrupt_set_gate(n, (uintptr_t) &interrupt_isr_stub_##n, \
    GDT_SEL_KERNEL_CODE, INTERRUPT_TYPE_NORMAL, GDT_PRIVILEGE_KERNEL)

  MAP_INTERRUPT(0);
  MAP_INTERRUPT(1);
  MAP_INTERRUPT(2);
  MAP_INTERRUPT(3);
  MAP_INTERRUPT(4);
  MAP_INTERRUPT(5);
  MAP_INTERRUPT(6);
  MAP_INTERRUPT(7);
  MAP_INTERRUPT(8);
  MAP_INTERRUPT(9);
  MAP_INTERRUPT(10);
  MAP_INTERRUPT(11);
  MAP_INTERRUPT(12);
  MAP_INTERRUPT(13);
  MAP_INTERRUPT(14);
  MAP_INTERRUPT(15);
  MAP_INTERRUPT(16);
  MAP_INTERRUPT(17);
  MAP_INTERRUPT(18);
  MAP_INTERRUPT(19);
  MAP_INTERRUPT(20);
  MAP_INTERRUPT(21);
  MAP_INTERRUPT(22);
  MAP_INTERRUPT(23);
  MAP_INTERRUPT(24);
  MAP_INTERRUPT(25);
  MAP_INTERRUPT(26);
  MAP_INTERRUPT(27);
  MAP_INTERRUPT(28);
  MAP_INTERRUPT(29);
  MAP_INTERRUPT(30);
  MAP_INTERRUPT(31);

  // Set IDT register.
  interrupt_idtr_t idtr;

  idtr.base_address = (uint64_t) interrupt_table;
  idtr.limit        = INTERRUPT_TABLE_SIZE - 1;

  lidt(idtr); // x86_64 instruction
}

void interrupt_enable()
{
  sti();
}

void interrupt_disable()
{
  cli();
}

void interrupt_set_gate(uint8_t index, uintptr_t routine_address,
  gdt_selector_t selector, interrupt_type_t type, gdt_privilege_t privilege)
{
  // Routine address is split into bits: (16, 16, 32) = 64 bits.

  interrupt_table[index].base_low  =  routine_address        & 0xFFFF;
  interrupt_table[index].base_mid  = (routine_address >> 16) & 0xFFFF;
  interrupt_table[index].base_high = (routine_address >> 32) & 0xFFFFFFFF;

  interrupt_table[index].segment   = selector;
  interrupt_table[index].type      = type;
  interrupt_table[index].privilege = privilege;

  interrupt_table[index].present   = 1;
}

/**
 * Called from interrupt_isr_stub_common.
 */
void interrupt_handler(interrupt_stack_t stack) {
  terminal_writestring("!! interrupt not implemented: 0x");
  terminal_writeuint64(stack.index, 16);
  terminal_putchar('\n');

#define DEBUG(str, value) \
  terminal_writestring(str); \
  terminal_writestring("0x"); \
  terminal_writeuint64((value), 16); \
  terminal_putchar(' ');

  DEBUG("ds=", stack.ds);
  DEBUG("r15=", stack.r15);
  DEBUG("r14=", stack.r14);
  DEBUG("r13=", stack.r13);
  DEBUG("r12=", stack.r12);
  DEBUG("r11=", stack.r11);
  DEBUG("r10=", stack.r10);
  DEBUG("r9=", stack.r9);
  DEBUG("r8=", stack.r8);
  DEBUG("rsp=", stack.rsp);
  DEBUG("rbp=", stack.rbp);
  DEBUG("rdi=", stack.rdi);
  DEBUG("rsi=", stack.rsi);
  DEBUG("rdx=", stack.rdx);
  DEBUG("rcx=", stack.rcx);
  DEBUG("rbx", stack.rbx);
  DEBUG("rax=", stack.rax);
  DEBUG("index=", stack.index);
  DEBUG("err_code=", stack.err_code);
  DEBUG("rip=", stack.rip);
  DEBUG("cs=", stack.cs);
  DEBUG("rflags=", stack.rflags);
  DEBUG("user_rsp=", stack.user_rsp);
  DEBUG("ss=", stack.ss);

  terminal_putchar('\n');
}
