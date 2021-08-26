/*******************************************************************************
 *
 * kit/system/libc/include/stdio.h
 * - <stdio.h>: 'standard buffered input/output'
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * This file should be compatible with ANSI C [C89].
 *
 ******************************************************************************/

#ifndef _STDIO_H
#define _STDIO_H

#define EOF (-1)

typedef struct {
  int fd;
} FILE;

#ifndef _STDIO_C
extern FILE _libc_stdin;
extern FILE _libc_stdout;
extern FILE _libc_stderr;
#else
FILE _libc_stdin  = {0};
FILE _libc_stdout = {1};
FILE _libc_stderr = {2};
#endif

#define stdin  (&_libc_stdin)
#define stdout (&_libc_stdout)
#define stderr (&_libc_stderr)

int fputc(int ch, FILE *stream);
int putchar(int ch);

int fputs(const char *str, FILE *stream);
int puts(const char *str);

int fgetc(FILE *stream);
int getchar();

char *fgets(char *s, int size, FILE *stream);

#define getc(stream) fgetc(stream)
#define putc(str, stream) fputc(str, stream)

int feof(FILE *stream);

__attribute__((__format__ (__printf__, 1, 2)))
int printf(const char *restrict format, ...);

#endif
