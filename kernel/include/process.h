/*******************************************************************************
 *
 * kit/kernel/include/process.h
 * - process management functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PROCESS_H
#define PROCESS_H

#include <stdint.h>
#include <stdbool.h>

#include "paging.h"

typedef uint32_t process_id_t;

process_id_t process_current_id();

int process_wait_exit_status(process_id_t pid, int *exit_status);

/**
 * Adjusts the length of the current process's heap by 'amount' bytes and
 * returns a pointer to the new end of the heap.
 */
void *process_adjust_heap(int64_t amount);

void process_exit(int status);

#endif
