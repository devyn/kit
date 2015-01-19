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

#include <stdint.h>

typedef struct paging_pageset {
  uint8_t dummy;
} paging_pageset_t;

void paging_initialize();

int paging_create_pageset(paging_pageset_t *pageset);

void paging_destroy_pageset(paging_pageset_t *pageset);

paging_pageset_t *paging_get_current_pageset();

void paging_set_current_pageset(paging_pageset_t *pageset);
