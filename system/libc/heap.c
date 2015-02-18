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

typedef struct block_header
{
  uint64_t size;
  uint64_t pad0;
} block_header_t;

void *malloc(size_t size)
{
  // malloc(0) returns NULL.
  if (size == 0)
  {
    return NULL;
  }

  size_t aligned_size = size;

  // Align to 16 bytes.
  if (size % 16 != 0)
  {
    aligned_size = (size / 16 + 1) * 16;
  }

  // New memory will be at start + length.
  uintptr_t ptr = (uintptr_t) _libc_heap_start + _libc_heap_length;

  // Advance heap.
  _libc_heap_end     = syscall_adjust_heap(aligned_size +
                                           sizeof(block_header_t));
  _libc_heap_length += aligned_size + sizeof(block_header_t);

  if ((uintptr_t) _libc_heap_end -
      (uintptr_t) _libc_heap_start <
      _libc_heap_length)
  {
    // FIXME: ENOMEM
    return NULL;
  }

  // Set block header.
  block_header_t *header = (block_header_t *) ptr;

  header->size = size;

  // Return the pointer.
  return (void *) (ptr + sizeof(block_header_t));
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

void *realloc(void *ptr, size_t size)
{
  if (ptr == NULL)
  {
    return malloc(size);
  }
  else if (size == 0)
  {
    free(ptr);
    return NULL;
  }
  else
  {
    block_header_t *header = (block_header_t *)
      ((uintptr_t) ptr - sizeof(block_header_t));

    if (size < header->size)
    {
      // Simply make the size smaller.
      header->size = size;
      return ptr;
    }
    else if (size > header->size)
    {
      // Allocate a new block, copy, and free.
      void *new_ptr = malloc(size);
      memcpy(new_ptr, ptr, header->size);
      free(ptr);
      return new_ptr;
    }
    else
    {
      return ptr;
    }
  }
}

void free(void *ptr)
{
  // FIXME: Stub.
  void *temp = ptr;
  ptr = temp;
}
