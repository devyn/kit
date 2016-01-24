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
#include <stdio.h>
#include <string.h>
#include <kit/syscall.h>

#include "parser.h"

static int last_exit_code = 0;

static void display_prompt(uint64_t lineno)
{
  putchar('\n');

  if (last_exit_code == 0)
  {
    fputs("\033[32;1m", stdout);
  }
  else
  {
    fputs("\033[31;1m", stdout);
  }

  printf("user %lu>>\033[0;1m ", lineno);
}

static void execute(char *line, uint64_t lineno)
{
  char *current_line = line;

  command_t command;

  do {
    current_line = parse_command(current_line, &command);

    if (command.filename != NULL)
    {
      const char *const *argv = (const char *const *) command.args.ptr;

      int pid = syscall_spawn(command.filename, command.args.len, argv);

      if (pid <= 0)
      {
        last_exit_code = -100 + pid;

        printf("\033[31m E: spawn('%s', %lu, argv) failed; => %d\033[0m\n",
            command.filename, command.args.len, pid);
      }
      else if (command.foreground &&
               syscall_wait_process(pid, &last_exit_code) < 0)
      {
        last_exit_code = -99;
        puts("\033[31m E: wait_process() failed\033[0m");
      }

      if (!command.foreground)
      {
        printf("[%lu] %d          ", lineno, pid);

        for (size_t i = 0; i < command.args.len; i++)
        {
          printf(" %s", command.args.ptr[i]);
        }

        putchar('\n');
      }
    }

    parse_command_cleanup(&command);

  } while (!command.end_of_stream);
}

#define UNUSED __attribute__((__unused__))

char line[4096];

int main(UNUSED int argc, UNUSED char **argv)
{
  uint64_t lineno = 1;

  while (true)
  {
    display_prompt(lineno);
    fgets(line, 4096, stdin);
    fputs("\033[0m", stdout);
    execute(line, lineno++);
  }
}
