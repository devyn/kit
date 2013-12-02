/*******************************************************************************
 *
 * kit/kernel/x86_64.c
 * - x86_64 asm functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Contains macros and functions for the required x86_64 instructions that C
 * doesn't provide access to.
 *
 ******************************************************************************/

#include "x86_64.h"

void rep_stosb(void *pointer, uint8_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosb"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}

void rep_stosq(void *pointer, uint64_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosq"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}
