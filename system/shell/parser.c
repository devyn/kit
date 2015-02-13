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

#include "parser.h"

int parser_prepare(size_t length, char *command_buf)
{
  int argc = 0;

  for (size_t i = 0; i < length - 1; i++)
  {
    if (command_buf[i] == ' ')
    {
      command_buf[i] = '\0';
      argc++;
    }
  }

  command_buf[length - 1] = '\0';
  argc++;

  return argc;
}

void parser_make_argv(size_t length, char *command_buf,
    int argc, char **argv)
{
  int ai = 1;

  argv[0] = command_buf;

  for (size_t ci = 0; ci < length - 1 && ai < argc; ci++)
  {
    if (command_buf[ci] == '\0')
    {
      argv[ai++] = command_buf + ci + 1;
    }
  }
}
