/*******************************************************************************
 *
 * kit/kernel/include/process.h
 * - process management functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PROCESS_H
#define PROCESS_H

#include <stdint.h>
#include <stdbool.h>

#include "paging.h"

void process_initialize();

typedef struct process_registers
{
  uint64_t rax, rcx, rdx, rbx;
  uint64_t rsp, rbp, rsi, rdi;
  uint64_t r8,  r9,  r10, r11;
  uint64_t r12, r13, r14, r15;

  uint64_t rip;
  uint32_t eflags;
} process_registers_t;

typedef enum process_state
{
  PROCESS_STATE_LOADING = 0,
  PROCESS_STATE_RUNNING,
  PROCESS_STATE_DEAD
} process_state_t;

typedef struct process
{
  uint16_t            id;
  char                name[256];
  process_state_t     state;
  paging_pageset_t    pageset;
  process_registers_t registers;
} process_t;

bool process_create(process_t *process, const char *name);

/**
 * Allocates 'length' bytes at 'address' (both aligned to the minimum page size)
 * and returns the adjusted pointer, if successful.
 *
 * Returns NULL on error.
 */
void *process_alloc(process_t *process, void *address, uint64_t length,
    paging_flags_t flags);

void process_set_entry_point(process_t *process, void *instruction);

void process_run(process_t *process);

#endif
