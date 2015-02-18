/*******************************************************************************
 *
 * kit/system/shell/parser.c
 * - kit shell language parser
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "io.h"
#include "parser.h"

char *parse_command(const char *line, command_t *command)
{
  bool        ignore_spaces = true;
  const char *command_end;

  command->argc = *line == '\0' ? 0 : 1;

  for (const char *c = line; ; c++)
  {
    switch (*c)
    {
      case ' ':
        if (!ignore_spaces)
        {
          command->argc++;
          ignore_spaces = true;
        }
        break;

      case ';':
      case '&':
      case '\n':
        command_end = c;
        command->end_of_stream = false;
        goto make_argv;

      case '\0':
        command_end = c;
        command->end_of_stream = true;
        goto make_argv;

      default:
        ignore_spaces = false;
    }
  }

make_argv:

  if (command->argc > 0)
  {
    size_t length = (uintptr_t) command_end - (uintptr_t) line + 1;

    command->argv = malloc(command->argc * sizeof(char *) + length);

    if (command->argv == NULL)
    {
      exit(8);
    }

    char *command_strings = (char *) (command->argv + command->argc);

    memcpy(command_strings, line, length - 1);
    command_strings[length - 1] = '\0';

    char *pos = command_strings;

    while (*pos == ' ') pos++;

    command->argv[0] = pos;

    for (int i = 1; i < command->argc; i++)
    {
      while (*pos == ' ') pos++;
      while (*pos != ' ') pos++;

      *pos = '\0';

      command->argv[i] = ++pos;
    }

    command->filename = malloc(strlen(command->argv[0]) + 5);

    if (command->filename == NULL)
    {
      exit(8);
    }

    memcpy(command->filename, "bin/", 5);
    strcat(command->filename, command->argv[0]);
  }
  else
  {
    command->filename = NULL;
    command->argv = NULL;
  }

  return (char *) command_end + 1;
}

void parse_command_cleanup(command_t *command)
{
  if (command->filename != NULL)
  {
    free(command->filename);
  }

  if (command->argv != NULL)
  {
    free(command->argv);
  }
}
