/*******************************************************************************
 *
 * kit/kernel/include/debug.h
 * - debug helper macros
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef DEBUG_H
#define DEBUG_H

#include <stdint.h>

#include "terminal.h"
#include "x86_64.h"

#define DEBUG_FORMAT(format, ...) \
  terminal_printf("%s:%d(%s): ", __FILE__, __LINE__, __func__); \
  terminal_printf(format, __VA_ARGS__); \
  terminal_writechar('\n')

#define DEBUG_MESSAGE(message) \
  terminal_printf("%s:%d(%s): %s\n", __FILE__, __LINE__, __func__, message)

#define DEBUG_MESSAGE_HEX(message, value) \
  terminal_printf("%s:%d(%s): %s (%#lx)\n", __FILE__, __LINE__, __func__, \
      (message), (uint64_t) (value))

#define DEBUG_BEGIN_VALUES() \
  terminal_printf("%s:%d(%s): ", __FILE__, __LINE__, __func__)

#define DEBUG_HEX(value) \
  terminal_printf("%s=%#lx ", #value, (uint64_t) (value))

#define DEBUG_DEC(value) \
  terminal_printf("%s=%lu ", #value, (uint64_t) (value))

#define DEBUG_END_VALUES() \
  terminal_writechar('\n')

#define DEBUG_ASSERT(condition) \
  if (!(condition)) \
  { \
    terminal_printf("%s:%d(%s): assertion failed: %s\n", \
        __FILE__, __LINE__, __func__, #condition); \
    cli(); \
    hlt(); \
  }

#endif
