#include "x86_64.h"

void rep_stosb(void *pointer, uint8_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosb"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}

void rep_stosq(void *pointer, uint64_t value, size_t count)
{
  int d0, d1; // black holes

  __asm__ volatile("cld; rep stosq"
                  : "=&D" (d0), "=&c" (d1)
                  : "0" (pointer), "a" (value), "1" (count)
                  : "memory");
}
