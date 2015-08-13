/*******************************************************************************
 *
 * kit/system/libc/include/setjmp.h
 * - <setjmp.h>: 'stack environment declarations'
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

#ifndef _SETJMP_H
#define _SETJMP_H

#include <stddef.h>

struct _kit_jmp_buf
{
  size_t rbx, rsp, rbp, r12, r13, r14, r15;
};

typedef struct _kit_jmp_buf jmp_buf[1];

/**
 * Store the calling environment in the jmp_buf and return 0.
 *
 * If longjmp() is called later on the same jmp_buf, control flow starts again
 * from the corresponding setjmp() which will return `value || 1` instead.
 *
 * A setjmp() call that returns from a longjmp() always returns a non-zero
 * value.
 */
int setjmp(jmp_buf);

/**
 * Jump back to the calling environment saved in the given jmp_buf.
 *
 * The setjmp() call will return the given value if non-zero, or one.
 */
void longjmp(jmp_buf, int value);

#endif
