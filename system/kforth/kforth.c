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
#include <stdlib.h>
#include <stdbool.h>

#define UNUSED __attribute__((unused))

int main(UNUSED int argc, UNUSED char **argv) {
  char line[1024];
  int i = 0;

  while (!feof(stdin)) {
    i++;
    printf("\x1b[1;32m%i ok] \x1b[0;1m", i);
    fgets(line, 1024, stdin);
  }
  printf("\x1b[0;31mSo sorry. We aren't ready yet. Check back later.\x1b[0m\n");
  return 0;
}
