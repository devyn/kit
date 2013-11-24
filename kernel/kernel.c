// Based on OSDev Bare Bones tutorial
// http://wiki.osdev.org/Bare_Bones

#include <stdint.h>

#include "multiboot.h"
#include "terminal.h"

/**
 * These aren't actually meant to be of type int; they're just here so that
 * we can get the address of them.
 */
extern int _kernel_begin;
extern int _kernel_end;

/**
 * Our bootstrap program copies the multiboot info here.
 */
extern struct multiboot_info kernel_multiboot_info;

#if defined(__cplusplus)
extern "C" /* Use C linkage for kernel_main. */
#endif
void kernel_main()
{
  terminal_initialize();
  terminal_setcolor(COLOR_WHITE, COLOR_RED);
  terminal_clear();

  terminal_setcolor(COLOR_RED, COLOR_WHITE);
  terminal_writestring("Kit Version 0.1\n");

  terminal_setcolor(COLOR_WHITE, COLOR_RED);
  terminal_writestring("\n* says Hello from long mode! *\n\n");

  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \x20\xdb\xdb\xdb\x20\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\x20 \xdb\x20\x20\x20\xdb\n");
  terminal_writestring("\xdb\x20\x20\x20\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \xdb\xdb\xdb\xdb\xdb \x20\xdb\xdb\xdb\x20\n");

  terminal_putchar('\n');

  if (kernel_multiboot_info.flags & MULTIBOOT_INFO_MEMORY)
  {
    terminal_writestring("Available memory: ");
    terminal_writeuint64(kernel_multiboot_info.mem_lower + kernel_multiboot_info.mem_upper, 10);
    terminal_writestring(" KB\n");
  }
  else
  {
    terminal_writestring("\nE: Bootloader did not provide valid memory information!\n");
  }

  if (kernel_multiboot_info.flags & MULTIBOOT_INFO_CMDLINE)
  {
    terminal_writestring("Kernel command line: ");
    terminal_writestring((char *) ((uint64_t) kernel_multiboot_info.cmdline));
    terminal_putchar('\n');
  }
  else
  {
    terminal_writestring("E: Bootloader did not provide kernel command line!\n");
  }

  terminal_writestring("Kernel starts at: 0x");
  terminal_writeuint64((uint64_t) &_kernel_begin, 16);
  terminal_putchar('\n');

  terminal_writestring("Kernel ends at:   0x");
  terminal_writeuint64((uint64_t) &_kernel_end, 16);
  terminal_putchar('\n');
}
