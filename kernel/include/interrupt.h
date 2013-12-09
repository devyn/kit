/*******************************************************************************
 *
 * kit/kernel/include/interrupt.h
 * - high level interface to processor interrupts
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef INTERRUPT_H
#define INTERRUPT_H

#include <stdint.h>
#include <stdbool.h>

#include "config.h"
#include "gdt.h"
#include "x86_64.h"

/**
 * x86_64 IDTR (IDT pointer).
 */
typedef struct PACKED interrupt_idtr
{
  uint16_t limit;
  uint64_t base_address;
} interrupt_idtr_t;

/**
 * x86_64 IDT entry.
 */
typedef struct PACKED interrupt_gate
{
  /* 16 bytes */
  unsigned int base_low     : 16; // byte: 0
  unsigned int segment      : 16; // byte: 2
  unsigned int stack_table  : 3;  // byte: 4
  unsigned int zero1        : 5;
  unsigned int type         : 4;  // byte: 5
  unsigned int zero2        : 1;
  unsigned int privilege    : 2;
  unsigned int present      : 1;
  unsigned int base_mid     : 16; // byte: 6
  unsigned int base_high    : 32; // byte: 8
  unsigned int zero3        : 32; // byte: 12
} interrupt_gate_t;

/**
 * x86_64 interrupt type.
 */
typedef enum interrupt_type
{
  INTERRUPT_TYPE_NORMAL = 0xE,
  INTERRUPT_TYPE_TRAP   = 0xF
} interrupt_type_t;

/**
 * The stack we get from interrupt_isr_stub_common.
 */
typedef struct interrupt_stack
{
  uint64_t ds;
  uint64_t r15, r14, r13, r12, r11, r10, r9, r8;
  uint64_t rsp, rbp, rdi, rsi, rdx, rcx, rbx, rax;
  uint64_t index, err_code;
  uint64_t rip, cs, rflags, user_rsp, ss;
} interrupt_stack_t;

/**
 * Number of entries in an interrupt table, and the size of one in bytes.
 */
#define INTERRUPT_TABLE_ENTRIES  64
#define INTERRUPT_TABLE_SIZE    (64 * sizeof(interrupt_gate_t))

/**
 * The index at which exceptions begin.
 */
#define INTERRUPT_INDEX_EXC 0

/**
 * The index at which IRQs begin.
 */
#define INTERRUPT_INDEX_IRQ 32

/**
 * Prepare the interrupt table and load it.
 */
void interrupt_initialize();

/**
 * Enable interrupts.
 * On x86_64, this is simply the sti (set interrupt flag) instruction.
 */
static inline void interrupt_enable()
{
  sti();
}

/**
 * Disable interrupts.
 * On x86_64, this is simply the cli (clear interrupt flag) instruction.
 */
static inline void interrupt_disable()
{
  cli();
}

/**
 * Set interrupt gate in interrupt table.
 */
void interrupt_set_gate(uint8_t index, uintptr_t routine_address,
  gdt_selector_t selector, interrupt_type_t type, gdt_privilege_t privilege);

/**
 * Acknowledge IRQ (called when handler is finished).
 */
void interrupt_irq_done(uint8_t irq);

/**
 * Returns true if the IRQ was spurious. Interrupt handler should return and do
 * nothing else.
 * Must be checked on IRQ 7, and only on IRQ 7.
 */
bool interrupt_handle_spurious_irq7();

/**
 * Returns true if the IRQ was spurious. Interrupt handler should return and do
 * nothing else.
 * Must be checked on IRQ 15, and only on IRQ 15.
 */
bool interrupt_handle_spurious_irq15();

#endif
