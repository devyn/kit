/*******************************************************************************
 *
 * kit/kernel/syscall.c
 * - system call interface
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#define SYSCALL_C

#include <stddef.h>
#include <stdbool.h>

#include "syscall.h"
#include "config.h"
#include "process.h"
#include "scheduler.h"
#include "x86_64.h"
#include "debug.h"
#include "gdt.h"

#include "terminal.h"
#include "interrupt.h"

// IA32_EFER.SCE (SysCall Enable); bit #0
#define IA32_EFER_SCE 0x1

// Disable all non-reserved, relevant EFLAGS bits when entering the kernel
#define SYSCALL_FLAG_MASK 0x003f4fd5

// Not intended to be called.
extern void syscall_handler();

void syscall_initialize()
{
  // Enable system calls.
  wrmsr(rdmsr(IA32_EFER) | IA32_EFER_SCE, IA32_EFER);

  // Load the STAR with the target segments.
  //
  // User mode [63:48]:   + 0x0  for 32-bit code
  //                      + 0x8  for data
  //                      + 0x10 for 64-bit code
  //
  // Kernel mode [47:32]: + 0x0  for 64-bit code
  //                      + 0x8  for data
  uint64_t star = rdmsr(IA32_STAR);

  star |= ((uint64_t) GDT_SEL_USER_CODE_32 << 48);
  star |= ((uint64_t) GDT_SEL_KERNEL_CODE  << 32);

  wrmsr(star, IA32_STAR);

  // Load LSTAR with the syscall handler.
  wrmsr((uint64_t) &syscall_handler, IA32_LSTAR);

  // Load FMASK with the flag mask.
  wrmsr(SYSCALL_FLAG_MASK, IA32_FMASK);
}

int syscall_exit(int status)
{
  process_exit(status);

  DEBUG_MESSAGE("failed to exit process");
  while (true) hlt();
}

int syscall_twrite(uint64_t length, const char *buffer)
{
  terminal_writebuf(length, buffer);
  return 0;
}

int syscall_key_get(keyboard_event_t *event)
{
  keyboard_sleep_dequeue(event);
  return 0;
}

int syscall_yield()
{
  // Might return immediately if there's nothing else to do.
  scheduler_tick();
  return 0;
}

int syscall_sleep()
{
  scheduler_sleep();
  return 0;
}

int64_t syscall_spawn(const char *file, int argc, const char *const *argv)
{
  return archive_utils_spawn(file, argc, argv);
}

int syscall_wait_process(process_id_t id, int *exit_status)
{
  return process_wait_exit_status(id, exit_status);
}

void *syscall_adjust_heap(int64_t amount)
{
  return process_adjust_heap(amount);
}

archive_header_t *syscall_mmap_archive()
{
  // Find the extent of the archive
  archive_iterator_t iterator = archive_iterate(archive_system);

  archive_entry_t *entry;

  uint64_t size = 0;

  // This *should* always work. Archive headers are written from first to last.
  while ((entry = archive_next(&iterator)) != NULL) {
    size = entry->offset + entry->length;
  }

  // Now map those pages. This probably isn't the most efficient way to do it,
  // but oh well. We should be able to do a better job with the page iterators
  // implemented in Rust.
  uint64_t src_address = (uint64_t) archive_system & ((uint64_t) -1 << 12);
  uint64_t dst_address = 0x00000ace00000000; // ace = 'archive'
  uint64_t phy_address;

  uint64_t src_limit = src_address + size;

  paging_pageset_t current_pageset = paging_get_current_pageset();

  while (src_address < src_limit)
  {
    // Get the physical address of the archive page.
    bool ok = paging_resolve_linear_address(
        paging_kernel_pageset, (void *) src_address, &phy_address);

    if (!ok) {
      DEBUG_FORMAT("unresolveable archive page: %#lx", src_address);
      while (true) hlt();
    }

    // Map it with read-only access.
    paging_map(current_pageset, (void *) dst_address, phy_address, 1,
        PAGING_READONLY | PAGING_USER);

    // Advance one page.
    src_address += 0x1000;
    dst_address += 0x1000;
  }

  return (archive_header_t *) 0x00000ace00000000;
}

void syscall_print_processes() {
    process_print_processes();
}
