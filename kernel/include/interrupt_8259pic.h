/*******************************************************************************
 *
 * kit/kernel/include/interrupt_8259pic.h
 * - 8259 PIC driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef INTERRUPT_8259PIC_H
#define INTERRUPT_8259PIC_H

#include <stdint.h>
#include <stdbool.h>

#include "x86_64.h"

// 8259 PIC port definitions
#define _8259_MASTER         0x20
#define _8259_SLAVE          0xA0
#define _8259_MASTER_COMMAND  _8259_MASTER
#define _8259_MASTER_DATA    (_8259_MASTER + 1)
#define _8259_SLAVE_COMMAND   _8259_SLAVE
#define _8259_SLAVE_DATA     (_8259_SLAVE + 1)

// 8259 command definitions
#define _8259_CMD_READ_IRR 0x0a
#define _8259_CMD_READ_ISR 0x0b
#define _8259_CMD_EOI      0x20

// Initialization command 1: initialize
#define _8259_ICW1_INIT 0x10

// Initialization command 1: ICW4 required
#define _8259_ICW1_ICW4 0x01

// Initialization command 4: 8086 mode
#define _8259_ICW4_8086 0x01

/**
 * Remap the PICs to a master interrupt index offset and a slave interrupt index
 * offset.
 */
void interrupt_8259pic_remap(uint8_t master_index, uint8_t slave_index);

/**
 * Mask or unmask all IRQs.
 */
void interrupt_8259pic_set_all_irq_masks(bool masked);

/**
 * Mask or unmask a single IRQ.
 */
void interrupt_8259pic_set_irq_mask(uint8_t irq, bool masked);

/**
 * Get IRQ request register (IRR).
 */
uint16_t interrupt_8259pic_get_irr();

/**
 * Get in-service register (ISR).
 */
uint16_t interrupt_8259pic_get_isr();

/**
 * Send end-of-interrupt to master PIC.
 */
static inline void interrupt_8259pic_send_master_eoi()
{
  outb(_8259_CMD_EOI, _8259_MASTER_COMMAND);
}

/**
 * Send end-of-interrupt to slave PIC.
 */
static inline void interrupt_8259pic_send_slave_eoi()
{
  outb(_8259_CMD_EOI, _8259_SLAVE_COMMAND);
}

#endif
