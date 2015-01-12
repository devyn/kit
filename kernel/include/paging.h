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

/* x86_64 PML4 */

#define PAGING_PML4_SIZE 512

typedef struct PACKED paging_pml4_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;  // accessible to user-mode if 1
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int zero1           : 6;
  unsigned int pdpt_physical   : 40; // shift 12
  unsigned int zero2           : 11;
  unsigned int execute_disable : 1;
} paging_pml4_entry_t;

typedef paging_pml4_entry_t paging_pml4_t[PAGING_PML4_SIZE];

/* x86_64 Page Directory Pointer Table */

#define PAGING_PDPT_SIZE 512

typedef struct PACKED paging_pdpt_pointer_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;  // accessible to user-mode if 1
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int zero1           : 6;
  unsigned int pd_physical     : 40; // shift 12
  unsigned int zero2           : 11;
  unsigned int execute_disable : 1;
} paging_pdpt_pointer_entry_t;

/* For 1 GB pages */
typedef struct PACKED paging_pdpt_page_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int dirty           : 1;
  unsigned int page_size       : 1;  // must be 1
  unsigned int global          : 1;
  unsigned int zero1           : 3;
  unsigned int pat             : 1;  // affects caching somehow
  unsigned int zero2           : 17;
  unsigned int page_physical   : 22; // shift 30
  unsigned int zero3           : 11;
  unsigned int execute_disable : 1;
} paging_pdpt_page_entry_t;

typedef union paging_pdpt_entry {
  /* 8 bytes */
  paging_pdpt_pointer_entry_t as_pointer;
  paging_pdpt_page_entry_t    as_page;

  struct {
    unsigned int ignore1   : 7;
    unsigned int page_size : 1; // 0 => as_pointer, 1 => as_page
  } info;
} paging_pdpt_entry_t;

typedef paging_pdpt_entry_t paging_pdpt_t[PAGING_PDPT_SIZE];

/* x86_64 Page Directory */

#define PAGING_PD_SIZE 512

typedef struct PACKED paging_pd_pointer_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;  // accessible to user-mode if 1
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int zero1           : 6;
  unsigned int pt_physical     : 40; // shift 12
  unsigned int zero2           : 11;
  unsigned int execute_disable : 1;
} paging_pd_pointer_entry_t;

/* For 2 MB pages */
typedef struct PACKED paging_pd_page_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int dirty           : 1;
  unsigned int page_size       : 1;  // must be 1
  unsigned int global          : 1;
  unsigned int zero1           : 3;
  unsigned int pat             : 1;  // affects caching somehow
  unsigned int zero2           : 8;
  unsigned int page_physical   : 31; // shift 21
  unsigned int zero3           : 11;
  unsigned int execute_disable : 1;
  /* 8 bytes */
} paging_pd_page_entry_t;

typedef union paging_pd_entry {
  /* 8 bytes */
  paging_pd_pointer_entry_t as_pointer;
  paging_pd_page_entry_t    as_page;

  struct {
    unsigned int ignore1   : 7;
    unsigned int page_size : 1; // 0 => as_pointer, 1 => as_page
  } info;
} paging_pd_entry_t;

typedef paging_pd_entry_t paging_pd_t[PAGING_PD_SIZE];

/* x86_64 Page Table */

#define PAGING_PT_SIZE 512

/* For 4 KB pages */
typedef struct PACKED paging_pt_entry {
  /* 8 bytes */
  unsigned int present         : 1;
  unsigned int writable        : 1;
  unsigned int user            : 1;
  unsigned int write_through   : 1;
  unsigned int cache_disable   : 1;
  unsigned int accessed        : 1;
  unsigned int dirty           : 1;
  unsigned int pat             : 1;  // affects caching somehow
  unsigned int global          : 1;
  unsigned int zero1           : 3;
  unsigned int page_physical   : 40; // shift 12
  unsigned int zero2           : 11;
  unsigned int execute_disable : 1;
} paging_pt_entry_t;
