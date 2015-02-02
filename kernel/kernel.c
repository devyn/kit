/*******************************************************************************
 *
 * kit/kernel/kernel.c
 * - main kernel entry point and top level management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Based on OSDev Bare Bones tutorial
 * http://wiki.osdev.org/Bare_Bones
 *
 ******************************************************************************/

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#include "x86_64.h"

#include "config.h"
#include "multiboot.h"
#include "terminal.h"
#include "interrupt.h"
#include "ps2_8042.h"
#include "ps2key.h"
#include "keyboard.h"
#include "memory.h"
#include "paging.h"
#include "archive.h"
#include "debug.h"
#include "shell.h"

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

bool kernel_test_memory_c();

#if defined(__cplusplus)
extern "C" /* Use C linkage for kernel_main. */
#endif
NORETURN
void kernel_main()
{
  terminal_initialize();

  terminal_setcolor(COLOR_RED, COLOR_WHITE);
  terminal_writestring("+ Hello. I'm Kit.\n");

  terminal_setcolor(COLOR_WHITE, COLOR_RED);
  terminal_writechar('\n');

  struct multiboot_info *mb_info = (struct multiboot_info *)
    (KERNEL_OFFSET + (uint64_t) &kernel_multiboot_info);

  if (mb_info->flags & MULTIBOOT_INFO_MEMORY)
  {
    terminal_printf("Lower memory:        %u kB\n"
                    "Upper memory:        %u kB\n",
                    mb_info->mem_lower,
                    mb_info->mem_upper);
  }
  else
  {
    terminal_writestring(
      "W: Bootloader did not provide valid memory information!\n");
  }

  if (mb_info->flags & MULTIBOOT_INFO_CMDLINE)
  {
    terminal_printf("Kernel command line: %s\n",
        (char *) (KERNEL_OFFSET + mb_info->cmdline));
  }
  else
  {
    terminal_writestring(
      "W: Bootloader did not provide kernel command line!\n");
  }

  terminal_printf("Kernel starts at:    %p\n"
                  "Kernel ends at:      %p\n",
                  (void *) &_kernel_begin, (void *) &_kernel_end);

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_writechar('\n');

  if (mb_info->flags & MULTIBOOT_INFO_MEM_MAP) {
    char *mmap = (char *) (KERNEL_OFFSET + mb_info->mmap_addr);

    memory_initialize(mmap, mb_info->mmap_length);
  }
  else {
    terminal_writestring(
      "E: Bootloader did not provide memory map! Halting.\n");

    goto hang;
  }

  interrupt_initialize();
  paging_initialize();

  keyboard_initialize();
  ps2key_initialize();

  if (!ps2_8042_initialize()) goto hang;

  multiboot_module_t *modules = (multiboot_module_t *)
    (KERNEL_OFFSET + mb_info->mods_addr);

  if (!archive_initialize(mb_info->mods_count, modules))
  {
    goto hang;
  }

  interrupt_enable();

  shell();

  goto hang;

hang:
  while (true) hlt();
}
