/*******************************************************************************
 *
 * kit/kernel/interrupt_8259pic.c
 * - 8259 PIC driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "interrupt_8259pic.h"
#include "debug.h"

void interrupt_8259pic_remap(uint8_t master_index, uint8_t slave_index)
{
  uint8_t master_original_mask, slave_original_mask;

  // Save the original masks, which is the current value in the data port.
  master_original_mask = inb(_8259_MASTER_DATA);
  slave_original_mask  = inb(_8259_SLAVE_DATA);

  // [ICW1] Start initialization in cascade mode.
  outb(_8259_ICW1_INIT | _8259_ICW1_ICW4, _8259_MASTER_COMMAND);
  outb(_8259_ICW1_INIT | _8259_ICW1_ICW4, _8259_SLAVE_COMMAND);

  // [ICW2] Program master and slave with their respective interrupt index
  // offsets.
  outb(master_index, _8259_MASTER_DATA);
  outb(slave_index,  _8259_SLAVE_DATA);

  // [ICW3] Set master -> slave cascade, and slave identity.
  outb(0x4,          _8259_MASTER_DATA); // 0000 0100 = IRQ2
  outb(0x2,          _8259_SLAVE_DATA);  // Identity 2

  // [ICW4] Put the PICs back in 8086 mode.
  outb(_8259_ICW4_8086,      _8259_MASTER_DATA);
  outb(_8259_ICW4_8086,      _8259_SLAVE_DATA);

  // Restore the saved masks now that we're back in 8086 mode.
  outb(master_original_mask, _8259_MASTER_DATA);
  outb(slave_original_mask,  _8259_SLAVE_DATA);
}

void interrupt_8259pic_set_all_irq_masks(bool masked)
{
  outb(masked ? 0xff : 0x00, _8259_MASTER_DATA);
  outb(masked ? 0xff : 0x00, _8259_SLAVE_DATA);
}

void interrupt_8259pic_set_irq_mask(uint8_t irq, bool masked)
{
  uint16_t port;
  uint8_t value;

  // IRQs less than 8 are handled by the master, and greater than 8 are handled
  // by the slave.
  if (irq < 8)
  {
    port = _8259_MASTER_DATA;
  }
  else
  {
    port = _8259_SLAVE_DATA;
    irq -= 8;
  }

  // Modify the mask accordingly.
  if (masked)
  {
    value = inb(port) | (1 << irq);  // set bit
  }
  else
  {
    value = inb(port) & ~(1 << irq); // clear bit
  }

  // Send the new mask to the PIC.
  outb(value, port);
}

static inline uint16_t interrupt_8259pic_get_irq_register(uint8_t ocw3)
{
  // Get the register values (via the provided OCW3 command) from the master and
  // slave PICs and concatenate them.
  outb(ocw3, _8259_MASTER_COMMAND);
  outb(ocw3, _8259_SLAVE_COMMAND);

  return (inb(_8259_SLAVE_COMMAND) << 8) | inb(_8259_MASTER_COMMAND);
}

uint16_t interrupt_8259pic_get_irr()
{
  return interrupt_8259pic_get_irq_register(_8259_CMD_READ_IRR);
}

uint16_t interrupt_8259pic_get_isr()
{
  return interrupt_8259pic_get_irq_register(_8259_CMD_READ_ISR);
}
