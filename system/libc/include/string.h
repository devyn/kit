/*******************************************************************************
 *
 * kit/system/libc/include/string.h
 * - <string.h>: 'string operations'
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * This file should be compatible with ANSI C [C89].
 *
 ******************************************************************************/

#ifndef _STRING_H
#define _STRING_H

#include <stddef.h>

/**
 * Initialize the memory region of 'n' bytes starting at 's' by setting each
 * byte within to 'c'.
 */
void *memset(void *s, int c, size_t n);

/**
 * Copy 'n' bytes from 'src' to 'dest'. The memory regions must not overlap.
 */
#if __STDC_VERSION__ >= 199901L
  void *memcpy(void *restrict dest, const void *restrict src, size_t n);
#else
  void *memcpy(void *dest, const void *src, size_t n);
#endif

/**
 * Copy 'n' bytes from 'src' to 'dest'. The memory regions may overlap.
 */
void *memmove(void *dest, const void *src, size_t n);

/**
 * Compare the 'n' byte memory regions 's1' and 's2'.
 *
 * If s1 > s2, returns  1.
 * If s1 < s2, returns -1.
 * Otherwise,  returns  0.
 */
int memcmp(const void *s1, const void *s2, size_t n);

/**
 * Compare the null-terminated strings 's1' and 's2'.
 *
 * If 's2' is a prefix of 's1' but 's1' is longer, then 's1' is considered to be
 * greater, and vice versa.
 *
 * If s1 > s2, returns  1.
 * If s1 < s2, returns -1.
 * Otherwise,  returns  0.
 */
int strcmp(const char *s1, const char *s2);

/**
 * Find the length of the string 's'.
 *
 * That is, count bytes up to but not including the terminating null byte.
 */
size_t strlen(const char *s);

/**
 * Copies the string 'src' to the end of the string 'dest'.
 *
 * Warning: The buffer must be large enough to contain the length of both
 * strings as well as the terminating null character.
 */
char *strcat(char *dest, const char *src);

/**
 * Locate byte 'c' in a string, searching from the beginning.
 *
 * Returns NULL if not found.
 */
char *strchr(const char *s, int c);

#endif
