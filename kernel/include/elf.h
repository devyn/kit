/*******************************************************************************
 *
 * kit/kernel/include/elf.h
 * - Executable and Linkable Format loader
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef ELF_H
#define ELF_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

#include "config.h"
#include "process.h"

static const uint8_t ELF_MAGIC[4] = {0x7f, 'E', 'L', 'F'};

typedef struct PACKED elf_header_ident
{
  /* 16 bytes */

  uint8_t ei_magic[4];

# define ELF_EI_CLASS_NONE 0
# define ELF_EI_CLASS_32   1
# define ELF_EI_CLASS_64   2
  uint8_t ei_class;

# define ELF_EI_DATA_NONE 0
# define ELF_EI_DATA_2LSB 1 /* little endian */
# define ELF_EI_DATA_2MSB 2 /* big endian */
  uint8_t ei_data;

  uint8_t ei_version;
  uint8_t ei_os_abi;
  uint8_t ei_abi_version;

  uint8_t ei_pad[7];
} elf_header_ident_t;

typedef struct PACKED elf_header_64
{
  elf_header_ident_t e_ident;

# define ELF_E_TYPE_NONE   0
# define ELF_E_TYPE_REL    1
# define ELF_E_TYPE_EXEC   2
# define ELF_E_TYPE_DYN    3
# define ELF_E_TYPE_CORE   4
  uint16_t e_type;

# define ELF_E_MACHINE_NONE  0
# define ELF_E_MACHINE_386   3
# define ELF_E_MACHINE_AMD64 62
  uint16_t e_machine;

  uint32_t e_version;
  uint64_t e_entry;
  uint64_t e_phoff;
  uint64_t e_shoff;

  // TODO: (processor specific flags)
  uint32_t e_flags;

  uint16_t e_ehsize;
  uint16_t e_phentsize;
  uint16_t e_phnum;
  uint16_t e_shentsize;
  uint16_t e_shnum;
  uint16_t e_shstrndx;
} elf_header_64_t;

typedef struct PACKED elf_program_header
{
# define ELF_P_TYPE_NULL    0
# define ELF_P_TYPE_LOAD    1
# define ELF_P_TYPE_DYNAMIC 2
# define ELF_P_TYPE_INTERP  3
# define ELF_P_TYPE_NOTE    4
# define ELF_P_TYPE_SHLIB   5
# define ELF_P_TYPE_PHDR    6
# define ELF_P_TYPE_TLS     7
  uint32_t p_type;

# define ELF_P_FLAG_READ    4
# define ELF_P_FLAG_WRITE   2
# define ELF_P_FLAG_EXECUTE 1
  uint32_t p_flags;

  uint64_t p_offset;
  uint64_t p_vaddr;
  uint64_t p_paddr;
  uint64_t p_filesz;
  uint64_t p_memsz;
  uint64_t p_align;
} elf_program_header_t;

typedef struct elf_program_header_iterator
{
  uint16_t               remaining;
  uint16_t               entry_size;
  elf_program_header_t  *current;
} elf_program_header_iterator_t;

static inline elf_program_header_iterator_t elf_program_header_iterate(
    elf_header_64_t *header)
{
  elf_program_header_iterator_t iterator =
    {header->e_phnum, header->e_phentsize,
      (elf_program_header_t *) (((char *) header) + header->e_phoff)};

  return iterator;
}

static inline elf_program_header_t *elf_program_header_next(
    elf_program_header_iterator_t *iterator)
{
  if (iterator->remaining > 0)
  {
    elf_program_header_t *entry = iterator->current;

    iterator->remaining--;

    iterator->current = (elf_program_header_t *)
      (((char *) entry) + iterator->entry_size);

    return entry;
  }
  else
  {
    return NULL;
  }
}

bool elf_verify(elf_header_64_t *header);

void elf_program_header_print(elf_program_header_t *ph);

bool elf_load(elf_header_64_t *header, process_t *process);

#endif
