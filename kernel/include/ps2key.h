/*******************************************************************************
 *
 * kit/kernel/include/ps2key.h
 * - PS/2 keyboard driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PS2KEY_H
#define PS2KEY_H

#include <stdint.h>

/**
 * Initializes the PS/2 keyboard state machine.
 */
void ps2key_initialize();

void ps2key_handle_irq(uint8_t data);

#endif
