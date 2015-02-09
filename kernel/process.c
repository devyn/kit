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
#include "scheduler.h"
#include "string.h"
#include "memory.h"
#include "debug.h"

void *process_original_ksp;

uint16_t process_next_id;

extern void *process_asm_prepare(void *stack_pointer);

extern void process_asm_switch(void **old_stack_pointer,
    void *new_stack_pointer);

// Offsets for access from assembly.
const size_t PROCESS_OFFSET_KERNEL_STACK_POINTER =
  offsetof(process_t, kernel_stack_pointer);

const size_t PROCESS_OFFSET_REGISTERS =
  offsetof(process_t, registers);

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

  // Set up the kernel stack
  process->kernel_stack_base = memory_alloc_aligned(2048, 16);

  if (process->kernel_stack_base == NULL)
  {
    return false;
  }

  process->kernel_stack_pointer =
    (void *) ((uintptr_t) process->kernel_stack_base + 2048);

  process->kernel_stack_pointer =
    process_asm_prepare(process->kernel_stack_pointer);

  // Set up the user stack
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

void process_switch(process_t *process)
{
  if (process != NULL)
  {
    DEBUG_FORMAT("-> [%hu] %s", process->id, process->name);

    DEBUG_ASSERT(process->state == PROCESS_STATE_RUNNING);

    process_t *old_process = process_current;

    process_current = process;

    paging_set_current_pageset(&process->pageset);

    if (old_process != NULL)
    {
      process_asm_switch(&old_process->kernel_stack_pointer,
          process->kernel_stack_pointer);
    }
    else
    {
      process_asm_switch(&process_original_ksp,
          process->kernel_stack_pointer);
    }
  }
  else if (process_current != NULL)
  {
    DEBUG_MESSAGE("-> kernel");

    process_t *old_process = process_current;

    process_current = NULL;

    paging_set_current_pageset(&paging_kernel_pageset);

    process_asm_switch(&old_process->kernel_stack_pointer,
        process_original_ksp);
  }
}

void process_run(process_t *process)
{
  DEBUG_ASSERT(process->state == PROCESS_STATE_LOADING);

  process->state = PROCESS_STATE_RUNNING;

  scheduler_enqueue_run(process);
}
