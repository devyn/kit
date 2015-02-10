/*******************************************************************************
 *
 * kit/system/shell/io.c
 * - temporary I/O functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdbool.h>
#include <stdarg.h>

#include "io.h"
#include "syscall.h"

void tputc(char c)
{
  syscall_twrite(1, &c);
}

void tputs(const char *str)
{
  syscall_twrite(strlen(str), str);
}

/**
 * Can handle any base from binary up to sexatrigesimal (36), encompassing all
 * alphanumeric characters
 */
int tputu64(uint64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer == 0)
  {
    tputc('0');
    return 0;
  }

  char string[65];
  size_t position = 64;

  string[position] = '\0';

  while (integer > 0)
  {
    uint8_t digit = integer % base;

    if (digit < 10)
    {
      string[--position] = '0' + digit;
    }
    else
    {
      string[--position] = 'a' + (digit - 10);
    }

    integer = integer / base;
  }

  tputs(string + position);

  return 0;
}

/**
 * Signed variant of tputu64
 */
int tputi64(int64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer & ((uint64_t) 1 << 63))
  {
    // Negative
    tputc('-');
    return tputu64(~integer + 1, base);
  }
  else
  {
    // Positive
    return tputu64(integer, base);
  }
}

size_t tgets(char *buffer, size_t size)
{
  size_t index = 0;

  keyboard_event_t event;

  while (index < size - 1)
  {
    syscall_key_get(&event);

    if (event.pressed && event.keychar != 0)
    {
      if (event.keychar == '\b')
      {
        // Handle backspace only if there are characters to erase.
        if (index > 0)
        {
          tputc('\b');
          index--;
        }
      }
      else
      {
        tputc(event.keychar);
        buffer[index++] = event.keychar;

        if (event.keychar == '\n') break;
      }
    }
  }

  buffer[index] = '\0';

  return index;
}

typedef struct tprintf_state
{
  bool active;
  size_t start;
  bool alt_form;

  enum {
    TPRINTF_LENGTH_CHAR,
    TPRINTF_LENGTH_SHORT,
    TPRINTF_LENGTH_NORMAL,
    TPRINTF_LENGTH_LONG,
    TPRINTF_LENGTH_LONG_LONG
  } length;
} tprintf_state_t;

/**
 * Incomplete printf implementation.
 * See kit/kernel/terminal.c.
 */
FORMAT_PRINTF(1, 2) void tprintf(const char *format, ...)
{
  va_list args;

  tprintf_state_t state;

#define CLEAR_STATE() \
  { \
    state.active   = false; \
    state.start    = 0; \
    state.alt_form = false; \
    state.length   = TPRINTF_LENGTH_NORMAL; \
  }

#define FORMAT_NUM(fn, mod, base) \
  switch (state.length) { \
    case TPRINTF_LENGTH_CHAR: \
      {mod char val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TPRINTF_LENGTH_SHORT: \
      {mod short val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TPRINTF_LENGTH_NORMAL: \
      {mod int val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TPRINTF_LENGTH_LONG: \
      {mod long val = va_arg(args, mod long); \
      fn(val, (base));} \
      break; \
    case TPRINTF_LENGTH_LONG_LONG: \
      {mod long long val = va_arg(args, mod long long); \
      fn(val, (base));} \
      break; \
  }

#define INVALID_FORMAT() \
  for (size_t j = state.start; j <= i; j++) \
  { \
    tputc(format[j]); \
  }

  CLEAR_STATE();
  va_start(args, format);

  for (size_t i = 0;; i++)
  {
    if (!state.active) {
      switch (format[i])
      {
        case '\0':
          goto end;

        case '%':
          state.active = true;
          state.start  = i;
          break;

        default:
          tputc(format[i]);
      }
    }
    else
    {
      switch (format[i])
      {
        case '\0':
          // flush
          tputs(format + state.start);
          goto end;

        case '%':
          tputc('%');
          CLEAR_STATE();
          break;

        // set short/char
        case 'h':
          if (state.length == TPRINTF_LENGTH_NORMAL)
          {
            state.length = TPRINTF_LENGTH_SHORT;
          }
          else if (state.length == TPRINTF_LENGTH_SHORT)
          {
            state.length = TPRINTF_LENGTH_CHAR;
          }
          else
          {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        // set long/long long
        case 'l':
          if (state.length == TPRINTF_LENGTH_NORMAL)
          {
            state.length = TPRINTF_LENGTH_LONG;
          }
          else if (state.length == TPRINTF_LENGTH_LONG)
          {
            state.length = TPRINTF_LENGTH_LONG_LONG;
          }
          else
          {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        // set alternate form
        case '#':
          state.alt_form = true;
          break;

        // signed decimal
        case 'd':
        case 'i':
          FORMAT_NUM(tputi64, signed, 10);
          CLEAR_STATE();
          break;

        // unsigned octal
        case 'o':
          if (state.alt_form)
          {
            tputc('0');
          }
          FORMAT_NUM(tputu64, unsigned, 8);
          CLEAR_STATE();
          break;

        // unsigned decimal
        case 'u':
          FORMAT_NUM(tputu64, unsigned, 10);
          CLEAR_STATE();
          break;

        // unsigned hexadecimal
        case 'x':
          if (state.alt_form)
          {
            tputc('0');
            tputc('x');
          }
          FORMAT_NUM(tputu64, unsigned, 16);
          CLEAR_STATE();
          break;

        // char
        case 'c':
          tputc(va_arg(args, int));
          CLEAR_STATE();
          break;

        // string
        case 's':
          tputs(va_arg(args, char *));
          CLEAR_STATE();
          break;

        // pointer
        case 'p':
          tputs("0x");
          tputu64((uint64_t) va_arg(args, void *), 16);
          CLEAR_STATE();
          break;

        default:
          INVALID_FORMAT();
          CLEAR_STATE();
      }
    }
  }

end:
  va_end(args);
}
