/*******************************************************************************
 *
 * kit/system/libc/include/stdlib.h
 * - <stdlib.h>: 'standard library definitions'
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * This file should be compatible with ANSI C [C89].
 *
 ******************************************************************************/

#ifndef _STDLIB_H
#define _STDLIB_H

#include <stddef.h>

/**
 * Exit with the given status code after cleaning up.
 */
void exit(int status);

/**
 * Exit with the given status code without cleaning up.
 */
void _Exit(int status);

/**
 * Allocate memory of 'size' bytes and return a pointer to the beginning of the
 * allocated block.
 */
void *malloc(size_t size);

/**
 * Allocate contiguous memory for an array of 'count' items of 'size' bytes in
 * length each, and initialize the entire block of memory to zero.
 */
void *calloc(size_t count, size_t size);

/**
 * Resize and possibly relocate a previously allocated block of memory, and
 * return the new pointer.
 *
 * If ptr is NULL, this is equivalent to malloc(size).
 * If size is zero, this is equivalent to free(ptr), and NULL is returned.
 */
void *realloc(void *ptr, size_t size);

/**
 * Free previously allocated memory.
 */
void free(void *ptr);

/**
 * Parse a string to a long int.
 *
 * Any amount of whitespace at the beginning of the string is skipped. A single
 * '+' or '-' may follow.
 *
 * If base is 16, a '0x' prefix may follow. Otherwise, if base is 8, a '0'
 * prefix may follow.
 *
 * If base is zero, and either prefix is present, base will be set respectively.
 * Otherwise, base will be set to 10.
 *
 * Digits are then interpreted intutitvely according to base, in the range
 * '0'..'9', 'A'..'Z', case-insensitive. The minimum base is 2; the maximum is
 * 36.
 *
 * The parser will stop at the first parsing error or null byte, with the
 * most recent position then stored in *endptr. This should be used to detect
 * errors. If *endptr is not the end of the string, the string is not a valid
 * input, and the return value should be ignored.
 *
 * FIXME: currently does not handle overflow, but should.
 */
long int strtol(const char *nptr, char **endptr, int base);

#endif
