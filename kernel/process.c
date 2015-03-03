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
#include "rbtree.h"
#include "debug.h"

static void *process_original_ksp;
static void *process_original_kslimit;

extern void *rust_stack_limit;

extern void *process_asm_prepare(void *stack_pointer);

extern void process_asm_switch(void **old_stack_pointer,
    void *new_stack_pointer);

// Offsets for access from assembly.
const size_t PROCESS_OFFSET_KERNEL_STACK_POINTER =
  offsetof(process_t, kernel_stack_pointer);

const size_t PROCESS_OFFSET_REGISTERS =
  offsetof(process_t, registers);

typedef struct {
  rbtree_t     tree;
  uint64_t     size;
  process_id_t next_id;
} process_list_t;

typedef struct {
  rbtree_node_t  node;
  process_id_t   id;
  process_t     *process;
} process_list_node_t;

static process_list_t process_list;

void process_initialize()
{
  process_current = NULL;

  process_list.tree.root = NULL;
  process_list.size      = 0;
  process_list.next_id   = 1;

  syscall_initialize();
}

process_t *process_get(process_id_t id)
{
  process_list_node_t *node =
    (process_list_node_t *) process_list.tree.root;

  while (node != NULL)
  {
    if (node->id == id)
    {
      return node->process;
    }
    else if (node->id < id)
    {
      node = (process_list_node_t *) node->node.right;
    }
    else if (node->id > id)
    {
      node = (process_list_node_t *) node->node.left;
    }
  }

  return NULL;
}

static void __process_list_insert(process_t *process)
{
  process_list_node_t *parent = NULL;

  process_list_node_t *node =
    (process_list_node_t *) process_list.tree.root;

  DEBUG_ASSERT(process_list.size != 65535); // XXX

  while (node != NULL)
  {
    DEBUG_ASSERT(node->id != process->id);

    if (node->id < process->id)
    {
      parent = node;
      node   = (process_list_node_t *) node->node.right;
    }
    else
    {
      parent = node;
      node   = (process_list_node_t *) node->node.left;
    }
  }

  process_list_node_t *new_node = memory_alloc(sizeof(process_list_node_t *));

  DEBUG_ASSERT(new_node != NULL);

  memory_set((void *) new_node, 0, sizeof(process_list_node_t));

  new_node->id          = process->id;
  new_node->process     = process;
  new_node->node.parent = &parent->node;

  if (parent != NULL)
  {
    if (parent->id < process->id)
    {
      parent->node.right = &new_node->node;
    }
    else
    {
      parent->node.left = &new_node->node;
    }

    rbtree_balance_insert(&process_list.tree, &new_node->node);
  }
  else
  {
    process_list.tree.root = &new_node->node;
  }

  process_list.size++;
}

process_t *process_create(const char *name)
{
  size_t name_length = string_length(name);

  if (name_length > 255)
  {
    return false;
  }

  process_t *process = memory_alloc(sizeof(process_t));

  if (process == NULL)
  {
    return NULL;
  }

  memory_set(process, 0, sizeof(process_t));

  memory_copy(name, &process->name, name_length + 1);

  if (!paging_create_pageset(&process->pageset))
  {
    return NULL;
  }

  // Set up the kernel stack
  process->kernel_stack_base = memory_alloc_aligned(2048, 16);

  if (process->kernel_stack_base == NULL)
  {
    return NULL;
  }

  process->kernel_stack_pointer =
    (void *) ((uintptr_t) process->kernel_stack_base + 2048);

  process->kernel_stack_pointer =
    process_asm_prepare(process->kernel_stack_pointer);

  // Set up the user stack
  process->registers.rsp = PROCESS_STACK_BASE;

  if (process_alloc(process, (void *) (process->registers.rsp - 8192), 8192, 0)
      == NULL)
  {
    return NULL;
  }

  process->id = process_list.next_id++;

  __process_list_insert(process);

  return process;
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

bool process_alloc_with_kernel(process_t *process, void *user_address,
    void *kernel_address, uint64_t length, paging_flags_t flags)
{
  union {
    uint64_t  linear;
    void     *pointer;
  } current_user, current_kernel;

  current_user.pointer   = user_address;
  current_kernel.pointer = kernel_address;

  // Address must be normalized.
  DEBUG_ASSERT(current_user.linear   % 4096 == 0);
  DEBUG_ASSERT(current_kernel.linear % 4096 == 0);

  // Normalize the length to get a number of pages.
  uint64_t pages = (length >> 12) + ((length & 0xfff) == 0 ? 0 : 1);

  // Ensure we have a non-zero number of pages.
  if (pages == 0) return NULL;

  // Retrieve and map physical pages until we've fulfilled the request.
  while (pages > 0)
  {
    uint64_t physical_base, mapped;

    mapped = memory_free_region_acquire(pages, &physical_base);

    // Make sure we didn't run out of memory.
    if (mapped > 0)
    {
      uint64_t mapped_user =
        paging_map(&process->pageset, current_user.pointer,
            physical_base, mapped, flags | PAGING_USER);

      uint64_t mapped_kernel =
        paging_map(&paging_kernel_pageset, current_kernel.pointer,
            physical_base, mapped, flags & ~PAGING_USER);

      DEBUG_ASSERT(mapped_user   == mapped);
      DEBUG_ASSERT(mapped_kernel == mapped);

      current_user.linear    += mapped << 12;
      current_kernel.linear  += mapped << 12;
      pages                  -= mapped;
    }
    else
    {
      // Out of memory.
      // FIXME: free any allocations
      return false;
    }
  }

  return true;
}

void process_free(process_t *process, void *address, uint64_t length)
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

  // Unmap and release physical pages until we've fulfilled the request.
  while (pages > 0)
  {
    uint64_t physical_base;

    if (paging_resolve_linear_address(&process->pageset,
          current_address.pointer, &physical_base))
    {
      // Page is present.
      uint64_t unmap = 1;
      current_address.linear += 4096;

      // Look for a contiguous physical region. FIXME: probably slow
      while (true)
      {
        uint64_t this_physical_base;

        if (paging_resolve_linear_address(&process->pageset,
              current_address.pointer, &this_physical_base))
        {
          if (this_physical_base == physical_base + unmap * 4096)
          {
            unmap++;
            current_address.linear += 4096;
          }
          else
          {
            break;
          }
        }
        else
        {
          break;
        }
      }

      // Release the region.
      memory_free_region_release(physical_base, unmap);

      pages -= unmap;
    }
    else
    {
      // Page is not present. Ignore.
      current_address.linear += 4096;
      pages--;
    }
  }
}

void *process_adjust_heap(process_t *process, int64_t amount)
{
  uint64_t old_heap_length = process->heap_length;

  process->heap_length += amount;

  uint64_t old_heap_pages = old_heap_length      / 4096;
  uint64_t new_heap_pages = process->heap_length / 4096;

  if (old_heap_length      % 4096 != 0) old_heap_pages++;
  if (process->heap_length % 4096 != 0) new_heap_pages++;

  if (new_heap_pages > old_heap_pages)
  {
    // Allocate pages
    if (process_alloc(process,
          (void *) (PROCESS_HEAP_BASE + old_heap_pages * 4096),
          (new_heap_pages - old_heap_pages) * 4096, 0) == NULL)
    {
      DEBUG_MESSAGE("allocation error");

      // Allocation error?
      process->heap_length -= amount;
      new_heap_pages = old_heap_pages;
    }
  }
  else if (new_heap_pages < old_heap_pages)
  {
    // Free pages
    process_free(process, (void *) (PROCESS_HEAP_BASE + new_heap_pages * 4096),
        (old_heap_pages - new_heap_pages) * 4096);
  }

  return (void *) (PROCESS_HEAP_BASE + new_heap_pages * 4096);
}

bool process_set_args(process_t *process, int argc, const char *const *argv)
{
  // If there are a negative number of args, return an error.
  if (argc < 0)
  {
    return false;
  }

  // If there are exactly zero args, just set rdi to argc and rsi to NULL.
  if (argc == 0)
  {
    process->registers.rdi = argc;
    process->registers.rsi = (uint64_t) NULL;
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
  void *user_base   = (void *) ((0x7feeffffffff - total_bytes) & (-1 << 12));
  void *kernel_base = (void *) 0xffff888800000000;

  ptrdiff_t base_delta = (uintptr_t) kernel_base - (uintptr_t) user_base;

  if (!process_alloc_with_kernel(process, user_base, kernel_base,
        total_bytes, 0))
  {
    return false;
  }

  // Copy the args.
  char **pointer_array = (char **) kernel_base;
  char  *data          = (char  *) (pointer_array + argc);

  for (int i = 0; i < argc; i++)
  {
    pointer_array[i] = (char *) ((uintptr_t) data - base_delta);

    for (const char *arg = argv[i]; *arg != '\0'; data++, arg++)
    {
      *data = *arg;
    }
    *(data++) = '\0';
  }

  // Unmap the arg region in the kernel.
  uint64_t pages = total_bytes >> 12;

  if (total_bytes % 4096 != 0) pages++;

  paging_unmap(&paging_kernel_pageset, kernel_base, pages);

  // Set argc, argv.
  process->registers.rdi = argc;
  process->registers.rsi = (uint64_t) user_base;

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
#ifdef PROCESS_DEBUG
    DEBUG_FORMAT("-> [%hu] %s", process->id, process->name);
#endif

    DEBUG_ASSERT(process->state == PROCESS_STATE_RUNNING);

    process_t *old_process = process_current;

    process_current = process;

    paging_set_current_pageset(&process->pageset);

    if (old_process != NULL)
    {
			process_original_kslimit = rust_stack_limit;
			rust_stack_limit         = process->kernel_stack_base;

      process_asm_switch(&old_process->kernel_stack_pointer,
          process->kernel_stack_pointer);
    }
    else
    {
			rust_stack_limit = process->kernel_stack_base;

      process_asm_switch(&process_original_ksp,
          process->kernel_stack_pointer);
    }
  }
  else if (process_current != NULL)
  {
#ifdef PROCESS_DEBUG
    DEBUG_MESSAGE("-> kernel");
#endif

    process_t *old_process = process_current;

    process_current = NULL;

    paging_set_current_pageset(&paging_kernel_pageset);

		rust_stack_limit = process_original_kslimit;

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
