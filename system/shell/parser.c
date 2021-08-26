/*******************************************************************************
 *
 * kit/system/shell/parser.c
 * - kit shell language parser
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#include "parser.h"

char *parse_command(const char *line, command_t *command)
{
  size_t index = 0;
  size_t arg_start;
  bool   continue_after_consume = true;

  ptr_vec_init(&command->args);

st_find_arg_start:

  switch (line[index])
  {
    case ' ':
    case '\n':
      index++;
      goto st_find_arg_start;

    case ';':
    case '&':
    case '\0':
      goto st_command_end;

    default:
      goto st_consume_bare_arg;
  }

st_consume_bare_arg:

  arg_start = index;

  while (true)
  {
    switch (line[index])
    {
      case ';':
      case '&':
      case '\0':
        continue_after_consume = false;

      case ' ':
      case '\n':
        goto st_finish_consume;

      default:
        index++;
    }
  }

st_finish_consume:

  {
    char *arg = malloc(index - arg_start + 1);

    if (arg == NULL) exit(8); // Out of memory

    memcpy(arg, &line[arg_start], index - arg_start);
    arg[index - arg_start] = '\0';

    ptr_vec_push(&command->args, (void *) arg);

    if (continue_after_consume)
    {
      index++;
      goto st_find_arg_start;
    }
    else
    {
      goto st_command_end;
    }
  }

st_command_end:

  switch (line[index])
  {
    case '&':
      command->foreground = false;
      break;

    default:
      command->foreground = true;
  }

  if (line[index] == '\0')
  {
    command->end_of_stream = true;
  }
  else
  {
    index++;
    command->end_of_stream = false;
  }

  if (command->args.len > 0)
  {
    size_t length = strlen((char *) command->args.ptr[0]) + 5;

    command->filename = malloc(length);

    if (command->filename == NULL) exit(8); // Out of memory

    memcpy(command->filename, "bin/", 5);
    strcat(command->filename, (char *) command->args.ptr[0]);
  }
  else
  {
    command->filename = NULL;
  }

  return (char *) &line[index];
}

void parse_command_cleanup(command_t *command)
{
  if (command->filename != NULL)
  {
    free(command->filename);
    command->filename = NULL;
  }

  for (size_t i = 0; i < command->args.len; i++)
  {
    free(command->args.ptr[i]);
  }

  ptr_vec_free(&command->args);
}
