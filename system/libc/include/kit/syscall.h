/*******************************************************************************
 *
 * kit/system/libc/include/kit/syscall.h
 * - system call helpers
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef _KIT_SYSCALL_H
#define _KIT_SYSCALL_H

#include <stdint.h>
#include <stdbool.h>

#define SYSCALL0(number, ret) \
  __asm__ volatile( \
      "syscall" \
      : "=a" (ret) \
      : "a" (number) \
      : "%rcx", "%r11")

#define SYSCALL1(number, ret, arg1) \
  __asm__ volatile( \
      "syscall" \
      : "=a" (ret) \
      : "a" (number), "D" (arg1) \
      : "%rcx", "%r11")

# define SYSCALL2(number, ret, arg1, arg2) \
  __asm__ volatile( \
      "syscall" \
      : "=a" (ret) \
      : "a" (number), "D" (arg1), "S" (arg2) \
      : "%rcx", "%r11")

# define SYSCALL3(number, ret, arg1, arg2, arg3) \
  __asm__ volatile( \
      "syscall" \
      : "=a" (ret) \
      : "a" (number), "D" (arg1), "S" (arg2), "d" (arg3) \
      : "%rcx", "%r11")

static inline int syscall_exit(int status)
{
# define SYSCALL_EXIT 0x0

  int ret;

  SYSCALL1(SYSCALL_EXIT, ret, status);

  return ret;
}

static inline int syscall_twrite(uint64_t length, const char *buffer)
{
# define SYSCALL_TWRITE 0x1

  int ret;

  SYSCALL2(SYSCALL_TWRITE, ret, length, buffer);

  return ret;
}

typedef struct keyboard_event
{
  uint8_t keycode;
  char    keychar; // ignore if '\0'

  bool    pressed    : 1;
  bool    ctrl_down  : 1;
  bool    alt_down   : 1;
  bool    shift_down : 1;
} keyboard_event_t;

static inline int syscall_key_get(keyboard_event_t *event)
{
# define SYSCALL_KEY_GET 0x2

  int ret;

  SYSCALL1(SYSCALL_KEY_GET, ret, event);

  return ret;
}

static inline int syscall_yield()
{
# define SYSCALL_YIELD 0x3

  int ret;

  SYSCALL0(SYSCALL_YIELD, ret);

  return ret;
}

static inline int syscall_sleep()
{
# define SYSCALL_SLEEP 0x4

  int ret;

  SYSCALL0(SYSCALL_SLEEP, ret);

  return ret;
}

static inline int64_t syscall_spawn(const char *file, int argc,
    const char *const *argv)
{
# define SYSCALL_SPAWN 0x5

  int64_t ret; // PID or error

  SYSCALL3(SYSCALL_SPAWN, ret, file, argc, argv);

  return ret;
}

static inline int syscall_wait_process(uint16_t id, int *exit_status)
{
# define SYSCALL_WAIT_PROCESS 0x6

  int ret;

  SYSCALL2(SYSCALL_WAIT_PROCESS, ret, id, exit_status);

  return ret;
}

static inline void *syscall_adjust_heap(int64_t amount)
{
# define SYSCALL_ADJUST_HEAP 0x7

  void *ret;

  SYSCALL1(SYSCALL_ADJUST_HEAP, ret, amount);

  return ret;
}

static inline void *syscall_mmap_archive()
{
# define SYSCALL_MMAP_ARCHIVE 0x8

  void *ret;

  SYSCALL0(SYSCALL_MMAP_ARCHIVE, ret);

  return ret;
}

#endif
