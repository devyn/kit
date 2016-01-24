/*******************************************************************************
 *
 * kit/system/kforth/kforth.c
 * - kFORTH: a FORTH dialect for kit
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdio.h>

#define UNUSED __attribute__((unused))

int main(UNUSED int argc, UNUSED char **argv) {
  char line[1024];

  printf("ok] ");
  fgets(line, 1024, stdin);
  printf("So sorry. We aren't ready yet. Check back later.\n");
  return 0;
}
