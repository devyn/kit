/*******************************************************************************
 *
 * kit/system/libc/include/stdlib.h
 * - <stdlib.h>: 'standard library definitions'
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
 * Absolute value of an integer.
 */
int abs(int n);

/**
 * Exit with status code 1.
 */
void abort(void);

#endif
