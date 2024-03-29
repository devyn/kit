/*******************************************************************************
 *
 * kit/kernel/include/memory.h
 * - kernel memory management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef MEMORY_H
#define MEMORY_H

#include <stddef.h>
#include <stdint.h>

void *memory_alloc(size_t size);

// Currently no-op
void memory_free(void *pointer);

void *memory_alloc_aligned(size_t size, size_t alignment);

static inline void memory_set(void *pointer, uint8_t value, size_t size)
{
  for (size_t i = 0; i < size; i++) {
    ((uint8_t *) pointer)[i] = value;
  }
}

static inline void memory_copy(const void *src, void *dest, size_t size)
{
  for (size_t i = 0; i < size; i++) {
    ((uint8_t *) dest)[i] = ((uint8_t *) src)[i];
  }
}

/**
 * Identical to C memcmp().
 */
static inline int memory_compare(const void *s1, const void *s2, size_t size)
{
  const char *c_s1 = s1, *c_s2 = s2;

  for (size_t i = 0; i < size; i++)
  {
    if (c_s1[i] < c_s2[i])
      return -1;
    if (c_s1[i] > c_s2[i])
      return 1;
  }

  return 0;
}

/**
 * Gets the number of free pages (4096 bytes) available in the system.
 */
uint64_t memory_get_total_free();

typedef struct {
  int dummy; // do not use
} *memory_rc_t;

#endif
