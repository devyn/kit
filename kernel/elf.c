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
