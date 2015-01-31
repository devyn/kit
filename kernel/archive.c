/*******************************************************************************
 *
 * kit/kernel/archive.c
 * - kit archive (init files) loader
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "archive.h"
#include "memory.h"
#include "string.h"
#include "debug.h"

bool archive_initialize(uint64_t modules_count, multiboot_module_t *modules)
{
  for (uint32_t i = 0; i < modules_count; i++)
  {
    char *cmdline = (modules[i].cmdline == 0 ? NULL :
                     (char *) KERNEL_OFFSET + modules[i].cmdline);

    if (cmdline != NULL)
    {
      if (string_compare(cmdline, ARCHIVE_SYSTEM_NAME) == 0)
      {
        archive_system = (archive_header_t *)
          (KERNEL_OFFSET + modules[i].mod_start);

        return true;
      }
    }
  }

  DEBUG_MESSAGE(ARCHIVE_SYSTEM_NAME " not found");

  return false;
}

bool archive_get(archive_header_t *header, const char *entry_name,
    char **buffer, uint64_t *length)
{
  size_t entry_name_length = string_length(entry_name);

  archive_iterator_t iterator = archive_iterate(header);

  archive_entry_t *entry;

  while ((entry = archive_next(&iterator)) != NULL)
  {
    if (memory_compare(&entry->name, entry_name,
          (entry_name_length > entry->name_length ?
           entry->name_length : entry_name_length)) == 0)
    {
      *buffer = ((char *) header) + entry->offset;
      *length = entry->length;

      return true;
    }
  }

  return false;
}
