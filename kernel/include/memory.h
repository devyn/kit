/*******************************************************************************
 *
 * kit/kernel/include/memory.h
 * - kernel memory management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef MEMORY_H
#define MEMORY_H

#include <stddef.h>
#include <stdint.h>

/**
 * Loads the memory map information into the region tree in order to know where
 * in physical memory it's safe to allocate fresh pages.
 */
void memory_initialize(const char *mmap_buffer, const uint32_t mmap_length);

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

/**
 * Gets the number of free pages (4096 bytes) available in the system.
 */
uint64_t memory_get_total_free();

/**
 * Acquires free memory from the physical address space.
 *
 * Given a number of pages to acquire and a pointer within which to store
 * the physical address of the acquired memory, memory_free_region_acquire()
 * attempts to acquire a contiguous region of physical memory.
 *
 * The return value indicates how many contiguous pages were actually allocated.
 * If the return value is zero, the system has no free memory available, and
 * *physical_base is undefined.
 */
uint64_t memory_free_region_acquire(const uint64_t pages,
                                    uint64_t *physical_base);

/**
 * Releases a contiguous region of previously acquired physical memory to the
 * free memory tree.
 */
void memory_free_region_release(const uint64_t physical_base,
                                const uint64_t pages);

#endif
