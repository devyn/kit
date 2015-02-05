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

static inline void invlpg(void *pointer)
{
  __asm__ volatile("invlpg (%0)" : : "r" (pointer));
}

#define IA32_EFER  0xC0000080
#define IA32_STAR  0xC0000081
#define IA32_LSTAR 0xC0000082
#define IA32_CSTAR 0xC0000083
#define IA32_FMASK 0xC0000084

static inline uint64_t rdmsr(uint32_t msr)
{
  uint64_t rax;
  uint64_t rdx;
  
  __asm__ volatile("rdmsr" : "=a" (rax), "=d" (rdx) : "c" (msr));

  return (rax << 32) | rdx;
}

static inline void wrmsr(uint64_t value, uint32_t msr)
{
  uint64_t rax = value & ~((uint64_t) -1 << 32);
  uint64_t rdx = value >> 32;

  __asm__ volatile("wrmsr" : : "a" (rax), "d" (rdx), "c" (msr));
}

#endif
