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

void upcase(char *str) {
  while (*str != '\0') {
    if (*str >= 'a' && *str <= 'z') {
      *str -= 0x20; // offset lower -> upper
    }
    str++;
  }
}

void init_dict();
void interpret();
void printdata();

int main(UNUSED int argc, UNUSED char **argv) {
  init_dict();
  while (!feof(stdin)) {
    printdata();
    printf("\x1b[1;32mok] \x1b[0;1m");
    char *res = fgets(line, 4096, stdin);
    fputs("\x1b[0m", stdout);
    if (res != NULL) interpret();
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

#define DICT_PRIMITIVE 1
#define DICT_CONSTANT  2

#define WORD_LENGTH 51

struct dict_entry {
  int  type; // 4 bytes
  char name[WORD_LENGTH + 1];
  union {
    void (*as_code)();
    void *as_ptr;
    uint64_t as_int;
  } value; // 8 bytes
};

struct dict_entry *dict;
int dict_len = 0;
int dict_cap = 0;

bool append_primitive(const char *name, void (*code)()) {
  if (dict_len < dict_cap) {
    dict[dict_len].type = DICT_PRIMITIVE;

    strncpy(dict[dict_len].name, name, WORD_LENGTH);
    dict[dict_len].name[WORD_LENGTH] = '\0';
    upcase(dict[dict_len].name);

    printf("PRIMITIVE %s = %p.\n", dict[dict_len].name, (void *) code);

    dict[dict_len].value.as_code = code;
    dict_len++;
    return 1;
  }
  else {
    puts("Dictionary is full!");
    return 0;
  }
}

bool append_constant(const char *name, uint64_t value) {
  if (dict_len < dict_cap) {
    dict[dict_len].type = DICT_CONSTANT;

    strncpy(dict[dict_len].name, name, WORD_LENGTH);
    dict[dict_len].name[WORD_LENGTH] = '\0';
    upcase(dict[dict_len].name);

    printf("CONSTANT  %s = %lx.\n", dict[dict_len].name, value);

    dict[dict_len].value.as_int = value;
    dict_len++;

    return 1;
  }
  else {
    puts("Dictionary is full!");
    return 0;
  }
}

void init_dict() {
  dict_cap = 512;

  dict = calloc(dict_cap, sizeof(struct dict_entry *));

  append_primitive("+",   &add);
  append_primitive("dup", &dup);
  append_primitive(".",   &display);
  append_primitive("cr",  &cr);

  append_constant("one", 1);
}

struct dict_entry *find_in_dict(char *word) {
  for (int i = dict_len - 1; i >= 0; i--) {
    if (strcmp(dict[i].name, word) == 0) {
      return &dict[i];
    }
  }

  return NULL;
}

void interpret() {
  char *in = line;

  void (*code[3])();
  char word[WORD_LENGTH + 1];

  while (*in != '\n') {
    int i;

    for (i = 0; *in != '\n' && *in != ' ' && i < WORD_LENGTH; i++, in++) {
      word[i] = *in;
    }
    word[i] = '\0';

    upcase(word);

    while (*in == ' ') in++;

    struct dict_entry *match;
    long number;
    char *endptr;

    if (strlen(word) == 0) {
      continue;
    }
    else if ((match = find_in_dict(word)) != NULL) {
      switch (match->type) {
        case DICT_PRIMITIVE:
          code[0] = match->value.as_code;
          code[1] = &ret;
          break;

        case DICT_CONSTANT:
          code[0] = &push;
          code[1] = match->value.as_code;
          code[2] = &ret;
          break;

        default:
          printf("Error: unknown dictionary entry type %i\n", match->type);
          return;
      }
    }
    else if (number = strtol(word, &endptr, 16), *endptr == '\0') {
      // Numeric word
      code[0] = &push;
      code[1] = (void (*)()) number;
      code[2] = &ret;
    }
    else {
      printf("Error: unknown word %s\n", word);
      return;
    }

    dp = execute(code, dp);

    if (dp > data_stack + 512) {
      puts("Stack underflow.");
      break;
    }
    else if (dp < data_stack) {
      puts("Stack overflow.");
      break;
    }
  }
}

void printu64x(uint64_t n) {
  printf("%lx ", n);
}

void printdata() {
  uint64_t *x;

  fputs("\x1b[1;33m", stdout);

  for (x = data_stack + 511; x >= dp; x--) {
    printu64x(*x);
  }

  fputs("\x1b[0m", stdout);
}
