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

int parser_prepare(size_t length, char *command_buf);

void parser_make_argv(size_t length, char *command_buf,
    int argc, char **argv);

#endif
