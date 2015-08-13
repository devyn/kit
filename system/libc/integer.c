/*******************************************************************************
 *
 * kit/system/libc/integer.c
 * - integer functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>

int abs(int n)
{
  return n & 0x7FFFFFF;
}
