/*******************************************************************************
 *
 * kit/system/libc/exit.c
 * - cleanup functions and exit()
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>
#include <kit/syscall.h>

void exit(int status)
{
  // TODO: any cleanup
  _Exit(status);
}

void _Exit(int status)
{
  syscall_exit(status);
}
