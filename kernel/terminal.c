/*******************************************************************************
 *
 * kit/kernel/terminal.c
 * - early text mode 80x25 terminal handler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "terminal.h"
#include "x86_64.h"

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
  terminal_buffer = (uint16_t*) 0xB8000;

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
}

void terminal_putchar_internal(char c)
{
  switch (c) {
    case '\n':
      terminal_newline();
      break;
    default:
      terminal_putentryat(c, terminal_color, terminal_column, terminal_row);
      if ( ++terminal_column == VGA_WIDTH )
      {
        terminal_newline();
      }
  }
}

void terminal_putchar(char c)
{
  terminal_putchar_internal(c);
  terminal_updatecursor();
}

void terminal_writestring(const char *data)
{
  for (size_t i = 0; data[i] != '\0'; i++)
  {
    terminal_putchar_internal(data[i]);
  }

  terminal_updatecursor();
}

/* Can handle any base from binary up to sexatrigesimal (36), encompassing all alphanumeric characters */
int terminal_writeuint64(uint64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer == 0)
  {
    terminal_putchar('0');
    return 0;
  }

  char string[33];
  size_t position = 32;

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
