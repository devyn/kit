/*******************************************************************************
 *
 * kit/kernel/terminal.c
 * - early text mode 80x25 terminal handler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
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

uint16_t terminal_make_vgaentry(char c, uint8_t color)
{
  uint16_t c16 = c;
  uint16_t color16 = color;
  return c16 | color16 << 8;
}

size_t terminal_row;
size_t terminal_column;
uint8_t terminal_color;
uint16_t* terminal_buffer;

void terminal_initialize()
{
  terminal_color = terminal_make_color(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_buffer = (uint16_t*) (KERNEL_OFFSET + 0xb8000);

  terminal_clear();
}

void terminal_clear()
{
  terminal_row = 0;
  terminal_column = 0;

  for ( size_t y = 0; y < VGA_HEIGHT; y++ )
  {
    for ( size_t x = 0; x < VGA_WIDTH; x++ )
    {
      terminal_putentryat(' ', terminal_color, x, y);
    }
  }
}

void terminal_scroll()
{
  // Shift everything one line back.
  for ( size_t y = 1; y < VGA_HEIGHT; y++ )
  {
    for ( size_t x = 0; x < VGA_WIDTH; x++ )
    {
      const size_t index = y * VGA_WIDTH + x;
      terminal_buffer[index - VGA_WIDTH] = terminal_buffer[index];
    }
  }

  // Clear last line.
  for ( size_t x = 0; x < VGA_WIDTH; x++ )
  {
    terminal_putentryat(' ', terminal_color, x, VGA_HEIGHT - 1);
  }
}

void terminal_updatecursor()
{
  uint16_t position = (terminal_row * VGA_WIDTH) + terminal_column;

  outb(0x0F,                   0x3D4);
  outb(position & 0xFF,        0x3D5);

  outb(0x0E,                   0x3D4);
  outb((position >> 8) & 0xFF, 0x3D5);
}

void terminal_getcursor(size_t *row, size_t *column)
{
  *row    = terminal_row;
  *column = terminal_column;
}

void terminal_setcursor(size_t row, size_t column)
{
  terminal_row    = row;
  terminal_column = column;

  terminal_updatecursor();
}

void terminal_getcolor(enum vga_color *fg, enum vga_color *bg)
{
  *fg =  terminal_color       & 0xff;
  *bg = (terminal_color >> 4) & 0xff;
}

void terminal_setcolor(enum vga_color fg, enum vga_color bg)
{
  terminal_color = terminal_make_color(fg, bg);
}

void terminal_putentryat(char c, uint8_t color, size_t x, size_t y)
{
  const size_t index = y * VGA_WIDTH + x;
  terminal_buffer[index] = terminal_make_vgaentry(c, color);
}

void terminal_newline()
{
  // Clear to end of line.
  while (terminal_column < VGA_WIDTH)
  {
    terminal_putentryat(' ', terminal_color, terminal_column, terminal_row);
    terminal_column++;
  }

  // Go to next line, scrolling if necessary.
  terminal_column = 0;
  if ( ++terminal_row == VGA_HEIGHT )
  {
    terminal_scroll();
    terminal_row--;
  }

  terminal_updatecursor();

#ifdef THROTTLE
  for (int i = 0; i < 40000000; i++) __asm__ volatile("mfence");
#endif
}

static bool    terminal_ansiattrib_read = false;
static uint8_t terminal_ansiattrib_number;

static enum vga_color terminal_ansiattrib_color[] = {
  [0] = COLOR_BLACK,
  [1] = COLOR_RED,
  [2] = COLOR_GREEN,
  [3] = COLOR_BROWN,
  [4] = COLOR_BLUE,
  [5] = COLOR_MAGENTA,
  [6] = COLOR_CYAN,
  [7] = COLOR_LIGHT_GREY
};

void terminal_writechar_internal(char c)
{
  if (!terminal_ansiattrib_read)
  {
    switch (c) {
      case '\n': // newline
        terminal_newline();
        break;

      case '\b': // backspace
        if ( terminal_column > 0 ) terminal_column--;

        terminal_putentryat(' ', terminal_color, terminal_column, terminal_row);
        terminal_updatecursor();
        break;

      case '\033': // escape
        terminal_ansiattrib_read = true;
        terminal_ansiattrib_number = 0;
        break;

      default:
        terminal_putentryat(c, terminal_color, terminal_column, terminal_row);
        if ( ++terminal_column == VGA_WIDTH )
        {
          terminal_newline();
        }
    }
  }
  else
  {
    // XXX: the following is a total hack
    if (c >= '0' && c <= '9')
    {
      terminal_ansiattrib_number *= 10;
      terminal_ansiattrib_number += c - '0';
    }
    else if (c == ';' || c == 'm')
    {
      enum vga_color fg, bg;

      terminal_getcolor(&fg, &bg);

      if (terminal_ansiattrib_number == 0)
      {
        fg = COLOR_LIGHT_GREY;
        bg = COLOR_BLACK;
      }
      else if (terminal_ansiattrib_number == 1)
      {
        if (fg < COLOR_DARK_GREY) fg += 8; // bright offset
      }
      else if (terminal_ansiattrib_number >= 30 &&
               terminal_ansiattrib_number <= 37)
      {
        fg = terminal_ansiattrib_color[terminal_ansiattrib_number - 30];
      }
      else if (terminal_ansiattrib_number >= 40 &&
               terminal_ansiattrib_number <= 47)
      {
        bg = terminal_ansiattrib_color[terminal_ansiattrib_number - 40];
      }

      terminal_setcolor(fg, bg);

      if (c == ';')
      {
        terminal_ansiattrib_number = 0;
      }
      else
      {
        terminal_ansiattrib_read = false;
      }
    }
    else if (c != '[')
    {
      terminal_ansiattrib_read = false;
    }
  }
}

void terminal_writechar(char c)
{
  terminal_writechar_internal(c);
  terminal_updatecursor();
}

void terminal_writebuf(uint64_t length, const char *buffer)
{
  for (uint64_t i = 0; i < length; i++)
  {
    terminal_writechar_internal(buffer[i]);
  }

  terminal_updatecursor();
}

void terminal_writestring(const char *data)
{
  for (size_t i = 0; data[i] != '\0'; i++)
  {
    terminal_writechar_internal(data[i]);
  }

  terminal_updatecursor();
}

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
