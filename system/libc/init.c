/*******************************************************************************
 *
 * kit/system/libc/init.c
 * - standard library initialization functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <kit/syscall.h>

extern uint64_t  _libc_heap_length;
extern void     *_libc_heap_start;
extern void     *_libc_heap_end;

void _libc_init()
{
  _libc_heap_length = 0;

  _libc_heap_start = syscall_adjust_heap(0);
  _libc_heap_end   = _libc_heap_start;
}
