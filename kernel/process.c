/*******************************************************************************
 *
 * kit/kernel/process.c
 * - process management functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stddef.h>

#include "process.h"
#include "syscall.h"
#include "string.h"
#include "memory.h"
#include "debug.h"

process_t *process_current;
uint16_t   process_next_id;

void process_initialize()
{
  process_current = NULL;
  process_next_id = 1;

  syscall_initialize();
}

bool process_create(process_t *process, const char *name)
{
  size_t name_length = string_length(name);

  if (name_length > 255)
  {
    return false;
  }

  memory_set(process, 0, sizeof(process_t));

  memory_copy(name, &process->name, name_length + 1);

  if (!paging_create_pageset(&process->pageset))
  {
    return false;
  }

  // Set up the stack
  process->registers.rsp = 0x7ffffffff000;

  if (process_alloc(process, (void *) (process->registers.rsp - 8192), 8192, 0)
      == NULL)
  {
    return false;
  }

  process->id = process_next_id++;

  return true;
}

void *process_alloc(process_t *process, void *address, uint64_t length,
    paging_flags_t flags)
{
  union {
    uint64_t  linear;
    void     *pointer;
  } padded_address, current_address;

  padded_address.pointer = address;

  // Normalize the address.
  length                += padded_address.linear & 0xfff;
  padded_address.linear &= ~0xfff;

  current_address = padded_address;

  // Normalize the length to get a number of pages.
  uint64_t pages = (length >> 12) + ((length & 0xfff) == 0 ? 0 : 1);

  // Ensure we have a non-zero number of pages.
  if (pages == 0) return NULL;

  // Force PAGING_USER flag to be set.
  flags |= PAGING_USER;

  // Retrieve and map physical pages until we've fulfilled the request.
  while (pages > 0)
  {
    uint64_t physical_base, mapped;

    mapped = memory_free_region_acquire(pages, &physical_base);

    // Make sure we didn't run out of memory.
    if (mapped > 0)
    {
      // FIXME: handle any errors here
      paging_map(&process->pageset, current_address.pointer,
          physical_base, mapped, flags);

      current_address.linear += mapped << 12;
      pages                  -= mapped;
    }
    else
    {
      // Out of memory.
      // FIXME: free any allocations
      return NULL;
    }
  }

  // Done. Return the padded address.
  return padded_address.pointer;
}

bool process_set_args(process_t *process, int argc, char **argv)
{
  // If there are a negative number of args, return an error.
  if (argc < 0)
  {
    return false;
  }

  // If there are exactly zero args, just set r8 to argc and r9 to NULL.
  if (argc == 0)
  {
    process->registers.r8 = argc;
    process->registers.r9 = (uint64_t) NULL;
    return true;
  }

  // Count the number of total bytes that will be needed to store the strings
  // and the pointer array.
  size_t total_bytes = 0;

  for (int i = 0; i < argc; i++)
  {
    total_bytes += sizeof(char *) + string_length(argv[i]) + 1;
  }

  // Allocate memory within the process by subtracting from a known pointer
  // value and aligning to page.
  uint64_t intended_base = (0x7feeffffffff - total_bytes) & (-1 << 12);

  void *base = process_alloc(process, (void *) intended_base, total_bytes, 0);

  if (base == NULL)
  {
    return false;
  }

  // Load the process's pageset.
  paging_pageset_t *old_pageset = paging_get_current_pageset();

  paging_set_current_pageset(&process->pageset);

  // Copy the args.
  char **pointer_array = (char **) base;
  char  *data          = (char  *) (pointer_array + argc);

  for (int i = 0; i < argc; i++)
  {
    pointer_array[i] = data;

    for (char *arg = argv[i]; *arg != '\0'; data++, arg++)
    {
      *data = *arg;
    }
    *(data++) = '\0';
  }

  // Set argc, argv.
  process->registers.r8 = argc;
  process->registers.r9 = (uint64_t) pointer_array;

  // Restore old pageset.
  paging_set_current_pageset(old_pageset);

  return true;
}

void process_set_entry_point(process_t *process, void *instruction)
{
  DEBUG_ASSERT(process->state == PROCESS_STATE_LOADING);

  process->registers.rip = (uint64_t) instruction;
}

extern void process_asm_call();

void process_run(process_t *process)
{
  // Make sure we aren't already running a process.
  DEBUG_ASSERT(process_current == NULL);

  // Make sure the process is ready to be run, and set it to RUNNING.
  DEBUG_ASSERT(process->state == PROCESS_STATE_LOADING);

  process->state = PROCESS_STATE_RUNNING;

  // Set the current process.
  process_current = process;

  // Load the process's pageset.
  paging_pageset_t *old_pageset = paging_get_current_pageset();

  paging_set_current_pageset(&process->pageset);

  // Enter the process.
  process_asm_call();

  // Print the process's registers. [DEBUG]
  DEBUG_FORMAT(
      "\n"
      " RAX=%lx RCX=%lx RDX=%lx RBX=%lx\n"
      " RSP=%lx RBP=%lx RSI=%lx RDI=%lx\n"
      " R8 =%lx R9 =%lx R10=%lx R11=%lx\n"
      " R12=%lx R13=%lx R14=%lx R15=%lx\n"
      " RIP=%lx\n"
      " EFLAGS=%x",
      process->registers.rax,
      process->registers.rcx,
      process->registers.rdx,
      process->registers.rbx,
      process->registers.rsp,
      process->registers.rbp,
      process->registers.rsi,
      process->registers.rdi,
      process->registers.r8,
      process->registers.r9,
      process->registers.r10,
      process->registers.r11,
      process->registers.r12,
      process->registers.r13,
      process->registers.r14,
      process->registers.r15,
      process->registers.rip,
      process->registers.eflags);

  // Kill the process.
  process->state = PROCESS_STATE_DEAD; // XXX

  // Set the current process to NULL (no process).
  process_current = NULL;

  // Load the original pageset.
  paging_set_current_pageset(old_pageset);
}
