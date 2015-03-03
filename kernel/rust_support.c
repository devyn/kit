/*******************************************************************************
 *
 * kit/kernel/rust_support.c
 * - support functions for rust libcore (most are stubs)
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "debug.h"
#include "x86_64.h"

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

void __morestack()
{
  static const char morestack_msg[18] =
    {'m',0xF0,'o',0xF0,'r',0xF0,'e',0xF0,'s',0xF0,
     't',0xF0,'a',0xF0,'c',0xF0,'k',0xF0};

  memcpy((void *) (0xffffffff800B8000L + (80 * 24 * 2)),
         (const void *) morestack_msg, 18);
  cli();
  while (1) hlt();
}

void __stub(const char *fn)
{
  DEBUG_FORMAT("%s", fn); cli(); while (1) hlt();
}

// Floating point stuff
void trunc()       { __stub(__func__); }
void truncf()      { __stub(__func__); }
void fmod()        { __stub(__func__); }
void fmodf()       { __stub(__func__); }
void exp()         { __stub(__func__); }
void expf()        { __stub(__func__); }
void exp2()        { __stub(__func__); }
void exp2f()       { __stub(__func__); }
void log()         { __stub(__func__); }
void logf()        { __stub(__func__); }
void log2()        { __stub(__func__); }
void log2f()       { __stub(__func__); }
void log10()       { __stub(__func__); }
void log10f()      { __stub(__func__); }
void pow()         { __stub(__func__); }
void powf()        { __stub(__func__); }
void floor()       { __stub(__func__); }
void floorf()      { __stub(__func__); }
void ceil()        { __stub(__func__); }
void ceilf()       { __stub(__func__); }
void round()       { __stub(__func__); }
void roundf()      { __stub(__func__); }
void fma()         { __stub(__func__); }
void fmaf()        { __stub(__func__); }
void __powisf2()   { __stub(__func__); }
void __powidf2()   { __stub(__func__); }

// XXX: What is this?
void _Unwind_Resume() { __stub(__func__); }
