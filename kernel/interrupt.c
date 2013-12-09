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
#include "interrupt_8259pic.h"
#include "memory.h"
#include "debug.h"

// Static definitions & function prototypes
static interrupt_gate_t *interrupt_table;

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
EXTERN_INTERRUPT_ISR_STUB(32)
EXTERN_INTERRUPT_ISR_STUB(33)
EXTERN_INTERRUPT_ISR_STUB(34)
EXTERN_INTERRUPT_ISR_STUB(35)
EXTERN_INTERRUPT_ISR_STUB(36)
EXTERN_INTERRUPT_ISR_STUB(37)
EXTERN_INTERRUPT_ISR_STUB(38)
EXTERN_INTERRUPT_ISR_STUB(39)
EXTERN_INTERRUPT_ISR_STUB(40)
EXTERN_INTERRUPT_ISR_STUB(41)
EXTERN_INTERRUPT_ISR_STUB(42)
EXTERN_INTERRUPT_ISR_STUB(43)
EXTERN_INTERRUPT_ISR_STUB(44)
EXTERN_INTERRUPT_ISR_STUB(45)
EXTERN_INTERRUPT_ISR_STUB(46)
EXTERN_INTERRUPT_ISR_STUB(47)
EXTERN_INTERRUPT_ISR_STUB(48)
EXTERN_INTERRUPT_ISR_STUB(49)
EXTERN_INTERRUPT_ISR_STUB(50)
EXTERN_INTERRUPT_ISR_STUB(51)
EXTERN_INTERRUPT_ISR_STUB(52)
EXTERN_INTERRUPT_ISR_STUB(53)
EXTERN_INTERRUPT_ISR_STUB(54)
EXTERN_INTERRUPT_ISR_STUB(55)
EXTERN_INTERRUPT_ISR_STUB(56)
EXTERN_INTERRUPT_ISR_STUB(57)
EXTERN_INTERRUPT_ISR_STUB(58)
EXTERN_INTERRUPT_ISR_STUB(59)
EXTERN_INTERRUPT_ISR_STUB(60)
EXTERN_INTERRUPT_ISR_STUB(61)
EXTERN_INTERRUPT_ISR_STUB(62)
EXTERN_INTERRUPT_ISR_STUB(63)

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
  MAP_INTERRUPT(32);
  MAP_INTERRUPT(33);
  MAP_INTERRUPT(34);
  MAP_INTERRUPT(35);
  MAP_INTERRUPT(36);
  MAP_INTERRUPT(37);
  MAP_INTERRUPT(38);
  MAP_INTERRUPT(39);
  MAP_INTERRUPT(40);
  MAP_INTERRUPT(41);
  MAP_INTERRUPT(42);
  MAP_INTERRUPT(43);
  MAP_INTERRUPT(44);
  MAP_INTERRUPT(45);
  MAP_INTERRUPT(46);
  MAP_INTERRUPT(47);
  MAP_INTERRUPT(48);
  MAP_INTERRUPT(49);
  MAP_INTERRUPT(50);
  MAP_INTERRUPT(51);
  MAP_INTERRUPT(52);
  MAP_INTERRUPT(53);
  MAP_INTERRUPT(54);
  MAP_INTERRUPT(55);
  MAP_INTERRUPT(56);
  MAP_INTERRUPT(57);
  MAP_INTERRUPT(58);
  MAP_INTERRUPT(59);
  MAP_INTERRUPT(60);
  MAP_INTERRUPT(61);
  MAP_INTERRUPT(62);
  MAP_INTERRUPT(63);

  // Set IDT register.
  interrupt_idtr_t idtr;

  idtr.base_address = (uint64_t) interrupt_table;
  idtr.limit        = INTERRUPT_TABLE_SIZE - 1;

  lidt(&idtr); // x86_64 instruction

  // Only the 8259 PIC is supported for now. I/O & local APIC support planned.
  // Initialize the 8259 by remapping it.
  interrupt_8259pic_remap(INTERRUPT_INDEX_IRQ, INTERRUPT_INDEX_IRQ + 8);

  // For now, mask every interrupt except the keyboard.
  interrupt_8259pic_set_all_irq_masks(true);
  interrupt_8259pic_set_irq_mask(1, false);
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
/*
  DEBUG_BEGIN_VALUES();
    DEBUG_HEX(stack.ds);
    DEBUG_HEX(stack.r15);
    DEBUG_HEX(stack.r14);
    DEBUG_HEX(stack.r13);
    DEBUG_HEX(stack.r12);
    DEBUG_HEX(stack.r11);
    DEBUG_HEX(stack.r10);
    DEBUG_HEX(stack.r9);
    DEBUG_HEX(stack.r8);
    DEBUG_HEX(stack.rsp);
    DEBUG_HEX(stack.rbp);
    DEBUG_HEX(stack.rdi);
    DEBUG_HEX(stack.rsi);
    DEBUG_HEX(stack.rdx);
    DEBUG_HEX(stack.rcx);
    DEBUG_HEX(stack.rbx);
    DEBUG_HEX(stack.rax);
    DEBUG_HEX(stack.index);
    DEBUG_HEX(stack.err_code);
    DEBUG_HEX(stack.rip);
    DEBUG_HEX(stack.cs);
    DEBUG_HEX(stack.rflags);
    DEBUG_HEX(stack.user_rsp);
    DEBUG_HEX(stack.ss);
  DEBUG_END_VALUES();
*/

  uint8_t key;

  switch (stack.index)
  {
    case INTERRUPT_INDEX_IRQ + 1:
      key = inb(0x60);

      DEBUG_MESSAGE_HEX("keyboard handler invoked", key);

      interrupt_irq_done(1);
      break;
    default:
      DEBUG_MESSAGE_HEX("interrupt not implemented", stack.index);
  }
}

void interrupt_irq_done(uint8_t irq)
{
  interrupt_8259pic_send_master_eoi();
  if (irq >= 8) {
    interrupt_8259pic_send_slave_eoi();
  }
}

bool interrupt_handle_spurious_irq7()
{
  uint16_t isr = interrupt_8259pic_get_isr();

  if (isr & 7)
  {
    // Not spurious.
    return false;
  }
  else
  {
    // Spurious.
    return true;
  }
}

bool interrupt_handle_spurious_irq15()
{
  uint16_t isr = interrupt_8259pic_get_isr();

  if (isr & 15)
  {
    // Not spurious.
    return false;
  }
  else
  {
    // Spurious. In the case of spurious IRQ 15, we also have to send
    // end-of-interrupt to the master PIC, since it doesn't know that the IRQ
    // received from the slave was spurious.
    interrupt_8259pic_send_master_eoi();
    return true;
  }
}
