/*******************************************************************************
 *
 * kit/kernel/include/paging.h
 * - kernel page management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PAGING_H
#define PAGING_H

#include <stdbool.h>
#include <stdint.h>

#include "config.h"
#include "rbtree.h"

/* Call this on system initialization. */

void paging_initialize();

/* x86_64 PML4 */

#define PAGING_PML4_SIZE    512
#define PAGING_PML4_HALF    256
#define PAGING_PML4_4KPAGES (512 * 512 * 512 * 512)

typedef struct PACKED paging_pml4_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;  // accessible to user-mode if 1
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long zero1           : 6;
  unsigned long pdpt_physical   : 40; // shift 12
  unsigned long zero2           : 11;
  unsigned long execute_disable : 1;
} paging_pml4_entry_t;

/* x86_64 Page Directory Pointer Table */

#define PAGING_PDPT_SIZE    512
#define PAGING_PDPT_4KPAGES (512 * 512 * 512)

typedef struct PACKED paging_pdpt_pointer_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;  // accessible to user-mode if 1
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long zero1           : 6;
  unsigned long pd_physical     : 40; // shift 12
  unsigned long zero2           : 11;
  unsigned long execute_disable : 1;
} paging_pdpt_pointer_entry_t;

/* For 1 GB pages */
typedef struct PACKED paging_pdpt_page_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long dirty           : 1;
  unsigned long page_size       : 1;  // must be 1
  unsigned long global          : 1;
  unsigned long zero1           : 3;
  unsigned long pat             : 1;  // affects caching somehow
  unsigned long zero2           : 17;
  unsigned long page_physical   : 22; // shift 30
  unsigned long zero3           : 11;
  unsigned long execute_disable : 1;
} paging_pdpt_page_entry_t;

typedef union paging_pdpt_entry {
  /* 8 bytes */
  paging_pdpt_pointer_entry_t as_pointer;
  paging_pdpt_page_entry_t    as_page;

  struct {
    unsigned long present   : 1;
    unsigned long ignore1   : 6;
    unsigned long page_size : 1; // 0 => as_pointer, 1 => as_page
  } info;
} paging_pdpt_entry_t;

/* x86_64 Page Directory */

#define PAGING_PD_SIZE    512
#define PAGING_PD_4KPAGES (512 * 512)

typedef struct PACKED paging_pd_pointer_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;  // accessible to user-mode if 1
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long zero1           : 6;
  unsigned long pt_physical     : 40; // shift 12
  unsigned long zero2           : 11;
  unsigned long execute_disable : 1;
} paging_pd_pointer_entry_t;

/* For 2 MB pages */
typedef struct PACKED paging_pd_page_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long dirty           : 1;
  unsigned long page_size       : 1;  // must be 1
  unsigned long global          : 1;
  unsigned long zero1           : 3;
  unsigned long pat             : 1;  // affects caching somehow
  unsigned long zero2           : 8;
  unsigned long page_physical   : 31; // shift 21
  unsigned long zero3           : 11;
  unsigned long execute_disable : 1;
  /* 8 bytes */
} paging_pd_page_entry_t;

typedef union paging_pd_entry {
  /* 8 bytes */
  paging_pd_pointer_entry_t as_pointer;
  paging_pd_page_entry_t    as_page;

  struct {
    unsigned long present   : 1;
    unsigned long ignore1   : 6;
    unsigned long page_size : 1; // 0 => as_pointer, 1 => as_page
  } info;
} paging_pd_entry_t;

/* x86_64 Page Table */

#define PAGING_PT_SIZE    512
#define PAGING_PT_4KPAGES 512

/* For 4 KB pages */
typedef struct PACKED paging_pt_entry {
  /* 8 bytes */
  unsigned long present         : 1;
  unsigned long writable        : 1;
  unsigned long user            : 1;
  unsigned long write_through   : 1;
  unsigned long cache_disable   : 1;
  unsigned long accessed        : 1;
  unsigned long dirty           : 1;
  unsigned long pat             : 1;  // affects caching somehow
  unsigned long global          : 1;
  unsigned long zero1           : 3;
  unsigned long page_physical   : 40; // shift 12
  unsigned long zero2           : 11;
  unsigned long execute_disable : 1;
} paging_pt_entry_t;

/* All x86_64 paging entry pointers in one structure */

typedef struct paging_entries {
  paging_pml4_entry_t *pml4_entry;
  paging_pdpt_entry_t *pdpt_entry;
  paging_pd_entry_t   *pd_entry;
  paging_pt_entry_t   *pt_entry;
} paging_entries_t;

/* x86_64 Linear Address structure, 48-bit on 64-bit */

typedef struct PACKED paging_linear64 {
  unsigned long offset     : 12;
  unsigned long pt_index   : 9;
  unsigned long pd_index   : 9;
  unsigned long pdpt_index : 9;
  unsigned long pml4_index : 9;

  // pml4_index & (1 << 9) ? 0xFFFF : 0x0000
  // otherwise an exception will be generated
  unsigned long prefix     : 16;
} paging_linear64_t;

static inline paging_linear64_t paging_pointer_to_linear64(void *pointer)
{
  union {
    paging_linear64_t  linear;
    void              *pointer;
  } intermediate;

  intermediate.pointer = pointer;
  return intermediate.linear;
}

static inline void *paging_linear64_to_pointer(paging_linear64_t linear)
{
  union {
    paging_linear64_t  linear;
    void              *pointer;
  } intermediate;

  intermediate.linear = linear;
  return intermediate.pointer;
}

/* Physical -> Linear map */

typedef struct paging_phy_lin_map {
  rbtree_t tree;
  uint64_t entries;
} paging_phy_lin_map_t;

typedef struct paging_phy_lin_map_node {
  rbtree_node_t  node;

  uint64_t page_frame;
  uint64_t page_number;
} paging_phy_lin_map_node_t;

bool paging_phy_lin_map_get(paging_phy_lin_map_t *map,
    uint64_t physical_address, void **linear_address);

void paging_phy_lin_map_set(paging_phy_lin_map_t *map,
    uint64_t physical_address, void *linear_address);

void paging_phy_lin_map_delete(paging_phy_lin_map_t *map,
    uint64_t physical_address);

void paging_phy_lin_map_clear(paging_phy_lin_map_t *map);

/* Pageset (management helper) */

typedef struct paging_pageset {
  uint64_t               pml4_physical; // build CR3 with this
  paging_pml4_entry_t   *pml4;

  paging_phy_lin_map_t   table_map;
} paging_pageset_t;

/**
 * The kernel's pageset
 */
paging_pageset_t paging_kernel_pageset;

bool paging_create_pageset(paging_pageset_t *pageset);

bool paging_destroy_pageset(paging_pageset_t *pageset);

/**
 * Low-level access
 * Returns true if complete
 */
bool paging_get_entry_pointers(paging_pageset_t *pageset,
    paging_linear64_t linear, paging_entries_t *entries);

bool paging_resolve_linear_address(paging_pageset_t *pageset,
    void *linear_address, uint64_t *physical_address);

typedef enum paging_flags {
  PAGING_READONLY = 0x1
} paging_flags_t;

uint64_t paging_map(paging_pageset_t *pageset, void *linear_address,
    uint64_t physical_address, uint64_t pages, paging_flags_t flags);

uint64_t paging_unmap(paging_pageset_t *pageset, void *linear_address,
    uint64_t pages);

bool paging_get_flags(paging_pageset_t *pageset, void *linear_address,
    paging_flags_t *flags);

uint64_t paging_set_flags(paging_pageset_t *pageset, void *linear_address,
    uint64_t pages, paging_flags_t flags);

paging_pageset_t *paging_get_current_pageset();

void paging_set_current_pageset(paging_pageset_t *pageset);

#endif
