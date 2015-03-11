/*******************************************************************************
 *
 * kit/kernel/include/paging.h
 * - kernel page management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
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

/* Pageset (management helper) */

struct paging_dummy {
  int foo;
};

typedef struct paging_dummy *paging_pageset_t;

/**
 * The kernel's pageset
 */
#define paging_kernel_pageset ((paging_pageset_t) NULL)

bool paging_create_pageset(paging_pageset_t *pageset);

bool paging_destroy_pageset(paging_pageset_t *pageset);

bool paging_resolve_linear_address(paging_pageset_t pageset,
    void *linear_address, uint64_t *physical_address);

typedef enum paging_flags {
  PAGING_READONLY    = 0x01,
  PAGING_USER        = 0x02,
  PAGING_EXECUTABLE  = 0x04, // TODO
} paging_flags_t;

uint64_t paging_map(paging_pageset_t pageset, void *linear_address,
    uint64_t physical_address, uint64_t pages, paging_flags_t flags);

uint64_t paging_unmap(paging_pageset_t pageset, void *linear_address,
    uint64_t pages);

bool paging_get_flags(paging_pageset_t pageset, void *linear_address,
    paging_flags_t *flags);

uint64_t paging_set_flags(paging_pageset_t pageset, void *linear_address,
    uint64_t pages, paging_flags_t flags);

paging_pageset_t paging_get_current_pageset();

void paging_set_current_pageset(paging_pageset_t pageset);

#endif
