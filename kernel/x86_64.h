/**
 * Contains macros for the required x86_64 instructions that C doesn't provide access to.
 */

#ifndef X86_64_H
#define X86_64_H

#define outb(value, port) \
  __asm__ volatile("outb %%al, %%dx" : : "a" ((uint8_t) (value)), "d" ((uint16_t) (port)))

#define inb(port, value) \
  __asm__ volatile("inb %%dx, %%al" : "=a" ((uint8_t) (value)) : "d" ((uint16_t) (port)))

#endif
