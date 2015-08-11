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

#include <stddef.h>
#include <stdbool.h>

#include "syscall.h"
#include "config.h"
#include "process.h"
#include "scheduler.h"
#include "x86_64.h"
#include "debug.h"
#include "gdt.h"

#include "terminal.h"
#include "interrupt.h"
#include "archive.h"

// IA32_EFER.SCE (SysCall Enable); bit #0
#define IA32_EFER_SCE 0x1

// Disable all non-reserved, relevant EFLAGS bits when entering the kernel
#define SYSCALL_FLAG_MASK 0x003f4fd5

// Not intended to be called.
extern void syscall_handler();

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

int syscall_exit(int status)
{
  process_exit(status);

  DEBUG_MESSAGE("failed to exit process");
  while (true) hlt();
}

int syscall_twrite(uint64_t length, const char *buffer)
{
  terminal_writebuf(length, buffer);
  return 0;
}

int syscall_key_get(keyboard_event_t *event)
{
  keyboard_sleep_dequeue(event);
  return 0;
}

int syscall_yield()
{
  // Might return immediately if there's nothing else to do.
  scheduler_tick();
  return 0;
}

int syscall_sleep()
{
  scheduler_sleep();
  return 0;
}

int syscall_spawn(const char *file, int argc, const char *const *argv)
{
/*
  char     *buffer;
  uint64_t  length;

  if (!archive_get(archive_system, file, &buffer, &length))
  {
    return -1;
  }

  elf_header_64_t *elf = (elf_header_64_t *) buffer;

  if (!elf_verify(elf))
  {
    return -2;
  }

  process_t *process = process_create(argv[0]);

  if (process == NULL)
  {
    return -3;
  }

  if (!elf_load(elf, process))
  {
    return -4;
  }

  if (!process_set_args(process, argc, argv))
  {
    return -5;
  }

  process_run(process);

  return process->id;
*/
  DEBUG_FORMAT("file = %p, argc = %i, argv = %p", file, argc, argv);
  return -1;
}

int syscall_wait_process(process_id_t id, int *exit_status)
{
  return process_wait_exit_status(id, exit_status);
}

void *syscall_adjust_heap(int64_t amount)
{
  return process_adjust_heap(amount);
}
