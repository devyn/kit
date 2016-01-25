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
#include <string.h>

#define UNUSED __attribute__((unused))

char line[4096];
uint64_t data_stack[512];

uint64_t *dp = data_stack + 512;

void interpret();

int main(UNUSED int argc, UNUSED char **argv) {
  while (!feof(stdin)) {
    printf("\x1b[1;32mok] \x1b[0;1m");
    fgets(line, 4096, stdin);
    fputs("\x1b[0m", stdout);
    interpret();
  }
  return 0;
}

// Callable with instruction pointer & data pointer
// Returns new data pointer
extern uint64_t *execute(void (**ip)(), uint64_t *dp);

// NOT CALLABLE! These are asm routines
extern void push();
extern void add();
extern void dup();
extern void display();
extern void cr();
extern void ret();

void (*code_buffer[512])();

void printdata();

void interpret() {
  char *in = line;

  void (*code[3])();
  char word[32];

  while (*in != '\n') {
    int i;

    for (i = 0; *in != '\n' && *in != ' ' && i < 31; i++, in++) {
      word[i] = *in;
    }
    word[i] = '\0';

    while (*in == ' ') in++;

    if (strlen(word) == 0) {
      continue;
    }
    else if (strcmp(word, "+") == 0) {
      code[0] = &add;
      code[1] = &ret;
    }
    else if (strcmp(word, "dup") == 0) {
      code[0] = &dup;
      code[1] = &ret;
    }
    else if (strcmp(word, ".") == 0) {
      code[0] = &display;
      code[1] = &ret;
    }
    else if (strcmp(word, "cr") == 0) {
      code[0] = &cr;
      code[1] = &ret;
    }
    else if (strcmp(word, "1") == 0) {
      code[0] = &push;
      code[1] = (void (*)()) 1;
      code[2] = &ret;
    }
    else {
      printf("Error: unknown word %s\n", word);
      continue;
    }

    dp = execute(code, dp);
  }

  printdata();
}

void printu64x(uint64_t n) {
  printf("%lx ", n);
}

void printdata() {
  uint64_t *x;

  fputs("\x1b[1;33m", stdout);

  for (x = dp; x < data_stack + 512; x++) {
    printu64x(*x);
  }

  fputs("\x1b[0m", stdout);
}
