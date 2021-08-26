/*******************************************************************************
 *
 * kit/kernel/include/config.h
 * - compiler/target configuration abstraction
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef CONFIG_H
#define CONFIG_H

#define KERNEL_OFFSET 0xffffffff80000000

#if defined(__GNUC__) | defined(__clang__)

#define NORETURN __attribute__((__noreturn__))

#define UNUSED __attribute__((__unused__))

#define PACKED __attribute__((__packed__))

#define FORMAT_PRINTF(string_index, first_to_check) \
  __attribute__((__format__ (__printf__, string_index, first_to_check)))


// The following three are stolen from Linux. <3

#define DEFINE(sym, val) \
        __asm__ volatile("\n->" #sym " %0 " #val : : "i" (val))

#define BLANK() __asm__ volatile("\n->" : : )

#define OFFSET(sym, str, mem) \
        DEFINE(sym, offsetof(struct str, mem));

#endif

#endif
