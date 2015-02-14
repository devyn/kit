/*******************************************************************************
 *
 * kit/system/util/yield.c
 * - spins and yields up to a count given in args
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
#include <stdbool.h>

const char USAGE[] =
  " Usage: util/yield <n>\n"
  " Where <n> is number of loop cycles to yield\n";

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

int syscall_yield()
{
# define SYSCALL_YIELD 0x3

  int ret;

  __asm__ volatile(
      "syscall"
      : "=a" (ret)
      : "a" (SYSCALL_YIELD)
      : "%rcx", "%r11");

  return ret;
}

unsigned long uatol(const char *nptr)
{
  int out = 0;

  for (; *nptr != '\0'; nptr++)
  {
    if (*nptr >= '0' && *nptr <= '9')
    {
      out = (out * 10) + (*nptr - '0');
    }
  }

  return out;
}

void printul(unsigned long n)
{
  if (n > 0)
  {
    char buf[64];
    int  index = 63;

    while (n > 0)
    {
      buf[index] = '0' + (char) (n % 10);
      n /= 10;
      index--;
    }

    syscall_twrite(64 - index - 1, &buf[index + 1]);
  }
  else
  {
    syscall_twrite(1, "0");
  }
}

int main(int argc, char **argv)
{
  if (argc != 2)
  {
    syscall_twrite(sizeof(USAGE) - 1, USAGE);
    return 1;
  }
  else
  {
    unsigned long cycles;

    for (cycles = uatol(argv[1]); cycles > 0; cycles--)
    {
      for (int i = 0; i < 40000000; i++) __asm__ volatile("mfence");

      syscall_twrite(6, "yield ");
      printul(cycles);
      syscall_twrite(1, "\n");

      syscall_yield();
    }

    return 0;
  }
}
