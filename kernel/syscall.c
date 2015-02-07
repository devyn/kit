/*******************************************************************************
 *
 * kit/kernel/syscall.c
 * - system call interface
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#define SYSCALL_C

#include <stdint.h>

#include "syscall.h"
#include "config.h"
#include "process.h"
#include "x86_64.h"
#include "gdt.h"

#include "terminal.h"

// IA32_EFER.SCE (SysCall Enable); bit #0
#define IA32_EFER_SCE 0x1

// Disable all non-reserved, relevant EFLAGS bits when entering the kernel
#define SYSCALL_FLAG_MASK 0x003f4fd5

// Not intended to be called.
extern void syscall_handler();

// For the syscall handler.
const size_t SYSCALL_OFFSET_PROCESS_REGISTERS = offsetof(process_t, registers);

void syscall_initialize()
{
  // Enable system calls.
  wrmsr(rdmsr(IA32_EFER) | IA32_EFER_SCE, IA32_EFER);

  // Load the STAR with the target segments.
  //
  // User mode [63:48]:   + 0x0  for 32-bit code
  //                      + 0x8  for data
  //                      + 0x10 for 64-bit code
  //
  // Kernel mode [47:32]: + 0x0  for 64-bit code
  //                      + 0x8  for data
  uint64_t star = rdmsr(IA32_STAR);

  star |= ((uint64_t) GDT_SEL_USER_CODE_32 << 48);
  star |= ((uint64_t) GDT_SEL_KERNEL_CODE  << 32);

  wrmsr(star, IA32_STAR);

  // Load LSTAR with the syscall handler.
  wrmsr((uint64_t) &syscall_handler, IA32_LSTAR);

  // Load FMASK with the flag mask.
  wrmsr(SYSCALL_FLAG_MASK, IA32_FMASK);
}

int syscall_twrite(uint64_t length, const char *buffer)
{
  terminal_writebuf(length, buffer);
  return 0;
}
