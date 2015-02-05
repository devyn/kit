/*******************************************************************************
 *
 * kit/kernel/include/syscall.h
 * - system call interface
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef SYSCALL_H
#define SYSCALL_H

#define SYSCALL_EXIT 0x0

void syscall_initialize();

#endif
