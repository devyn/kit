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

void syscall_initialize();

#define SYSCALL_EXIT 0x0
  // emulated

#define SYSCALL_TWRITE 0x1
  int syscall_twrite(uint64_t length, const char *buffer);

#define SYSCALL_KEY_GET 0x2
  int syscall_key_get(keyboard_event_t *event);

#ifdef SYSCALL_C
  const uint64_t syscall_table[] =
  {
    0, // SYSCALL_EXIT
    (uint64_t) &syscall_twrite,
    (uint64_t) &syscall_key_get,
  };
  const uint64_t syscall_table_size = sizeof(syscall_table)/8;
#else
  extern const uint64_t syscall_table[];
  extern const uint64_t syscall_table_size;
#endif

#endif
