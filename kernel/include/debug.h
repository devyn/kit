/*******************************************************************************
 *
 * kit/kernel/include/debug.h
 * - debug helper macros
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef DEBUG_H
#define DEBUG_H

#include "terminal.h"

#define DEBUG_MESSAGE_HEX(message, value) \
  terminal_writestring("W: "); \
  terminal_writestring(message); \
  terminal_writestring(" ("); \
  terminal_writeuint64((value), 16); \
  terminal_writestring(")\n")

#define DEBUG_HEX(value) \
  terminal_writestring(#value); \
  terminal_writestring("=0x"); \
  terminal_writeuint64((value), 16); \
  terminal_putchar(' ')

#define DEBUG_END_VALUES() \
  terminal_putchar('\n')

#endif
