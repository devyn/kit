/*******************************************************************************
 *
 * kit/system/util/echo.c
 * - prints its args to the terminal
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdint.h>
#include <stddef.h>

int syscall_twrite(uint64_t length, const char *buffer)
{
# define SYSCALL_TWRITE 0x1

  int ret;

  __asm__ volatile(
      "syscall"
      : "=a" (ret)
      : "a" (SYSCALL_TWRITE), "D" (length), "S" (buffer)
      : "%rcx", "%r11");

  return ret;
}

static inline size_t strlen(const char *s)
{
  size_t len = 0;

  while (s[len] != '\0') len++;

  return len;
}

int main(int argc, char **argv)
{
  for (int i = 1; i < argc; i++)
  {
    if (i > 1) syscall_twrite(1, " ");

    syscall_twrite(strlen(argv[i]), argv[i]);
  }

  syscall_twrite(1, "\n");

  return 0;
}
