/*******************************************************************************
 *
 * kit/kernel/include/test.h
 * - runtime unit tests
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef TEST_H
#define TEST_H

#include <stdbool.h>

/* Run a testcase */
bool test_run(const char *name, bool (*testcase)());

/* Individual testcases */
bool test_memory_c();

#endif
