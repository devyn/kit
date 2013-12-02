/*******************************************************************************
 *
 * kit/kernel/memory.c
 * - kernel memory management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdint.h>
#include "x86_64.h"

#include "memory.h"

/**
 * Not actually a uint8_t; just a location, and uint8_t is convenient
 * because it matches.
 */
extern uint8_t _kernel_end;

/* uint8_t in order to operate byte-by-byte. */
uint8_t *memory_stack_base    = &_kernel_end;
uint8_t *memory_stack_pointer = &_kernel_end;

void *memory_alloc(const size_t size)
{
  /**
   * TODO: Proper memory management and bounds checking.
   * As it is, this function can easily "allocate" memory
   * outside of the hilariously puny page that we have set
   * up for our kernel (first 2MB).
   */

  void *result = memory_stack_pointer;

  memory_stack_pointer += size;

  return result;
}

void *memory_alloc_aligned(size_t size, size_t alignment)
{
  size_t pointer_value = (size_t) memory_stack_pointer;

  if (pointer_value % alignment != 0)
  {
    memory_stack_pointer += alignment - (pointer_value % alignment);
  }

  return memory_alloc(size);
}

void memory_clear(void *pointer, size_t size)
{
  size_t size_in_quads = size / 8;

  rep_stosq(pointer, 0, size_in_quads);

  uint8_t *remaining_pointer = (uint8_t *) pointer + (size / 8 * 8);
  size_t   remaining_bytes   = size % 8;

  rep_stosb(remaining_pointer, 0, remaining_bytes);
}
