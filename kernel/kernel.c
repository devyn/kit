// Based on OSDev Bare Bones tutorial
// http://wiki.osdev.org/Bare_Bones

#include <stdint.h>

#include "multiboot.h"
#include "terminal.h"

/* Check if the compiler thinks if we are targeting the wrong operating system. */
#if defined(__linux__)
#error "You are not using a cross-compiler, you will most certainly run into trouble"
#endif

extern uint32_t kernel_end;

#if defined(__cplusplus)
extern "C" /* Use C linkage for kernel_main. */
#endif
void kernel_main(struct multiboot_info *mboot)
{
  terminal_initialize();
  terminal_setcolor(COLOR_WHITE, COLOR_RED);
  terminal_clear();

  terminal_setcolor(COLOR_RED, COLOR_WHITE);
  terminal_writestring("Kit Version 0.1\n");

  terminal_setcolor(COLOR_WHITE, COLOR_RED);
  terminal_writestring("\n* says Hello! *\n\n");

  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \x20\xdb\xdb\xdb\x20\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \x20\xdb\xdb\xdb\x20\n");

  terminal_putchar('\n');

  if (mboot->flags & MULTIBOOT_INFO_MEMORY)
  {
    terminal_writestring("Available memory: ");
    terminal_writeuint32(mboot->mem_lower + mboot->mem_upper, 10);
    terminal_writestring(" KB\n");
  }
  else
  {
    terminal_writestring("\nE: Bootloader did not provide valid memory information!\n");
  }

  if (mboot->flags & MULTIBOOT_INFO_CMDLINE)
  {
    terminal_writestring("Kernel command line: ");
    terminal_writestring((char *) mboot->cmdline);
    terminal_putchar('\n');
  }
  else
  {
    terminal_writestring("E: Bootloader did not provide kernel command line!\n");
  }

  terminal_writestring("Kernel ends at: 0x");
  terminal_writeuint32(kernel_end, 16);
  terminal_putchar('\n');
}
