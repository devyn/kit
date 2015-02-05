/*******************************************************************************
 *
 * kit/kernel/include/test.h
 * - runtime unit tests
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef TEST_H
#define TEST_H

#include <stdbool.h>
#include <stddef.h>

typedef struct test_unit
{
  char *name;
  bool (*run)();
} test_unit_t;

/* Run a test unit */
bool test_run(const test_unit_t *unit);

/* Individual test units */
bool test_memory_c();
bool test_interrupt_c();
bool test_rbtree_c();
bool test_paging_c();
bool test_elf_c();

/* List of test units */
#ifdef TEST_C
  const test_unit_t test_units[] = {
    {"memory.c",     &test_memory_c},
    {"interrupt.c",  &test_interrupt_c},
    {"rbtree.c",     &test_rbtree_c},
    {"paging.c",     &test_paging_c},
    {"elf.c",        &test_elf_c}
  };

  const size_t TEST_UNITS_SIZE =
    sizeof(test_units) / sizeof(test_unit_t);

#else
  extern const test_unit_t test_units[];
  extern const size_t TEST_UNITS_SIZE;
#endif

/* Run everything */
bool test_all();

#endif
