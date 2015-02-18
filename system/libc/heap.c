/*******************************************************************************
 *
 * kit/system/libc/heap.c
 * - heap allocation
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <kit/syscall.h>

uint64_t  _libc_heap_length;
void     *_libc_heap_start;
void     *_libc_heap_end;

void *malloc(size_t size)
{
  // malloc(0) returns NULL.
  if (size == 0)
  {
    return NULL;
  }

  // Align to 16 bytes.
  if (size % 16 != 0)
  {
    size = (size / 16 + 1) * 16;
  }

  // New memory will be at start + length.
  void *ptr = (void *) ((uintptr_t) _libc_heap_start + _libc_heap_length);

  // Advance heap.
  _libc_heap_end     = syscall_adjust_heap(size);
  _libc_heap_length += size;

  if ((uintptr_t) _libc_heap_end -
      (uintptr_t) _libc_heap_start <
      _libc_heap_length)
  {
    // FIXME: ENOMEM
    return NULL;
  }

  // Return the pointer.
  return ptr;
}

void *calloc(size_t count, size_t size)
{
  void *pointer = malloc(count * size);

  if (pointer != NULL)
  {
    memset(pointer, 0, count * size);
  }

  return pointer;
}

void free(void *ptr)
{
  // FIXME: Stub.
  void *temp = ptr;
  ptr = temp;
}
