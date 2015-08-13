/*******************************************************************************
 *
 * kit/system/libc/string.c
 * - string initialization and comparison functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <string.h>

void *memset(void *s, int c, size_t n)
{
  unsigned char *ptr = (unsigned char *) s;
  unsigned char byte = c;

  for (size_t i = 0; i < n; i++)
  {
    ptr[i] = byte;
  }

  return s;
}

void *memcpy(void *restrict dest, const void *restrict src, size_t n)
{
  char *restrict dest_c = (char *restrict) dest;

  const char *restrict src_c = (const char *restrict) src;

  for (size_t i = 0; i < n; i++)
  {
    dest_c[i] = src_c[i];
  }

  return dest;
}

void *memmove(void *dest, const void *src, size_t n)
{
  char *dest_c = (char *) dest;
  const char *src_c = (const char *) src;

  if (src_c < dest_c)
  {
    /**
     * SSSSS
     *   DDDDD
     * (start from end)
     */
    for (size_t i = n; i > 0; i--)
    {
      dest_c[i - 1] = src_c[i - 1];
    }
  }
  else if (dest_c < src_c)
  {
    /**
     *   SSSSS
     * DDDDD
     * (start from beginning)
     */
    for (size_t i = 0; i < n; i++)
    {
      dest_c[i] = src_c[i];
    }
  }
  else
  {
    // Pointers are identical; do nothing
  }

  return dest;
}

int memcmp(const void *s1, const void *s2, size_t n)
{
  const char *s1_c = s1, *s2_c = s2;

  for (size_t i = 0; i < n; i++)
  {
    if (s1_c[i] < s2_c[i])
      return -1;
    if (s1_c[i] > s2_c[i])
      return 1;
  }

  return 0;
}

int strcmp(const char *s1, const char *s2)
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

size_t strlen(const char *s)
{
  size_t len = 0;

  while (s[len] != '\0') len++;

  return len;
}

char *strcat(char *dest, const char *src)
{
  size_t i, j;

  for (i = 0; dest[i] != '\0'; i++);

  for (j = 0; src[j] != '\0'; i++, j++)
  {
    dest[i] = src[j];
  }

  dest[i] = '\0';

  return dest;
}

char *strchr(const char *s, int c)
{
  while (*s != 0)
  {
    if (*s == c) return (char *) s;
  }

  return NULL;
}
