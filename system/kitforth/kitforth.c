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
char *in = NULL;

#define DATA_STACK_SIZE 512
#define DATA_STACK_SAFE 504

uint64_t data_stack[DATA_STACK_SIZE];

uint64_t *dp = data_stack + DATA_STACK_SAFE;

void upcase(char *str) {
  while (*str != '\0') {
    if (*str >= 'a' && *str <= 'z') {
      *str -= 0x20; // offset lower -> upper
    }
    str++;
  }
}

void init_dict();
void consume_line();
void printdata();

int main(UNUSED int argc, UNUSED char **argv) {
  init_dict();
  while (!feof(stdin)) {
    printdata();
    printf("\x1b[1;32mok] \x1b[0;1m");
    char *res = fgets(line, 4096, stdin);
    fputs("\x1b[0m", stdout);
    if (res != NULL) {
      in = line;
      consume_line();
    }
  }
  return 0;
}

// Callable with instruction pointer & data pointer
// Returns new data pointer
extern uint64_t *execute(void (**ip)(), uint64_t *dp);

#define DICT_TYPE_PRIMITIVE 0x01
#define DICT_TYPE_CODE      0x02
#define DICT_TYPE_CONSTANT  0x03

#define DICT_FLAG_IMMEDIATE 0x01

#define WORD_LENGTH 47

struct dict_entry {
  uint16_t type; // 2 bytes
  uint16_t flags; // 2 bytes
  uint16_t len; // 2 bytes
  uint16_t cap; // 2 bytes

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

struct dict_entry *last_word = NULL;

bool append_primitive(const char *name, void (*code)()) {
  if (dict_len < dict_cap) {
    dict[dict_len].type = DICT_TYPE_PRIMITIVE;
    dict[dict_len].flags = 0;

    strncpy(dict[dict_len].name, name, WORD_LENGTH);
    dict[dict_len].name[WORD_LENGTH] = '\0';
    upcase(dict[dict_len].name);

    dict[dict_len].value.as_code = code;

    last_word = &dict[dict_len];

    printf("PRIMITIVE %s = %p.\n", last_word->name, last_word->value.as_ptr);

    dict_len++;
    return true;
  }
  else {
    puts("Dictionary is full!");
    return false;
  }
}

#define CODE_SIZE 256

bool append_code(const char *name) {
  if (dict_len < dict_cap) {
    dict[dict_len].type = DICT_TYPE_CODE;
    dict[dict_len].flags = 0;
    dict[dict_len].len = 0;
    dict[dict_len].cap = CODE_SIZE;

    strncpy(dict[dict_len].name, name, WORD_LENGTH);
    dict[dict_len].name[WORD_LENGTH] = '\0';
    upcase(dict[dict_len].name);

    dict[dict_len].value.as_ptr = calloc(CODE_SIZE, sizeof(void (*)()));

    last_word = &dict[dict_len];

    printf("CODE      %s = %p.\n", last_word->name, last_word->value.as_ptr);

    dict_len++;
    return true;
  }
  else {
    puts ("Dictionary is full!");
    return false;
  }
}

bool append_constant(const char *name, uint64_t value) {
  if (dict_len < dict_cap) {
    dict[dict_len].type = DICT_TYPE_CONSTANT;
    dict[dict_len].flags = 0;

    strncpy(dict[dict_len].name, name, WORD_LENGTH);
    dict[dict_len].name[WORD_LENGTH] = '\0';
    upcase(dict[dict_len].name);

    dict[dict_len].value.as_int = value;

    printf("CONSTANT  %s = %lx.\n", dict[dict_len].name, value);

    last_word = &dict[dict_len];

    dict_len++;
    return true;
  }
  else {
    puts("Dictionary is full!");
    return false;
  }
}

void immediate() {
  last_word->flags |= DICT_FLAG_IMMEDIATE;
}

// NOT CALLABLE! These are asm routines
extern void push();
extern void add();
extern void dup();
extern void display();
extern void emit();
extern void in_char();
extern void compiler_off();
extern void compiler_on();
extern void call();
extern void ret();
extern void ret_quit();
extern void literal_stub();
extern void postpone_stub();
extern void immediate_stub();
extern void defword_stub();
extern void endword_stub();

void init_dict() {
  dict_cap = 512;

  dict = calloc(dict_cap, sizeof(struct dict_entry *));

  append_primitive("+",         &add);
  append_primitive("dup",       &dup);
  append_primitive(".",         &display);
  append_primitive("emit",      &emit);
  append_primitive("char",      &in_char);
  append_primitive("[",         &compiler_off); immediate();
  append_primitive("]",         &compiler_on);
  append_primitive("literal",   &literal_stub); immediate();
  append_primitive("postpone",  &postpone_stub); immediate();
  append_primitive("immediate", &immediate_stub);
  append_primitive(":",         &defword_stub);
  append_primitive(";",         &endword_stub); immediate();

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

bool read_word(char *word) {
  int i;

  for (i = 0;
       (*in != '\0' &&
        *in != '\n' &&
        *in != ' ' &&
        i < WORD_LENGTH);
       i++, in++) {

    word[i] = *in;
  }
  word[i] = '\0';

  return i != 0;
}

int read_charword() {
  char word[WORD_LENGTH + 1];

  while (*in == ' ');

  if (!read_word(word)) {
    return 0;
  }
  else {
    return word[0];
  }
}

int compiling = 0;

void interpret(char *word);
void compile(char *word);

void consume_line() {
  char word[WORD_LENGTH + 1];

  while (*in != '\0' && *in != '\n') {
    while (*in == ' ') in++;

    if (!read_word(word)) continue;

    upcase(word);

    while (*in == ' ') in++;

    if (compiling) {
      compile(word);
    }
    else {
      interpret(word);
    }
  }
}

void interpret(char *word) {
  void (*code[3])();

  struct dict_entry *match;
  long number;
  char *endptr;

  if ((match = find_in_dict(word)) != NULL) {
    switch (match->type) {
      case DICT_TYPE_PRIMITIVE:
        code[0] = match->value.as_code;
        code[1] = &ret_quit;
        break;

      case DICT_TYPE_CODE:
        code[0] = &call;
        code[1] = match->value.as_code;
        code[2] = &ret_quit;
        break;

      case DICT_TYPE_CONSTANT:
        code[0] = &push;
        code[1] = match->value.as_code;
        code[2] = &ret_quit;
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
    code[2] = &ret_quit;
  }
  else {
    printf("Error: unknown word %s\n", word);
    return;
  }

  dp = execute(code, dp);

  if (dp > data_stack + DATA_STACK_SAFE) {
    puts("Stack underflow.");
    dp = data_stack + DATA_STACK_SAFE;
    while (*in != '\0');
  }
  else if (dp <= data_stack) {
    puts("Stack overflow.");
    dp = data_stack + 1;
    while (*in != '\0');
  }
}

bool disable_immediate = false;

void compile(char *word) {
  void (**code)() = (void (**)()) last_word->value.as_ptr;

  struct dict_entry *match;
  long number;
  char *endptr;

  if ((match = find_in_dict(word)) != NULL) {
    if (!disable_immediate && match->flags & DICT_FLAG_IMMEDIATE) {
      interpret(word);
    }
    else {
      switch (match->type) {
        case DICT_TYPE_PRIMITIVE:
          code[last_word->len++] = match->value.as_code;
          break;

        case DICT_TYPE_CODE:
          code[last_word->len++] = &call;
          code[last_word->len++] = match->value.as_code;
          break;

        case DICT_TYPE_CONSTANT:
          code[last_word->len++] = &push;
          code[last_word->len++] = match->value.as_code;
          break;

        default:
          printf("Error: unknown dictionary entry type %i\n", match->type);
          return;
      }
    }
  }
  else if (number = strtol(word, &endptr, 16), *endptr == '\0') {
    // Numeric word
    code[last_word->len++] = &push;
    code[last_word->len++] = (void (*)()) number;
  }
  else {
    printf("Error: unknown word %s\n", word);
    return;
  }
}

void literal(uint64_t value) {
  void (**code)() = last_word->value.as_ptr;

  code[last_word->len++] = &push;
  code[last_word->len++] = (void (*)()) value;
}

void postpone() {
  char word[WORD_LENGTH + 1];

  if (!read_word(word)) return;

  upcase(word);

  disable_immediate = true;
  compile(word);
  disable_immediate = false;
}

void defword() {
  char word[WORD_LENGTH + 1];

  if (!read_word(word)) return;

  append_code(word);
  compiling = 1;
}

void dumpptrarray(void **ptr, size_t len) {
  for (size_t i = 0; i < len; i++) {
    printf("%p ", ptr[i]);
  }
  puts("");
}

void endword() {
  void (**code)() = last_word->value.as_ptr;

  code[last_word->len++] = &ret;

  dumpptrarray((void **) code, last_word->len);

  compiling = 0;
}

void printu64x(uint64_t n) {
  printf("%lx ", n);
}

void printdata() {
  uint64_t *x;

  fputs("\x1b[1;33m", stdout);

  for (x = data_stack + DATA_STACK_SAFE - 1; x >= dp; x--) {
    printu64x(*x);
  }

  fputs("\x1b[0m", stdout);
}
