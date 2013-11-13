// Based on OSDev Bare Bones tutorial
// http://wiki.osdev.org/Bare_Bones

#include <stdint.h>
 
#include "terminal.h"

/* Check if the compiler thinks if we are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif

#if defined(__cplusplus)
extern "C" /* Use C linkage for kernel_main. */
#endif
void kernel_main()
{
	terminal_initialize();
  terminal_setcolor(make_color(COLOR_WHITE, COLOR_RED));
  terminal_clear();

  terminal_setcolor(make_color(COLOR_RED, COLOR_WHITE));
	terminal_writestring("Kit Version 0.1\n");

  terminal_setcolor(make_color(COLOR_WHITE, COLOR_RED));
  terminal_writestring("\n* says Hello! *\n\n");

  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \x20\xdb\xdb\xdb\x20\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \x20\xdb\xdb\xdb\x20\n");

  terminal_writestring("\nscrolling...\n");

  terminal_setcolor(make_color(COLOR_BLACK, COLOR_GREEN));

  for (uint32_t i = 0; i < 20; i++)
  {
    terminal_writeuint32(i, 16);
    terminal_newline();
  }
}
