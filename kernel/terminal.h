#ifndef TERMINAL_H
#define TERMINAL_H

#if !defined(__cplusplus)
#include <stdbool.h> /* C doesn't have booleans by default. */
#endif
#include <stddef.h>
#include <stdint.h>

/* Hardware text mode color constants. */
enum vga_color
{
  COLOR_BLACK = 0,
  COLOR_BLUE = 1,
  COLOR_GREEN = 2,
  COLOR_CYAN = 3,
  COLOR_RED = 4,
  COLOR_MAGENTA = 5,
  COLOR_BROWN = 6,
  COLOR_LIGHT_GREY = 7,
  COLOR_DARK_GREY = 8,
  COLOR_LIGHT_BLUE = 9,
  COLOR_LIGHT_GREEN = 10,
  COLOR_LIGHT_CYAN = 11,
  COLOR_LIGHT_RED = 12,
  COLOR_LIGHT_MAGENTA = 13,
  COLOR_LIGHT_BROWN = 14,
  COLOR_WHITE = 15,
};

#define VGA_WIDTH  80
#define VGA_HEIGHT 25

uint8_t terminal_make_color(enum vga_color fg, enum vga_color bg);

void terminal_initialize();
void terminal_clear();

void terminal_getcursor(size_t *row, size_t *column);
void terminal_setcursor(size_t  row, size_t  column);

void terminal_getcolor(enum vga_color *fg, enum vga_color *bg);
void terminal_setcolor(enum vga_color  fg, enum vga_color  bg);

void terminal_putentryat(char c, uint8_t color, size_t x, size_t y);

void terminal_newline();
void terminal_putchar(char c);

void terminal_writestring(const char *data);
int  terminal_writeuint32(uint32_t integer, uint8_t base);

#endif
