/*******************************************************************************
 *
 * kit/kernel/include/paging.h
 * - kernel page management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PAGING_H
#define PAGING_H

#include <stdbool.h>
#include <stdint.h>

#include "config.h"
#include "memory.h"

/* Pageset (management helper) */

struct paging_rc_contents {
  int dummy;
  //...
};

typedef struct paging_rc_contents *paging_pageset_t;

/**
 * The kernel's pageset
 */
#define paging_kernel_pageset ((paging_pageset_t) NULL)

bool paging_create_pageset(paging_pageset_t *pageset);

bool paging_resolve_linear_address(paging_pageset_t pageset,
    const void *linear_address, uint64_t *physical_address);

#define PAGING_READONLY   0x01
#define PAGING_USER       0x02
#define PAGING_EXECUTABLE 0x04
typedef uint8_t paging_flags_t;

uint64_t paging_map(paging_pageset_t pageset, const void *linear_address,
    uint64_t physical_address, uint64_t pages, paging_flags_t flags);

uint64_t paging_unmap(paging_pageset_t pageset, const void *linear_address,
    uint64_t pages);

bool paging_get_flags(paging_pageset_t pageset, const void *linear_address,
    paging_flags_t *flags);

uint64_t paging_set_flags(paging_pageset_t pageset, const void *linear_address,
    uint64_t pages, paging_flags_t flags);

paging_pageset_t paging_get_current_pageset();

void paging_set_current_pageset(paging_pageset_t pageset);

paging_pageset_t paging_clone_ref(paging_pageset_t pageset);

void paging_drop_ref(paging_pageset_t *pageset);

#endif
