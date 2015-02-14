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
  PROCESS_STATE_SLEEPING,
  PROCESS_STATE_DEAD
} process_state_t;

typedef uint16_t process_id_t;

typedef struct process
{
  process_id_t         id;
  char                 name[256];
  process_state_t      state;
  paging_pageset_t     pageset;
  process_registers_t  registers;

  void                *kernel_stack_base;
  void                *kernel_stack_pointer;

  int                  exit_status;
  struct process      *waiting; // XXX

  // For scheduler use.
  struct {
    bool            waiting;
    struct process *run_queue_next;
  } sched;
} process_t;

process_t *process_current;

process_t *process_get(process_id_t id);

process_t *process_create(const char *name);

/**
 * Allocates 'length' bytes at 'address' (both aligned to the minimum page size)
 * and returns the adjusted pointer, if successful.
 *
 * Returns NULL on error.
 */
void *process_alloc(process_t *process, void *address, uint64_t length,
    paging_flags_t flags);

bool process_alloc_with_kernel(process_t *process, void *user_address,
    void *kernel_address, uint64_t length, paging_flags_t flags);

bool process_set_args(process_t *process, int argc, const char *const *argv);

void process_set_entry_point(process_t *process, void *instruction);

void process_switch(process_t *process);

void process_run(process_t *process);

#endif
