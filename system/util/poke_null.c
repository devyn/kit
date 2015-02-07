/*******************************************************************************
 *
 * kit/system/util/poke_null.c
 * - pokes NULL in order to cause a critical page fault
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#define UNUSED __attribute__((__unused__))

int main(UNUSED int argc, UNUSED char **argv)
{
  __asm__("movb $0, 0"); // clang liked to turn my thing into UD2

  return 1;
}
