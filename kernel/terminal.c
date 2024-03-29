/*******************************************************************************
 *
 * kit/kernel/terminal.c
 * - early text mode 80x25 terminal handler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "terminal.h"
#include "x86_64.h"
#include "config.h"

uint8_t terminal_make_color(enum vga_color fg, enum vga_color bg)
{
  return fg | bg << 4;
}

extern void terminal_initialize();

extern void terminal_clear();

extern void terminal_updatecursor();

extern void terminal_getcursor(size_t *row, size_t *column);

extern void terminal_setcursor(size_t row, size_t column);

extern void terminal_getcolor(enum vga_color *fg, enum vga_color *bg);

extern void terminal_setcolor(enum vga_color fg, enum vga_color bg);

extern void terminal_putentryat(char c, uint8_t color, size_t x, size_t y);

extern void terminal_newline();

extern void terminal_writechar_internal(char c);

extern void terminal_writechar(char c);

extern void terminal_writebuf(uint64_t length, const char *buffer);

extern void terminal_writestring(const char *data);

/**
 * Can handle any base from binary up to sexatrigesimal (36), encompassing all
 * alphanumeric characters
 */
int terminal_writeuint64(uint64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer == 0)
  {
    terminal_writechar('0');
    return 0;
  }

  char string[65];
  ptrdiff_t position = 64;

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

  terminal_writestring(string + position);

  return 0;
}

/**
 * Signed variant of terminal_writeuint64
 */
int terminal_writeint64(int64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer & ((uint64_t) 1 << 63))
  {
    // Negative
    terminal_writechar_internal('-');
    return terminal_writeuint64(~integer + 1, base);
  }
  else
  {
    // Positive
    return terminal_writeuint64(integer, base);
  }
}

typedef struct terminal_printf_state
{
  bool active;
  ptrdiff_t start;
  bool alt_form;

  enum {
    TERMINAL_LENGTH_CHAR,
    TERMINAL_LENGTH_SHORT,
    TERMINAL_LENGTH_NORMAL,
    TERMINAL_LENGTH_LONG,
    TERMINAL_LENGTH_LONG_LONG
  } length;
} terminal_printf_state_t;

/**
 * Incomplete printf implementation.
 */
FORMAT_PRINTF(1, 2) void terminal_printf(const char *format, ...)
{
  va_list args;

  terminal_printf_state_t state;

#define CLEAR_STATE() \
  { \
    state.active   = false; \
    state.start    = 0; \
    state.alt_form = false; \
    state.length   = TERMINAL_LENGTH_NORMAL; \
  }

#define FORMAT_NUM(fn, mod, base) \
  switch (state.length) { \
    case TERMINAL_LENGTH_CHAR: \
      {mod char val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TERMINAL_LENGTH_SHORT: \
      {mod short val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TERMINAL_LENGTH_NORMAL: \
      {mod int val = va_arg(args, mod int); \
      fn(val, (base));} \
      break; \
    case TERMINAL_LENGTH_LONG: \
      {mod long val = va_arg(args, mod long); \
      fn(val, (base));} \
      break; \
    case TERMINAL_LENGTH_LONG_LONG: \
      {mod long long val = va_arg(args, mod long long); \
      fn(val, (base));} \
      break; \
  }

#define INVALID_FORMAT() \
  for (ptrdiff_t j = state.start; j <= i; j++) \
  { \
    terminal_writechar_internal(format[j]); \
  }

  CLEAR_STATE();
  va_start(args, format);

  for (ptrdiff_t i = 0;; i++)
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
          terminal_writechar_internal(format[i]);
      }
    }
    else
    {
      switch (format[i])
      {
        case '\0':
          // flush
          terminal_writestring(format + state.start);
          goto end;

        case '%':
          terminal_writechar_internal('%');
          CLEAR_STATE();
          break;

        // set short/char
        case 'h':
          if (state.length == TERMINAL_LENGTH_NORMAL)
          {
            state.length = TERMINAL_LENGTH_SHORT;
          }
          else if (state.length == TERMINAL_LENGTH_SHORT)
          {
            state.length = TERMINAL_LENGTH_CHAR;
          }
          else
          {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        // set long/long long
        case 'l':
          if (state.length == TERMINAL_LENGTH_NORMAL)
          {
            state.length = TERMINAL_LENGTH_LONG;
          }
          else if (state.length == TERMINAL_LENGTH_LONG)
          {
            state.length = TERMINAL_LENGTH_LONG_LONG;
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
          FORMAT_NUM(terminal_writeint64, signed, 10);
          CLEAR_STATE();
          break;

        // unsigned octal
        case 'o':
          if (state.alt_form)
          {
            terminal_writechar_internal('0');
          }
          FORMAT_NUM(terminal_writeuint64, unsigned, 8);
          CLEAR_STATE();
          break;

        // unsigned decimal
        case 'u':
          FORMAT_NUM(terminal_writeuint64, unsigned, 10);
          CLEAR_STATE();
          break;

        // unsigned hexadecimal
        case 'x':
          if (state.alt_form)
          {
            terminal_writechar_internal('0');
            terminal_writechar_internal('x');
          }
          FORMAT_NUM(terminal_writeuint64, unsigned, 16);
          CLEAR_STATE();
          break;

        // char
        case 'c':
          terminal_writechar_internal(va_arg(args, int));
          CLEAR_STATE();
          break;

        // string
        case 's':
          terminal_writestring(va_arg(args, char *));
          CLEAR_STATE();
          break;

        // pointer
        case 'p':
          terminal_writechar_internal('0');
          terminal_writechar_internal('x');
          terminal_writeuint64((uint64_t) va_arg(args, void *), 16);
          CLEAR_STATE();
          break;

        default:
          INVALID_FORMAT();
          CLEAR_STATE();
      }
    }
  }

end:
  terminal_updatecursor();
  va_end(args);
}
