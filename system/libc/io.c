/*******************************************************************************
 *
 * kit/system/libc/io.c
 * - standard C I/O implementation
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdbool.h>
#include <stdarg.h>
#include <string.h>
#include <kit/syscall.h>

#define _STDIO_C
#include <stdio.h>

int putchar(int ch)
{
  char c = ch;

  syscall_twrite(1, &c);

  return ch;
}

int fputc(int ch, FILE *stream) {
  if (stream == stdout || stream == stderr) {
    return putchar(ch);
  }
  else {
    return EOF;
  }
}

void _libc_puts_nonl(const char *str)
{
  syscall_twrite(strlen(str), str);
}

int puts(const char *str) {
  _libc_puts_nonl(str);
  putchar('\n');
  return 1;
}

int fputs(const char *str, FILE *stream) {
  if (stream == stdout || stream == stderr) {
    _libc_puts_nonl(str);
    return 1;
  }
  else {
    return EOF;
  }
}

int _libc_uwidth(uint64_t integer, uint8_t base)
{
  int digits = 0;

  do {
    integer /= base;
    digits++;
  } while (integer > 0);
  return digits;
}

int _libc_iwidth(int64_t integer, uint8_t base)
{
  int digits = 0;
  int sign = integer >> 63;

  do {
    integer /= base;
    digits++;
  } while (integer != 0);
  return digits + sign;
}

/**
 * Can handle any base from binary up to sexatrigesimal (36), encompassing all
 * alphanumeric characters
 */
int _libc_putu64(uint64_t integer, uint8_t base)
{
  int len = 0;

  if (base < 2 || base > 36)
    return -1;

  if (integer == 0)
  {
    putchar('0');
    return 1;
  }

  char string[65];
  size_t position = 64;

  string[position] = '\0';

  while (integer > 0)
  {
    uint8_t digit = integer % base;

    if (digit < 10)
    {
      string[--position] = '0' + digit;
    }
    else
    {
      string[--position] = 'a' + (digit - 10);
    }

    len++;

    integer = integer / base;
  }

  _libc_puts_nonl(string + position);

  return len;
}

/**
 * Signed variant of _libc_putu64
 */
int _libc_puti64(int64_t integer, uint8_t base)
{
  if (base < 2 || base > 36)
    return -1;

  if (integer & ((uint64_t) 1 << 63))
  {
    // Negative
    putchar('-');
    return _libc_putu64(~integer + 1, base) + 1;
  }
  else
  {
    // Positive
    return _libc_putu64(integer, base);
  }
}

bool _libc_stdin_eof = false;

char *_libc_fgets_stdin(char *s, size_t size)
{
  size_t index = 0;

  keyboard_event_t event;

  if (_libc_stdin_eof) {
    return NULL;
  }

  while (index < size - 1)
  {
    syscall_key_get(&event);

    if (event.pressed && event.keychar != 0)
    {
      if (event.ctrl_down && event.keychar == 'd') {
        // C-d = EOF
        _libc_stdin_eof = true;

        if (index == 0) {
          return NULL;
        }
        else {
          break;
        }
      }
      else if (event.keychar == '\b')
      {
        // Handle backspace only if there are characters to erase.
        if (index > 0)
        {
          putchar('\b');
          index--;
        }
      }
      else
      {
        putchar(event.keychar);
        s[index++] = event.keychar;

        if (event.keychar == '\n') break;
      }
    }
  }

  s[index] = '\0';

  return s;
}

char *fgets(char *s, int size, FILE *stream) {
  if (stream == stdin) {
    return _libc_fgets_stdin(s, size);
  }
  else {
    return NULL;
  }
}

int _libc_fgetc_stdin() {
  keyboard_event_t event;

  if (_libc_stdin_eof) {
    return EOF;
  }

  while (true) {
    syscall_key_get(&event);

    if (event.pressed && event.keychar != 0) {
      if (event.ctrl_down && event.keychar == 'd') {
        // C-d = EOF
        _libc_stdin_eof = true;
        return EOF;
      }
      else {
        return event.keychar;
      }
    }
  }
}

int fgetc(FILE *stream) {
  if (stream == stdin) {
    return _libc_fgetc_stdin();
  }
  else {
    return EOF;
  }
}

int getchar() {
  return _libc_fgetc_stdin();
}

int feof(FILE *stream) {
  if (stream == stdin && _libc_stdin_eof) {
    return 1;
  }
  else {
    return 0;
  }
}

typedef struct _libc_printf_state
{
  bool active;
  size_t start;
  bool alt_form;

  int pad;
  bool pad_left;
  char pad_char;

  enum {
    _LIBC_PRINTF_LENGTH_CHAR,
    _LIBC_PRINTF_LENGTH_SHORT,
    _LIBC_PRINTF_LENGTH_NORMAL,
    _LIBC_PRINTF_LENGTH_LONG,
    _LIBC_PRINTF_LENGTH_LONG_LONG
  } length;
} _libc_printf_state_t;

/**
 * Incomplete printf implementation.
 */
__attribute__((__format__ (__printf__, 1, 2)))
int printf(const char *format, ...)
{
  va_list args;
  int len = 0;

  _libc_printf_state_t state;

#define CLEAR_STATE() \
  { \
    state.active   = false; \
    state.start    = 0; \
    state.alt_form = false; \
    state.pad      = 0; \
    state.pad_left = false; \
    state.pad_char = ' '; \
    state.length   = _LIBC_PRINTF_LENGTH_NORMAL; \
  }

#define LPAD(size) \
  if (!state.pad_left) { \
    for (int l = state.pad - size; l > 0; l--) { \
      putchar(state.pad_char); \
      len++; \
    } \
  }

#define RPAD(size) \
  if (state.pad_left) { \
    for (int l = state.pad - size; l > 0; l--) { \
      putchar(state.pad_char); \
      len++; \
    } \
  }

#define FORMAT_NUM(fn, lenfn, mod, base) \
  switch (state.length) { \
    case _LIBC_PRINTF_LENGTH_CHAR: \
      {mod char val = va_arg(args, mod int); \
      LPAD(lenfn(val, (base))); \
      len += fn(val, (base)); \
      RPAD(lenfn(val, (base)));} \
      break; \
    case _LIBC_PRINTF_LENGTH_SHORT: \
      {mod short val = va_arg(args, mod int); \
      LPAD(lenfn(val, (base))); \
      len += fn(val, (base)); \
      RPAD(lenfn(val, (base)));} \
      break; \
    case _LIBC_PRINTF_LENGTH_NORMAL: \
      {mod int val = va_arg(args, mod int); \
      LPAD(lenfn(val, (base))); \
      len += fn(val, (base)); \
      RPAD(lenfn(val, (base)));} \
      break; \
    case _LIBC_PRINTF_LENGTH_LONG: \
      {mod long val = va_arg(args, mod long); \
      LPAD(lenfn(val, (base))); \
      len += fn(val, (base)); \
      RPAD(lenfn(val, (base)));} \
      break; \
    case _LIBC_PRINTF_LENGTH_LONG_LONG: \
      {mod long long val = va_arg(args, mod long long); \
      LPAD(lenfn(val, (base))); \
      len += fn(val, (base)); \
      RPAD(lenfn(val, (base)));} \
      break; \
  }

#define INVALID_FORMAT() \
  for (size_t j = state.start; j <= i; j++) \
  { \
    putchar(format[j]); \
    len++; \
  }

  CLEAR_STATE();
  va_start(args, format);

  for (size_t i = 0;; i++)
  {
    if (!state.active) {
      switch (format[i])
      {
        case '\0':
          goto end;

        case '%':
          state.active = true;
          state.start  = i;
          break;

        default:
          putchar(format[i]);
          len++;
      }
    }
    else
    {
      switch (format[i])
      {
        case '\0':
          // flush
          _libc_puts_nonl(format + state.start);
          len += strlen(format + state.start);
          goto end;

        case '%':
          putchar('%');
          len++;
          CLEAR_STATE();
          break;

        case '-':
          if (state.pad == 0 && state.pad_char == ' ' && !state.pad_left) {
            state.pad_left = true;
          }
          else {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        case '0':
          if (state.pad == 0 && state.pad_char == ' ') {
            state.pad_char = '0';
            break;
          }
        case '1':
        case '2':
        case '3':
        case '4':
        case '5':
        case '6':
        case '7':
        case '8':
        case '9':
          state.pad *= 10;
          state.pad += format[i] - '0';
          break;

        // set short/char
        case 'h':
          if (state.length == _LIBC_PRINTF_LENGTH_NORMAL)
          {
            state.length = _LIBC_PRINTF_LENGTH_SHORT;
          }
          else if (state.length == _LIBC_PRINTF_LENGTH_SHORT)
          {
            state.length = _LIBC_PRINTF_LENGTH_CHAR;
          }
          else
          {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        // set long/long long
        case 'l':
          if (state.length == _LIBC_PRINTF_LENGTH_NORMAL)
          {
            state.length = _LIBC_PRINTF_LENGTH_LONG;
          }
          else if (state.length == _LIBC_PRINTF_LENGTH_LONG)
          {
            state.length = _LIBC_PRINTF_LENGTH_LONG_LONG;
          }
          else
          {
            INVALID_FORMAT();
            CLEAR_STATE();
          }
          break;

        // set alternate form
        case '#':
          state.alt_form = true;
          break;

        // signed decimal
        case 'd':
        case 'i':
          FORMAT_NUM(_libc_puti64, _libc_iwidth, signed, 10);
          CLEAR_STATE();
          break;

        // unsigned octal
        case 'o':
          if (state.alt_form)
          {
            putchar('0');
            len++;
          }
          FORMAT_NUM(_libc_putu64, _libc_uwidth, unsigned, 8);
          CLEAR_STATE();
          break;

        // unsigned decimal
        case 'u':
          FORMAT_NUM(_libc_putu64, _libc_uwidth, unsigned, 10);
          CLEAR_STATE();
          break;

        // unsigned hexadecimal
        case 'x':
          if (state.alt_form)
          {
            _libc_puts_nonl("0x");
            len += 2;
          }
          FORMAT_NUM(_libc_putu64, _libc_uwidth, unsigned, 16);
          CLEAR_STATE();
          break;

        // char
        case 'c':
          putchar(va_arg(args, int));
          CLEAR_STATE();
          break;

        // string
        case 's':
          _libc_puts_nonl(va_arg(args, char *));
          CLEAR_STATE();
          break;

        // pointer
        case 'p':
          _libc_puts_nonl("0x");
          len += 2 + _libc_putu64((uint64_t) va_arg(args, void *), 16);
          CLEAR_STATE();
          break;

        default:
          INVALID_FORMAT();
          CLEAR_STATE();
      }
    }
  }

end:
  va_end(args);

  return len;
}
