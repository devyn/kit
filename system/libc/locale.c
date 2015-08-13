/*******************************************************************************
 *
 * kit/system/libc/locale.c
 * - C localization support
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <locale.h>
#include <limits.h>

// Defaults correspond to C locale.
struct lconv global_lconv = {
  "",       // char *currency_symbol;
  ".",      // char *decimal_point;
  CHAR_MAX, // char  frac_digits;
  "",       // char *grouping;
  "",       // char *mon_decimal_point;
  "",       // char *mon_grouping;
  "",       // char *mon_thousands_sep;
  "",       // char *negative_sign;
  CHAR_MAX, // char  n_cs_precedes;
  CHAR_MAX, // char  n_sep_by_space;
  CHAR_MAX, // char  n_sign_posn;
  "",       // char *positive_sign;
  CHAR_MAX, // char  p_cs_precedes;
  CHAR_MAX, // char  p_sep_by_space;
  CHAR_MAX, // char  p_sign_posn;
  "",       // char *thousands_sep;
};

char *setlocale(__attribute__((unused)) int category,
                __attribute__((unused)) const char *locale)
{
  // XXX: Do nothing.
  return "C";
}

struct lconv *localeconv() {
  return &global_lconv;
}
