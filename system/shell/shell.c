/*******************************************************************************
 *
 * kit/system/shell/shell.c
 * - the kit shell
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

#include "io.h"
#include "syscall.h"
#include "parser.h"

static int last_exit_code = 0;

static void display_prompt(uint64_t lineno)
{
  tputc('\n');

  if (last_exit_code == 0)
  {
    tputs("\033[30;42m");
  }
  else
  {
    tputs("\033[30;41m");
  }

  tprintf("[%lu]\033[0;1m ", lineno);
}

static void execute(char *line)
{
  size_t length = strlen(line);

  int argc = parser_prepare(length, line);

  char *argv[argc];

  parser_make_argv(length, line, argc, argv);

  for (int i = 0; i < argc; i++)
  {
    tprintf("%s\n", argv[i]);
  }
}

#define UNUSED __attribute__((__unused__))

char line[4096];

int main(UNUSED int argc, UNUSED char **argv)
{
  uint64_t lineno = 1;

  while (true)
  {
    display_prompt(lineno++);
    tgets(line, 4096);
    tputs("\033[0m");
    execute(line);
  }
}
