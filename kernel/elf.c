/*******************************************************************************
 *
 * kit/kernel/elf.c
 * - Executable and Linkable Format loader
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "elf.h"
#include "memory.h"
#include "terminal.h"
#include "paging.h"
#include "debug.h"

bool elf_verify(elf_header_64_t *header)
{
  if (memory_compare(&header->e_ident.ei_magic, ELF_MAGIC, 4) != 0)
    return false;

  /* XXX: everything else here */

  if (header->e_ident.ei_class != ELF_EI_CLASS_64)
    return false;

  if (header->e_ident.ei_data != ELF_EI_DATA_2LSB)
    return false;

  if (header->e_ident.ei_version != 1)
    return false;

  if (header->e_ident.ei_os_abi != 0)
    return false;

  if (header->e_ident.ei_abi_version != 0)
    return false;

  if (header->e_type != ELF_E_TYPE_EXEC)
    return false;

  if (header->e_machine != ELF_E_MACHINE_AMD64)
    return false;

  return true;
}

void elf_program_header_print(elf_program_header_t *ph)
{
  terminal_writestring("  ");

  switch (ph->p_type)
  {
    case ELF_P_TYPE_NULL:
      terminal_writestring("NULL    "); break;
    case ELF_P_TYPE_LOAD:
      terminal_writestring("LOAD    "); break;
    case ELF_P_TYPE_DYNAMIC:
      terminal_writestring("DYNAMIC "); break;
    case ELF_P_TYPE_INTERP:
      terminal_writestring("INTERP  "); break;
    case ELF_P_TYPE_NOTE:
      terminal_writestring("NOTE    "); break;
    case ELF_P_TYPE_SHLIB:
      terminal_writestring("SHLIB   "); break;
    case ELF_P_TYPE_PHDR:
      terminal_writestring("PHDR    "); break;
    case ELF_P_TYPE_TLS:
      terminal_writestring("TLS     "); break;
    default:
      terminal_writestring("UNKNOWN ");
  }

  terminal_writechar(ph->p_flags & ELF_P_FLAG_READ    ? 'r' : '-');
  terminal_writechar(ph->p_flags & ELF_P_FLAG_WRITE   ? 'w' : '-');
  terminal_writechar(ph->p_flags & ELF_P_FLAG_EXECUTE ? 'x' : '-');

  terminal_printf(" %#lx (%lu) --> %#lx (%lu)\n",
      ph->p_offset, ph->p_filesz,
      ph->p_vaddr,  ph->p_memsz);
}

bool elf_load(elf_header_64_t *elf, process_t *process)
{
  // First, verify the ELF.
  if (!elf_verify(elf)) return false;

  // Then load the process's pageset.
  paging_pageset_t *old_pageset = paging_get_current_pageset();

  paging_set_current_pageset(&process->pageset);

  // Iterate through the program headers, following all LOAD instructions.
  bool exit_status = true;

  elf_program_header_iterator_t iterator = elf_program_header_iterate(elf);

  elf_program_header_t *ph;

  while ((ph = elf_program_header_next(&iterator)) != NULL)
  {
    switch (ph->p_type)
    {
      // Ignore NULL and PHDR instructions.
      case ELF_P_TYPE_NULL: break;
      case ELF_P_TYPE_PHDR: break;

      case ELF_P_TYPE_LOAD: {

        // Figure out which page flags to set.
        paging_flags_t flags = PAGING_USER;

        if (!(ph->p_flags & ELF_P_FLAG_WRITE))
          flags |= PAGING_READONLY;

        if (ph->p_flags & ELF_P_FLAG_EXECUTE)
          flags |= PAGING_EXECUTABLE;

        // Allocate memory as requested.
        if (process_alloc(process, (void *) ph->p_vaddr,
              ph->p_memsz, flags) != NULL)
        {
          // Copy from the file into the allocated area.
          memory_copy((void *) ((uintptr_t) elf + ph->p_offset),
              (void *) ph->p_vaddr, ph->p_filesz);

          // Zero the remainder if the in-file size is less than the in-memory
          // size. This is required by ELF.
          if (ph->p_filesz < ph->p_memsz)
          {
            memory_set((void *) ((uintptr_t) ph->p_vaddr + ph->p_filesz), 0,
                ph->p_memsz - ph->p_filesz);
          }

          break;
        }
        else
        {
          // Failing to allocate memory is an error.
          DEBUG_FORMAT("process_alloc() failed for ph=%p", (void *) ph);
          elf_program_header_print(ph);
          goto error;
        }
      };

      default:
        // Other instructions are not supported yet.
        DEBUG_FORMAT("unsupported p_type at ph=%p", (void *) ph);
        elf_program_header_print(ph);
        goto error;
    }
  }

  // Set the process's entry point.
  process_set_entry_point(process, (void *) elf->e_entry);

  goto exit;

error:
  exit_status = false;

exit:
  paging_set_current_pageset(old_pageset);
  return exit_status;
}
