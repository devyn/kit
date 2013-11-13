// Based on OSDev Bare Bones tutorial
// http://wiki.osdev.org/Bare_Bones

#include "terminal.h"

/* Check if the compiler thinks if we are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif

uint8_t make_color(enum vga_color fg, enum vga_color bg)
{
  return fg | bg << 4;
}

uint16_t make_vgaentry(char c, uint8_t color)
{
  uint16_t c16 = c;
  uint16_t color16 = color;
  return c16 | color16 << 8;
}

size_t strlen(const char* str)
{
  size_t ret = 0;
  while ( str[ret] != 0 )
    ret++;
  return ret;
}

size_t terminal_row;
size_t terminal_column;
uint8_t terminal_color;
uint16_t* terminal_buffer;

void terminal_initialize()
{
  terminal_color = make_color(COLOR_LIGHT_GREY, COLOR_BLACK);
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
      const size_t index = y * VGA_WIDTH + x;
      terminal_buffer[index] = make_vgaentry(' ', terminal_color);
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

void terminal_setcolor(uint8_t color)
{
  terminal_color = color;
}

void terminal_putentryat(char c, uint8_t color, size_t x, size_t y)
{
  const size_t index = y * VGA_WIDTH + x;
  terminal_buffer[index] = make_vgaentry(c, color);
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
}

void terminal_putchar(char c)
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

void terminal_writestring(const char* data)
{
  size_t datalen = strlen(data);
  for ( size_t i = 0; i < datalen; i++ )
    terminal_putchar(data[i]);
}

/* Can handle any base from binary up to sexatrigesimal (36), encompassing all alphanumeric characters */
int terminal_writeuint32(uint32_t integer, uint32_t base)
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
    uint8_t digit = integer / (base / base) % base;

    if (digit < 10)
    {
      string[--position] = '0' + digit;
    }
    else
    {
      string[--position] = 'a' + (digit - 10);
    }

    integer = integer / base;
    base    = base * base;
  }

  terminal_writestring(string + position);

  return 0;
}
