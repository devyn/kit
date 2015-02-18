/*******************************************************************************
 *
 * kit/system/shell/include/parser.h
 * - kit shell language parser
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef _KIT_SHELL_PARSER_H
#define _KIT_SHELL_PARSER_H

#include <stddef.h>
#include <stdbool.h>

typedef struct command
{
  char  *filename;
  int    argc;
  char **argv;
  bool   end_of_stream;
} command_t;

char *parse_command(const char *line, command_t *command);

void parse_command_cleanup(command_t *command);

#endif
