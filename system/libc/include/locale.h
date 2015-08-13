/*******************************************************************************
 *
 * kit/system/libc/include/locale.h
 * - <locale.h>: 'localization functions'
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * This file should be compatible with ANSI C [C89].
 *
 ******************************************************************************/

#ifndef _LOCALE_H
#define _LOCALE_H

#include <stddef.h>

struct lconv
{
  char *currency_symbol;
  char *decimal_point;
  char  frac_digits;
  char *grouping;
  char *mon_decimal_point;
  char *mon_grouping;
  char *mon_thousands_sep;
  char *negative_sign;
  char  n_cs_precedes;
  char  n_sep_by_space;
  char  n_sign_posn;
  char *positive_sign;
  char  p_cs_precedes;
  char  p_sep_by_space;
  char  p_sign_posn;
  char *thousands_sep;
};

/**
 * Categories to be used with setlocale().
 */
#define LC_ALL      1
#define LC_COLLATE  2
#define LC_CTYPE    3
#define LC_MONETARY 4
#define LC_NUMERIC  5
#define LC_TIME     6

/**
 * Loads the named locale into the global lconv structure for the given
 * category.
 */
char *setlocale(int category, const char *locale);

/**
 * Gets the current global lconv structure.
 */
struct lconv *localeconv(void);

#endif
