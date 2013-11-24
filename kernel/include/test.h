#ifndef TEST_H
#define TEST_H

#include <stdbool.h>

/* Run a testcase */
bool test_run(const char *name, bool (*testcase)());

/* Individual testcases */
bool test_memory_c();

#endif
