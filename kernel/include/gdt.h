/*******************************************************************************
 *
 * kit/kernel/include/gdt.h
 * - x86_64 GDT-related constants 
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef GDT_H
#define GDT_H

typedef enum gdt_selector
{
  GDT_SEL_KERNEL_CODE = 0x08,
  GDT_SEL_KERNEL_DATA = 0x10
} gdt_selector_t;

typedef enum gdt_privilege
{
  GDT_PRIVILEGE_KERNEL = 0,
  GDT_PRIVILEGE_USER   = 3
} gdt_privilege_t;

#endif
