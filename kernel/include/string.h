/*******************************************************************************
 *
 * kit/kernel/include/string.h
 * - C string utilities
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef STRING_H
#define STRING_H

#include <stddef.h>

// Identical to standard C strcmp()
static inline int string_compare(const char *s1, const char *s2)
{
  size_t pos = 0;

  while (s1[pos] != '\0' && s2[pos] != '\0')
  {
    if (s1[pos] < s2[pos])
      return -1;
    else if (s1[pos] > s2[pos])
      return 1;
    else
      pos++;
  }

  if (s1[pos] != '\0')
    return 1;
  else if (s2[pos] != '\0')
    return -1;
  else
    return 0;
}

// Identical to standard C strlen()
static inline size_t string_length(const char *s)
{
  size_t len = 0;

  while (s[len] != '\0') len++;

  return len;
}

#endif
