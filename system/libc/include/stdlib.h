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

/**
 * Exit with the given status code after cleaning up.
 */
void exit(int status);

/**
 * Exit with the given status code without cleaning up.
 */
void _Exit(int status);

#endif
