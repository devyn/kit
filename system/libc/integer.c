/*******************************************************************************
 *
 * kit/system/libc/integer.c
 * - integer parsing, formatting, and math functions
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdlib.h>

long int strtol(const char *nptr, char **endptr, int base) {
  long int n = 0;
  long int sign = 1;

  // FIXME: check for underflow/overflow and set errno

  while (*nptr == ' ');

  if (*nptr == '-') {
    sign = -1;
    nptr++;
  }
  else if (*nptr == '+') {
    nptr++;
  }

  if ((base == 0 || base == 16) && nptr[0] == '0' && nptr[1] == 'x') {
    nptr += 2;
    base = 16;
  }
  else if ((base == 0 || base == 8) && nptr[0] == '0') {
    nptr++;
    base = 8;
  }
  else if (base == 0) {
    base = 10;
  }

  if (base < 2 || base > 36) {
    *endptr = (char *) nptr;
    return n;
  }

  while (*nptr != '\0') {
    n *= base;

    if (*nptr >= '0' && *nptr <= '9' && *nptr < ('0' + base)) {
      n += *nptr - '0';
    }
    else if (base > 10 && *nptr >= 'a' && *nptr < ('a' + (base - 10))) {
      n += *nptr - 'a' + 10;
    }
    else if (base > 10 && *nptr >= 'A' && *nptr < ('A' + (base - 10))) {
      n += *nptr - 'A' + 10;
    }
    else {
      break; // error!
    }

    nptr++;
  }

  *endptr = (char *) nptr;
  return n;
}
