/*******************************************************************************
 *
 * kit/system/shell/vec.c
 * - vectors (automatically resizing arrays)
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>

#include "vec.h"

void ptr_vec_resize(ptr_vec_t *vec, size_t size)
{
  if (vec->cap < size)
  {
    vec->cap = size;
    vec->ptr = realloc(vec->ptr, size * sizeof(char **));

    if (vec->ptr == NULL) exit(8); //XXX: Out of memory
  }
  else if (vec->cap > size)
  {
    vec->len = size;
    vec->cap = size;
    vec->ptr = realloc(vec->ptr, size * sizeof(char **));
  }
}

void ptr_vec_push(ptr_vec_t *vec, void *ptr)
{
  if (vec->len == vec->cap)
  {
    if (vec->len == 0)
    {
      ptr_vec_resize(vec, 1);
    }
    else
    {
      ptr_vec_resize(vec, vec->cap * 2);
    }
  }

  vec->ptr[vec->len++] = ptr;
}

void ptr_vec_free(ptr_vec_t *vec)
{
  ptr_vec_resize(vec, 0);
}
