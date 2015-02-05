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

#include "process.h"
#include "string.h"
#include "memory.h"
#include "debug.h"

uint16_t process_next_id;

void process_initialize()
{
  process_next_id = 1;
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

void process_set_entry_point(process_t *process, void *instruction)
{
  DEBUG_ASSERT(process->state == PROCESS_STATE_LOADING);

  process->registers.rip = (uint64_t) instruction;
}

extern void process_asm_call(process_registers_t *registers);

void process_run(process_t *process)
{
  DEBUG_ASSERT(process->state == PROCESS_STATE_LOADING);

  process->state = PROCESS_STATE_RUNNING;

  paging_pageset_t *old_pageset = paging_get_current_pageset();

  paging_set_current_pageset(&process->pageset);

  process_registers_t *regs = &process->registers;

  process_asm_call(regs);

  DEBUG_FORMAT(
      "\n"
      " RAX=%lx RCX=%lx RDX=%lx RBX=%lx\n"
      " RSP=%lx RBP=%lx RSI=%lx RDI=%lx\n"
      " R8 =%lx R9 =%lx R10=%lx R11=%lx\n"
      " R12=%lx R13=%lx R14=%lx R15=%lx\n"
      " RIP=%lx\n"
      " EFLAGS=%x",
      regs->rax,
      regs->rcx,
      regs->rdx,
      regs->rbx,
      regs->rsp,
      regs->rbp,
      regs->rsi,
      regs->rdi,
      regs->r8,
      regs->r9,
      regs->r10,
      regs->r11,
      regs->r12,
      regs->r13,
      regs->r14,
      regs->r15,
      regs->rip,
      regs->eflags);

  process->state = PROCESS_STATE_DEAD; // XXX

  paging_set_current_pageset(old_pageset);
}
