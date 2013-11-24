#ifndef MEMORY_H
#define MEMORY_H

#include <stddef.h>

void *memory_alloc(size_t size);

/* No memory_free yet, as allocation is stack based */

void *memory_alloc_aligned(size_t size, size_t alignment);

void memory_clear(void *pointer, size_t size);

#endif
