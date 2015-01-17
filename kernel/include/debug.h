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

#include <stdint.h>

#include "terminal.h"

#define DEBUG_MESSAGE(message) \
  __debug_message((message), __FILE__, __LINE__)

static inline void __debug_message(const char *message, const char *file,
  int line)
{
  terminal_writestring(file);
  terminal_writechar(':');
  terminal_writeuint64(line, 10);
  terminal_writestring(": ");
  terminal_writestring(message);
  terminal_writechar('\n');
}

#define DEBUG_MESSAGE_HEX(message, value) \
  __debug_message_hex((message), (uint64_t) (value), __FILE__, __LINE__)

static inline void __debug_message_hex(const char *message, uint64_t value,
  const char *file, int line)
{
  terminal_writestring(file);
  terminal_writechar(':');
  terminal_writeuint64(line, 10);
  terminal_writestring(": ");
  terminal_writestring(message);
  terminal_writestring(" (0x");
  terminal_writeuint64(value, 16);
  terminal_writestring(")\n");
}

#define DEBUG_BEGIN_VALUES() \
  __debug_begin_values(__FILE__, __LINE__)

static inline void __debug_begin_values(const char *file, int line)
{
  terminal_writestring(file);
  terminal_writechar(':');
  terminal_writeuint64(line, 10);
  terminal_writestring(": ");
}

#define DEBUG_HEX(value) \
  terminal_writestring(#value); \
  terminal_writestring("=0x"); \
  terminal_writeuint64((uint64_t) (value), 16); \
  terminal_writechar(' ')

#define DEBUG_DEC(value) \
  terminal_writestring(#value); \
  terminal_writestring("="); \
  terminal_writeuint64((uint64_t) (value), 10); \
  terminal_writechar(' ')

#define DEBUG_END_VALUES() \
  terminal_writechar('\n')

#endif
