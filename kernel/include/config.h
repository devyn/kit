/*******************************************************************************
 *
 * kit/kernel/include/config.h
 * - compiler/target configuration abstraction
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef CONFIG_H
#define CONFIG_H

#if defined(__GNUC__) | defined(__clang__)
#define PACKED __attribute__((__packed__))
#endif

#endif
