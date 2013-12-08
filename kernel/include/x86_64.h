/*******************************************************************************
 *
 * kit/kernel/include/x86_64.h
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

#ifndef X86_64_H
#define X86_64_H

#include <stddef.h>
#include <stdint.h>

#define outb(value, port) \
  __asm__ volatile("outb %%al, %%dx" : : "a" ((uint8_t) (value)), "d" ((uint16_t) (port)))

#define inb(port, value) \
  __asm__ volatile("inb %%dx, %%al" : "=a" ((uint8_t) (value)) : "d" ((uint16_t) (port)))

#define lidt(pointer) \
  __asm__ volatile("lidt %0" : : "m" (pointer))

#define hlt() \
  __asm__ volatile("hlt")

#define cli() \
  __asm__ volatile("cli")

#define sti() \
  __asm__ volatile("sti")

void rep_stosb(void *pointer, uint8_t  value, size_t count);
void rep_stosq(void *pointer, uint64_t value, size_t count);

#endif
