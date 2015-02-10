#ifndef _KIT_SHELL_IO_H
#define _KIT_SHELL_IO_H

#include <stdint.h>
#include <stddef.h>

static inline size_t strlen(const char *str)
{
  size_t size = 0;

  while (str[size] != '\0') size++;

  return size;
}

void tputc(char c);

void tputs(const char *str);

int tputu64(uint64_t integer, uint8_t base);
int tputi64( int64_t integer, uint8_t base);

size_t tgets(char *buffer, size_t size);

#define FORMAT_PRINTF(string_index, first_to_check) \
  __attribute__((__format__ (__printf__, string_index, first_to_check)))

FORMAT_PRINTF(1, 2) void tprintf(const char *format, ...);

#endif
