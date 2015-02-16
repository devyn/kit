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
#include <kit/syscall.h>

#include "io.h"
#include "parser.h"

static int last_exit_code = 0;

static void display_prompt(uint64_t lineno)
{
  tputc('\n');

  if (last_exit_code == 0)
  {
    tputs("\033[32;1m");
  }
  else
  {
    tputs("\033[31;1m");
  }

  tprintf("user %lu>>\033[0;1m ", lineno);
}

static void execute(char *line)
{
  size_t length = strlen(line);

  int argc = parser_prepare(length, line);

  char *argv[argc];

  parser_make_argv(length, line, argc, argv);

  // XXX FIXME XXX FIXME XXX WTFBBQ
  char filename[256];
  filename[0]='b';
  filename[1]='i';
  filename[2]='n';
  filename[3]='/';
  filename[4]='\0';
  strcat(filename, argv[0]);

  const char *const *pargv = (const char *const *) argv;

  int pid = syscall_spawn(filename, argc, pargv);

  if (pid <= 0)
  {
    last_exit_code = -100 + pid;

    tprintf("\033[31m E: spawn() failed; => %d\033[0m\n", pid);
    return;
  }

  if (syscall_wait_process(pid, &last_exit_code) < 0)
  {
    last_exit_code = -99;
    tputs("\033[31m E: wait_process() failed\033[0m\n");
    return;
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
