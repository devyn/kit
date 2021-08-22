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

#include "engine.h"
#include "boot.h"

#define UNUSED __attribute__((unused))

char line[4096];
char *in = NULL;
uint64_t in_length = 0;

#define DATA_STACK_SIZE 512
#define DATA_STACK_SAFE 504

uint64_t data_stack[DATA_STACK_SIZE];

uint64_t *dp = data_stack + DATA_STACK_SAFE;

#define DATA_SPACE_SIZE 65536

char *here;
char *there;

void upcase(char *str) {
  while (*str != '\0') {
    if (*str >= 'a' && *str <= 'z') {
      *str -= 0x20; // offset lower -> upper
    }
    str++;
  }
}

void skip(char c) {
  while (*in == (c) && in_length > 0) {
    in++;
    in_length--;
  }
}

void init_dict();
void consume_line();
void printdata();

bool readline() {
  int c;

  char *buf = line;
  
  while (true) {
    c = getchar();

    if (c == EOF) {
      return false;
    }
    else if (c == '\n') {
      *buf++ = c;
      break;
    }
    else if (c == '\b') {
      if (buf > line) {
        buf--;
        putchar('\b');
      }
    }
    else {
      *buf++ = c;
      putchar(c);
    }

    if (buf - line >= 4095) {
      break;
    }
  }

  *buf++ = '\0';
  return true;
}

bool ok;

int main(UNUSED int argc, UNUSED char **argv) {
  here = calloc(1, DATA_SPACE_SIZE);
  there = here + DATA_SPACE_SIZE;
  init_dict();

  while (!feof(stdin)) {
    putchar('\n');
    printdata();
    printf("\x1b[1;33m> \x1b[0;1m");
    ok = readline();
    fputs("\x1b[0m", stdout);
    if (ok) {
      in = line;
      in_length = strlen(line);
      consume_line();
    }
    if (ok) {
      fputs("\x1b[1;32m ok\x1b[0m", stdout);
    }
  }
  return 0;
}

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

void init_dict() {
  dict_cap = 1024;

  dict = calloc(dict_cap, sizeof(struct dict_entry));

  append_primitive("see",       &see_stub);
  append_primitive("dump",      &dump_stub);
  append_primitive("evaluate",  &evaluate_stub);

  append_primitive("+",         &add);
  append_primitive("-",         &sub);
  append_primitive("*",         &mul);
  append_primitive("/mod",      &divmod);

  append_primitive("xor",       &bit_xor);
  append_primitive("and",       &bit_and);
  append_primitive("or",        &bit_or);
  append_primitive("lshift",    &bit_lshift);
  append_primitive("rshift",    &bit_rshift);

  append_primitive("=",         &equal);
  append_primitive(">",         &gt);
  append_primitive(">=",        &gte);
  append_primitive("<",         &lt);
  append_primitive("<=",        &lt);

  append_primitive("@",         &fetch);
  append_primitive("!",         &store);
  append_primitive("c@",        &fetch_char);
  append_primitive("c!",        &store_char);
  append_primitive("move",      &move);
  append_primitive("allocate",  &allocate_f);
  append_primitive("free",      &free_f);
  append_primitive("resize",    &resize_f);

  append_primitive("dup",       &dup);
  append_primitive("swap",      &swap);
  append_primitive("over",      &over);
  append_primitive("rot",       &rot);
  append_primitive("drop",      &drop);
  append_primitive(">r",        &to_rstack);
  append_primitive("r>",        &from_rstack);
  append_primitive("r@",        &fetch_rstack);

  append_primitive("state",     &state);
  append_primitive("cp",        &cp_stub);
  append_primitive("cp,",       &cp_comma_stub);
  append_primitive("branch",    &branch);
  append_primitive("?branch",   &branch_if_zero);

  append_primitive("sp@",       &get_stack_ptr);

  append_primitive(".",         &display);
  append_primitive("emit",      &emit);
  append_primitive("char",      &in_char);
  append_primitive("(string)",  &get_string);

  append_primitive("[",         &compiler_off); immediate();
  append_primitive("]",         &compiler_on);

  append_primitive("literal",   &literal_stub); immediate();
  append_primitive("postpone",  &postpone_stub); immediate();
  append_primitive("immediate", &immediate_stub);
  append_primitive("create",    &create_stub);
  append_primitive(":",         &defword_stub);
  append_primitive(";",         &endword_stub); immediate();

  append_primitive("parse",     &parse_stub);

  append_primitive("syscall",   &syscall_from_forth);

  append_constant("false", 0);
  append_constant("true", ~0);

  append_constant("(here)",  (uint64_t) &here);
  append_constant("(there)", (uint64_t) &there);

  // None of these should ever be invoked directly.
  append_primitive("(push)", &push);
  append_primitive("(call)", &call);
  append_primitive("(ret)",  &ret);

  in = boot_source;
  in_length = strlen(boot_source);
  ok = true;

  while (ok && in_length > 0) {
    consume_line();
  }
}

struct dict_entry *find_in_dict(char *word) {
  for (int i = dict_len - 1; i >= 0; i--) {
    if (strcmp(dict[i].name, word) == 0) {
      return &dict[i];
    }
  }

  return NULL;
}

struct dict_entry *find_val_in_dict(uint64_t value) {
  for (int i = dict_len - 1; i >= 0; i--) {
    if (dict[i].value.as_int == value) {
      return &dict[i];
    }
  }

  return NULL;
}

bool read_word(char *word) {
  int i;

  for (i = 0;
       (in_length > 0 &&
        *in != '\n' &&
        *in != ' ' &&
        i < WORD_LENGTH);
       i++, in++, in_length--) {

    word[i] = *in;
  }
  word[i] = '\0';

  return i != 0;
}

int read_charword() {
  char word[WORD_LENGTH + 1];

  skip(' ');

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

  while (ok && in_length > 0 && *in != '\n') {
    skip(' ');

    if (!read_word(word)) continue;

    upcase(word);

    skip(' ');

    if (compiling) {
      compile(word);
    }
    else {
      interpret(word);
    }
  }

  if (in_length > 0 && *in == '\n') {
    in++;
    in_length--;
  }

  if (!ok) {
    // Reset stack pointer on error.
    dp = data_stack + DATA_STACK_SAFE;
  }
}

// For nested use.
void evaluate(char *addr, uint64_t len) {
  char *old_in = in;
  uint64_t old_in_length = in_length;

  in = addr;
  in_length = len;

  while (ok && in_length > 0) {
    consume_line();
  }

  in = old_in;
  in_length = old_in_length;
}

void interpret_dict_entry(struct dict_entry *entry) {
  void (*code[3])();

  switch (entry->type) {
    case DICT_TYPE_PRIMITIVE:
      code[0] = entry->value.as_code;
      code[1] = &ret_quit;
      break;

    case DICT_TYPE_CODE:
      code[0] = &call;
      code[1] = entry->value.as_code;
      code[2] = &ret_quit;
      break;

    case DICT_TYPE_CONSTANT:
      code[0] = &push;
      code[1] = entry->value.as_code;
      code[2] = &ret_quit;
      break;

    default:
      printf(" \x1b[1;31munknown dictionary entry type %i\x1b[0m", entry->type);
      ok = false;
      return;
  }

  dp = execute(code, dp);
}

void interpret(char *word) {
  struct dict_entry *match;
  long number;
  char *endptr;

  if ((match = find_in_dict(word)) != NULL) {
    interpret_dict_entry(match);
  }
  else if (number = strtol(word, &endptr, 10), *endptr == '\0') {
    dp -= 1;
    *((uint64_t *) dp) = number;
  }
  else {
    printf(" \x1b[1;31munknown word %s\x1b[0m", word);
    ok = false;
    return;
  }

  if (dp > data_stack + DATA_STACK_SAFE) {
    fputs(" \x1b[1;31mstack underflow\x1b[0m", stderr);
    dp = data_stack + DATA_STACK_SAFE;
    ok = false;
  }
  else if (dp <= data_stack) {
    fputs(" \x1b[1;31mstack overflow\x1b[0m", stderr);
    dp = data_stack + 1;
    ok = false;
  }
}

void compile_dict_entry(struct dict_entry *entry) {
  void (**code)() = (void (**)()) last_word->value.as_ptr;

  if (entry->flags & DICT_FLAG_IMMEDIATE) {
    interpret_dict_entry(entry);
  }
  else {
    switch (entry->type) {
      case DICT_TYPE_PRIMITIVE:
        code[last_word->len++] = entry->value.as_code;
        break;

      case DICT_TYPE_CODE:
        code[last_word->len++] = &call;
        code[last_word->len++] = entry->value.as_code;
        break;

      case DICT_TYPE_CONSTANT:
        code[last_word->len++] = &push;
        code[last_word->len++] = entry->value.as_code;
        break;

      default:
        printf(" \x1b[1;31munknown dictionary entry type %i\x1b[0m", entry->type);
    }
  }
}

void compile(char *word) {
  void (**code)() = (void (**)()) last_word->value.as_ptr;

  struct dict_entry *match;
  long number;
  char *endptr;

  if ((match = find_in_dict(word)) != NULL) {
    compile_dict_entry(match);
  }
  else if (number = strtol(word, &endptr, 10), *endptr == '\0') {
    // Numeric word
    code[last_word->len++] = &push;
    code[last_word->len++] = (void (*)()) number;
  }
  else {
    printf(" \x1b[1;31munknown word %s\x1b[0m", word);
    ok = false;
    return;
  }
}

void literal(uint64_t value) {
  void (**code)() = last_word->value.as_ptr;

  code[last_word->len++] = &push;
  code[last_word->len++] = (void (*)()) value;
}

void postpone() {
  void (**code)() = last_word->value.as_ptr;

  struct dict_entry *match;
  char word[WORD_LENGTH + 1];

  if (!read_word(word)) return;

  upcase(word);

  if ((match = find_in_dict(word)) != NULL) {
    code[last_word->len++] = &postponed;
    code[last_word->len++] = (void (*)()) match;
  }
  else {
    printf(" \x1b[1;31munknown word %s\x1b[0m", word);
    ok = false;
    return;
  }
}

void *cp() {
  return (void (**)()) last_word->value.as_ptr + last_word->len;
}

void cp_comma(uint64_t value) {
  uint64_t *code = last_word->value.as_ptr;

  code[last_word->len++] = value;
}

void create() {
  char word[WORD_LENGTH + 1];

  if (!read_word(word)) return;

  append_constant(word, (uint64_t) here);
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
  putchar('\n');
}

void colortype(int type) {
  switch (type) {
  case DICT_TYPE_PRIMITIVE:
    fputs("\x1b[1;33m", stdout);
    break;
  case DICT_TYPE_CONSTANT:
    fputs("\x1b[1;36m", stdout);
    break;
  case DICT_TYPE_CODE:
    fputs("\x1b[1;35m", stdout);
    break;
  }
}

void dumpcode(void **ptr, size_t len) {
  for (size_t i = 0; i < len; i++) {
    printf("\n\x1b[32m%4lu:\x1b[0m %16lx \x1b[1;30m\"", i, (uint64_t) ptr[i]);

    for (int j = 0; j < 8; j++) {
      char c = ((char *) (ptr + i))[j];

      putchar(c < 32 ? '.' : c);
    }
    fputs("\"\x1b[0m", stdout);

    struct dict_entry *entry;

    if ((entry = find_val_in_dict((uint64_t) ptr[i])) != NULL) {
      colortype(entry->type);
      printf(" %s\x1b[0m", entry->name);
    }
    else if (ptr[i] > (void *)ptr && ptr[i] < (void *)(ptr + len)) {
      printf("\x1b[32m ref %lu:\x1b[0m",
             ((uint64_t) ptr[i] - (uint64_t) ptr)/8);
    }
  }
}

void endword() {
  void (**code)() = last_word->value.as_ptr;

  code[last_word->len++] = &ret;

  //dumpptrarray((void **) code, last_word->len);

  compiling = 0;
  dict_len++;
}

void see_word(char *word) {
  struct dict_entry *entry = find_in_dict(word);

  if (entry != NULL) {
    colortype(entry->type);
    printf("\n%s\x1b[0m", entry->name);

    switch (entry->type) {
    case DICT_TYPE_PRIMITIVE:
      fputs(" primitive", stdout);
      if (entry->flags & DICT_FLAG_IMMEDIATE) {
        fputs(" immediate", stdout);
      }
      printf(" = %p", entry->value.as_ptr);
      break;

    case DICT_TYPE_CONSTANT:
      printf(" constant = %ld", entry->value.as_int);
      break;

    case DICT_TYPE_CODE:
      fputs(" code", stdout);
      if (entry->flags & DICT_FLAG_IMMEDIATE) {
        fputs(" immediate", stdout);
      }
      printf(" = %p (%u cells)", entry->value.as_ptr, entry->len);
      dumpcode((void **) entry->value.as_ptr, entry->len);
      break;

    default:
      printf(" bugged!");
    }
  }
  else {
    printf("\n%s not defined", word);
  }
}

void see() {
  char word[WORD_LENGTH + 1];

  if (!read_word(word)) return;

  upcase(word);

  see_word(word);
}

void dump(char *ptr, uint64_t len) {
  // XXX: lol this is so bad

  char *cur = ptr;
  uint64_t counter = 0;

  while (cur < ptr + len) {
    if (counter == 16) {
      fputs("\x1b[1;30m ", stdout);
      for (char *cur_inner = cur - counter; cur_inner < cur; cur_inner++) {
        putchar(*cur_inner >= 32 ? *cur_inner : '.');
      }
      fputs("\x1b[0m", stdout);
      counter = 0;
    }

    if (counter == 0) {
      putchar('\n');
    }

    if (counter % 2 == 0) putchar(' ');
    printf("%02hhx", *cur);

    cur++;
    counter++;
  }

  if (counter < 16) {
    for (uint64_t i = 0; i < 16 - counter; i++) {
      if ((counter + i) % 2 == 0) putchar(' ');
      putchar(' ');
      putchar(' ');
    }
  }

  fputs("\x1b[1;30m ", stdout);
  for (char *cur_inner = cur - counter; cur_inner < cur; cur_inner++) {
    putchar(*cur_inner >= 32 ? *cur_inner : '.');
  }
  fputs("\x1b[0m", stdout);
}

uint64_t parse(char delimiter, char **addr) {
  *addr = in;

  while (in_length > 0 && *in != delimiter) {
    in++;
    in_length--;
  }

  uint64_t len = in - *addr;

  if (in_length > 0 && *in == delimiter) {
    in++;
    in_length--;
  }

  return len;
}

void printi64(int64_t n) {
  printf(" %ld", n);
}

void printdata() {
  uint64_t *x;

  fputs("\x1b[1;33m", stdout);

  for (x = data_stack + DATA_STACK_SAFE - 1; x >= dp; x--) {
    printi64(*x);
  }

  fputs("\x1b[0m", stdout);
}
