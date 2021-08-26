/*******************************************************************************
 *
 * kit/kernel/include/archive.h
 * - kit archive (init files) loader
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef ARCHIVE_H
#define ARCHIVE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#include "config.h"
#include "multiboot.h"

/* "kit AR01" */
#define ARCHIVE_MAGIC 0x313052412074696b

typedef struct PACKED archive_entry
{
  uint64_t  offset;
  uint64_t  length;
  uint64_t  checksum;
  uint64_t  name_length;
  char      name; // array
} archive_entry_t;

typedef struct PACKED archive_header
{
  uint64_t        magic;
  uint64_t        entries_length;
  archive_entry_t entries; // array
} archive_header_t;

typedef struct archive_iterator
{
  uint64_t         remaining;
  archive_entry_t *current;
} archive_iterator_t;

static inline archive_iterator_t archive_iterate(archive_header_t *header)
{
  archive_iterator_t iterator = {header->entries_length, &header->entries};

  return iterator;
}

static inline archive_entry_t *archive_next(archive_iterator_t *iterator)
{
  if (iterator->remaining > 0)
  {
    archive_entry_t *entry = iterator->current;

    iterator->remaining--;

    iterator->current = (archive_entry_t *)
      (((char *) entry) + 32 + entry->name_length);

    return entry;
  }
  else
  {
    return NULL;
  }
}

#define ARCHIVE_SYSTEM_NAME "system.kit"

extern archive_header_t *archive_system;

bool archive_initialize(uint64_t modules_count, multiboot_module_t *modules);

bool archive_get(archive_header_t *header, const char *entry_name,
    char **buffer, uint64_t *length);

bool archive_verify(archive_entry_t *entry, uint8_t *buffer);

int64_t archive_utils_spawn(const char *filename, int argc,
    const char *const *argv);

#endif
