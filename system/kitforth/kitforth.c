/*******************************************************************************
 *
 * kit/system/kitforth/kitforth.c
 * - kitFORTH: a FORTH dialect for kit
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
#include <stdint.h>

#define UNUSED __attribute__((unused))

char line[4096];
uint64_t data_stack[512];

void interpret();

int main(UNUSED int argc, UNUSED char **argv) {
  int i = 0;

  while (!feof(stdin)) {
    i++;
    printf("\x1b[1;32m%i ok] \x1b[0;1m", i);
    fgets(line, 4096, stdin);
    fputs("\x1b[0m", stdout);
    interpret();
  }
  return 0;
}

struct execution_state {
  void (**ip)(); // instruction pointer
  void *dp;      // data pointer
};

// Callable with instruction pointer & data pointer
// Returns new data pointer
extern uint64_t *execute(void (**ip)(), uint64_t *dp);

// NOT CALLABLE! These are asm routines
extern void push();
extern void add();
extern void display();
extern void ret();

void interpret() {
  void (*code1[])() = {&push, (void (*)()) 4, &ret};
  void (*code2[])() = {&push, (void (*)()) 2, &ret};
  void (*code3[])() = {&add, &display, &ret};

  uint64_t *dp = data_stack + 512;

  dp = execute(code1, dp);
  dp = execute(code2, dp);
  dp = execute(code3, dp);
}

void printu64x(uint64_t n) {
  printf("%lx ", n);
}
