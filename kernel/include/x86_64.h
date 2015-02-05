/*******************************************************************************
 *
 * kit/kernel/include/x86_64.h
 * - x86_64 asm functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Contains macros and functions for the required x86_64 instructions that C
 * doesn't provide access to.
 *
 ******************************************************************************/

#ifndef X86_64_H
#define X86_64_H

#include <stddef.h>
#include <stdint.h>

static inline void outb(uint8_t value, uint16_t port)
{
  __asm__ volatile("outb %%al, %%dx" : : "a" (value), "d" (port));
}

static inline uint8_t inb(uint16_t port)
{
  uint8_t value;
  __asm__ volatile("inb %%dx, %%al" : "=a" (value) : "d" (port));
  return value;
}

static inline void lidt(void *pointer)
{
  __asm__ volatile("lidt (%0)" : : "r" (pointer));
}

static inline void hlt()
{
  __asm__ volatile("hlt");
}

static inline void cli()
{
  __asm__ volatile("cli");
}

static inline void sti()
{
  __asm__ volatile("sti");
}

static inline void rep_stosb(void *pointer, uint8_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosb"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}

static inline void rep_stosq(void *pointer, uint64_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosq"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}

static inline void invlpg(void *pointer)
{
  __asm__ volatile("invlpg (%0)" : : "r" (pointer));
}

#endif
