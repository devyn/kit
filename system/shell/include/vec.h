/*******************************************************************************
 *
 * kit/system/shell/include/vec.h
 * - vectors (automatically resizing arrays)
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef _KIT_SHELL_VEC_H
#define _KIT_SHELL_VEC_H

#include <stddef.h>

typedef struct ptr_vec
{
  void   **ptr;
  size_t   len;
  size_t   cap;
} ptr_vec_t;

static inline void ptr_vec_init(ptr_vec_t *vec)
{
  vec->ptr = NULL;
  vec->len = 0;
  vec->cap = 0;
}

static inline ptr_vec_t ptr_vec_new()
{
  ptr_vec_t vec;

  ptr_vec_init(&vec);

  return vec;
}

void ptr_vec_resize(ptr_vec_t *vec, size_t size);

void ptr_vec_push(ptr_vec_t *vec, void *ptr);

void ptr_vec_free(ptr_vec_t *vec);

#endif
