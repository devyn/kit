/*******************************************************************************
 *
 * kit/kernel/include/syscall.h
 * - system call interface
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef SYSCALL_H
#define SYSCALL_H

#include <stdint.h>

#include "keyboard.h"
#include "process.h"

void syscall_initialize();

#define SYSCALL_EXIT 0x0
  int syscall_exit(int status);

#define SYSCALL_TWRITE 0x1
  int syscall_twrite(uint64_t length, const char *buffer);

#define SYSCALL_KEY_GET 0x2
  int syscall_key_get(keyboard_event_t *event);

#define SYSCALL_YIELD 0x3
  int syscall_yield();

#define SYSCALL_SLEEP 0x4
  int syscall_sleep();

#define SYSCALL_SPAWN 0x5
  int syscall_spawn(const char *file, int argc, const char *const *argv);

#define SYSCALL_WAIT_PROCESS 0x6
  int syscall_wait_process(process_id_t id, int *exit_status);

#ifdef SYSCALL_C
  const uint64_t syscall_table[] =
  {
    (uint64_t) &syscall_exit,
    (uint64_t) &syscall_twrite,
    (uint64_t) &syscall_key_get,
    (uint64_t) &syscall_yield,
    (uint64_t) &syscall_sleep,
    (uint64_t) &syscall_spawn,
    (uint64_t) &syscall_wait_process
  };
  const uint64_t syscall_table_size = sizeof(syscall_table)/8;
#else
  extern const uint64_t syscall_table[];
  extern const uint64_t syscall_table_size;
#endif

#endif
