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

void *memory_alloc(size_t size);

/* No memory_free yet, as allocation is stack based */

void *memory_alloc_aligned(size_t size, size_t alignment);

void memory_clear(void *pointer, size_t size);

#endif
