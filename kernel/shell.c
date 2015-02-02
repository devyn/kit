/*******************************************************************************
 *
 * kit/kernel/shell.c
 * - kernel hacking command interface
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#include "shell.h"
#include "string.h"
#include "terminal.h"
#include "keyboard.h"
#include "memory.h"
#include "paging.h"
#include "interrupt.h"
#include "ps2_8042.h"
#include "archive.h"
#include "debug.h"
#include "test.h"
#include "config.h"

static void shell_display_prompt(uint64_t lineno)
{
  terminal_writechar('\n');
  terminal_setcolor(COLOR_BLACK, COLOR_GREEN);
  terminal_printf("[%lu]", lineno);
  terminal_setcolor(COLOR_WHITE, COLOR_BLACK);
  terminal_writechar(' ');
}

static void shell_read_line(char *buffer, size_t size)
{
  size_t index = 0;

  // XXX: this shouldn't be necessary, but this stops working randomly
  // sometimes without it and I have absolutely no idea why
  keyboard_event_t event1, event2;
  keyboard_event_t *event = &event2;

  while (index < size - 1)
  {
    // XXX
    if (event == &event2) event = &event1;
    else event = &event2;

    keyboard_wait_dequeue(event);

    if (event->pressed && event->keychar != 0)
    {
      if (event->keychar == '\b')
      {
        // Handle backspace only if there are characters to erase.
        if (index > 0)
        {
          terminal_writechar('\b');
          index--;
        }
      }
      else
      {
        terminal_writechar(event->keychar);
        buffer[index++] = event->keychar;

        if (event->keychar == '\n') break;
      }
    }
  }

  buffer[index] = '\0';
}

static int shell_command_clear(UNUSED int argc, UNUSED char **argv)
{
  terminal_clear();
  return 0;
}

static int shell_command_echo(int argc, char **argv)
{
  for (int i = 1; i < argc; i++)
  {
    if (i > 1) terminal_writechar(' ');
    terminal_writestring(argv[i]);
  }

  terminal_writechar('\n');

  return 0;
}

static int shell_command_ver()
{
  terminal_writechar('\n');

  terminal_setcolor(COLOR_RED, COLOR_WHITE);

  for (int i = 0; i < 80; i++) terminal_writechar('+');

  terminal_setcolor(COLOR_WHITE, COLOR_RED);

  terminal_writestring(
      "                                                         \n"
      "              K   K    IIII   TTTTTTTT                   \n"
      "              K  K      II       TT                      \n"
      "              K K       II       TT                      \n"
      "              K  K      II       TT          ~devyn      \n"
      "              K   K    IIII      TT          version 0.1 \n"
      "                                                         \n");

  terminal_setcolor(COLOR_RED, COLOR_WHITE);

  for (int i = 0; i < 80; i++) terminal_writechar('+');

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);

  return 0;
}

static int shell_command_reboot(UNUSED int argc, UNUSED char **argv)
{
  ps2_8042_cpu_reset(); // this should not return

  terminal_setcolor(COLOR_RED, COLOR_BLACK);
  terminal_writestring("E: ps2_8042_cpu_reset() failed\n");

  return 1;
}

static int shell_command_mem(UNUSED int argc, UNUSED char **argv)
{
  uint64_t pages = memory_get_total_free();

  paging_pageset_t *pageset = paging_get_current_pageset();

  terminal_printf(
      " free:      %lu pages (%lu MiB)\n"
      " pageset:   %p\n"
      " PML4:      %p (phy %#lx)\n"
      " table_map: %lu entries (root %p)\n",

      pages, pages / 256,
      (void *) pageset,
      (void *) pageset->pml4, pageset->pml4_physical,
      pageset->table_map.entries, (void *) pageset->table_map.tree.root);

  return 0;
}

static int shell_command_test(int argc, char **argv)
{
  if (argc < 2)
  {
    terminal_writestring(
        " Usage: test <unit-name>\n"
        "        test all\n"
        "\n"
        " Units available for testing:\n"
        "\n   ");

    for (size_t i = 0; i < TEST_UNITS_SIZE; i++)
    {
      if (i != 0)
      {
        if (i % 5 == 0)
          terminal_writestring("\n   ");
        else
          terminal_writestring(", ");
      }
      terminal_writestring(test_units[i].name);
    }
    terminal_writechar('\n');

    return 2;
  }
  else
  {
    if (string_compare("all", argv[1]) == 0)
    {
      interrupt_disable();

      bool success = test_all();

      interrupt_enable();

      return success ? 0 : 1;
    }
    else
    {
      // Search for the unit name given in argv[1].
      const test_unit_t *unit = NULL;

      for (size_t i = 0; i < TEST_UNITS_SIZE; i++)
      {
        if (string_compare(argv[1], test_units[i].name) == 0)
        {
          unit = &test_units[i];
          break;
        }
      }

      if (unit != NULL)
      {
        interrupt_disable();

        bool success = test_run(unit);

        interrupt_enable();

        return success ? 0 : 1;
      }
      else
      {
        terminal_setcolor(COLOR_RED, COLOR_BLACK);
        terminal_printf("E: unit not found: %s\n", argv[1]);
        return 2;
      }
    }
  }
}

static int shell_command_ls(UNUSED int argc, UNUSED char **argv)
{
  archive_iterator_t iterator = archive_iterate(archive_system);

  archive_entry_t *entry;

  while ((entry = archive_next(&iterator)) != NULL)
  {
    terminal_writechar(' ');

    for (uint64_t i = 0; i < entry->name_length; i++)
    {
      terminal_writechar((&entry->name)[i]);
    }
    terminal_writechar('\n');
  }

  return 0;
}

static int shell_command_cat(int argc, char **argv)
{
  for (int i = 1; i < argc; i++)
  {
    char     *buffer;
    uint64_t  length;

    if (archive_get(archive_system, argv[i], &buffer, &length))
    {
      for (uint64_t i = 0; i < length; i++)
      {
        terminal_writechar(buffer[i]);
      }
    }
    else
    {
      terminal_setcolor(COLOR_RED, COLOR_BLACK);
      terminal_printf("E: file not found: %s\n", argv[i]);
      return 1;
    }
  }

  return 0;
}

typedef struct shell_command
{
  const char *name;
  int (*main)(int argc, char **argv);
} shell_command_t;

const shell_command_t shell_commands[] = {
  {"clear",  &shell_command_clear},
  {"echo",   &shell_command_echo},
  {"ver",    &shell_command_ver},
  {"reboot", &shell_command_reboot},
  {"mem",    &shell_command_mem},
  {"test",   &shell_command_test},
  {"ls",     &shell_command_ls},
  {"cat",    &shell_command_cat}
};

static void shell_execute(const char *command)
{
  size_t length = string_length(command);

  // Do nothing if length == 1 (empty).
  if (length == 1) return;

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);

  // Prepare argc and argv_strings.
  int  argc = 0;
  char argv_strings[length - 1];

  size_t i;

  for (i = 0; i < length - 1; i++)
  {
    if (command[i] == ' ')
    {
      argv_strings[i] = '\0'; // end of argument
      argc++;
    }
    else
    {
      argv_strings[i] = command[i];
    }
  }

  argv_strings[i] = '\0';
  argc++;

  // And now rescan to get argv.
  char *argv[argc];

  int arg_index = 1;
  argv[0] = argv_strings;

  for (size_t j = 0; j < length - 1 && arg_index < argc; j++)
  {
    if (argv_strings[j] == '\0')
    {
      argv[arg_index++] = argv_strings + j + 1;
    }
  }

  // Search for a command matching (&argv)[0].
  for (i = 0; i < sizeof(shell_commands) / sizeof(shell_command_t); i++)
  {
    if (string_compare(shell_commands[i].name, argv[0]) == 0)
    {
      // We found it.
      shell_commands[i].main(argc, argv);
      return;
    }
  }

  // We didn't find it.
  terminal_setcolor(COLOR_RED, COLOR_BLACK);
  terminal_printf("E: command not found: %s\n", argv[0]);
}

void shell()
{
  uint64_t lineno = 1;

  char *line = memory_alloc(sizeof(char) * 4096);

  while (true)
  {
    shell_display_prompt(lineno++);
    shell_read_line(line, 4096);
    shell_execute(line);
  }
}
