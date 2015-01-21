/*******************************************************************************
 *
 * kit/kernel/paging.c
 * - kernel page management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "paging.h"
#include "memory.h"
#include "x86_64.h"
#include "debug.h"

static void __paging_initialize_scan_pdpt(paging_pdpt_entry_t *pdpt);
static void __paging_initialize_scan_pd(paging_pd_entry_t *pd);

void paging_initialize()
{
  // Initialize the pageset.
  memory_set(&paging_kernel_pageset, 0, sizeof(paging_pageset_t));

  // Load the current PML4 location into pml4_physical.
  // This will be used as the basis for further pagesets.
  uint64_t cr3;
  __asm__ volatile("mov %%cr3, %0" : "=r" (cr3));

  // Get only the physical address.
  paging_kernel_pageset.pml4_physical = cr3 & (~((uint64_t) -1 << 51) << 1);

  // Offset to get the linear address in the new kernel space.
  paging_kernel_pageset.pml4 = (paging_pml4_entry_t *)
    (paging_kernel_pageset.pml4_physical + KERNEL_OFFSET);

  // Remove our identity map at 0 - 2 MB. We don't need it anymore.
  memory_set(&paging_kernel_pageset.pml4[0], 0, sizeof(paging_pml4_entry_t));

  // Invalidate the affected linear addresses.
  for (char *address = (char *) 0x0;
       address < (char *) 0x200000;
       address += 0x1000)
  {
    invlpg(address);
  }

  // We know that every page table so far is at physical address +
  // KERNEL_OFFSET. This won't always be the case, so we need to track
  // these in the pageset's table_map. We'll do this recursively through PML4,
  // PDPT and PD entries to yield PDPT, PD and PT table addresses.
  paging_pml4_entry_t *pml4 = paging_kernel_pageset.pml4;

  for (int i = 0; i < PAGING_PML4_SIZE; i++)
  {
    if (pml4[i].present)
    {
      uint64_t addr = pml4[i].pdpt_physical << 12;

      paging_phy_lin_map_set(&paging_kernel_pageset.table_map, addr,
          (void *) (addr + KERNEL_OFFSET));

      __paging_initialize_scan_pdpt(
          (paging_pdpt_entry_t *) (addr + KERNEL_OFFSET));
    }
  }

  uint64_t paging_initialize_phy;

  DEBUG_BEGIN_VALUES();
    DEBUG_HEX(&paging_initialize);
    if (paging_resolve_linear_address(&paging_kernel_pageset, 
          (void *) &paging_initialize, &paging_initialize_phy))
    {
      DEBUG_HEX(paging_initialize_phy);
    }
  DEBUG_END_VALUES();
}

static void __paging_initialize_scan_pdpt(paging_pdpt_entry_t *pdpt)
{
  for (int i = 0; i < PAGING_PDPT_SIZE; i++)
  {
    if (pdpt[i].info.present && pdpt[i].info.page_size == 0)
    {
      uint64_t addr = pdpt[i].as_pointer.pd_physical << 12;

      paging_phy_lin_map_set(&paging_kernel_pageset.table_map, addr,
          (void *) (addr + KERNEL_OFFSET));

      __paging_initialize_scan_pd(
          (paging_pd_entry_t *) (addr + KERNEL_OFFSET));
    }
  }
}

static void __paging_initialize_scan_pd(paging_pd_entry_t *pd)
{
  for (int i = 0; i < PAGING_PD_SIZE; i++)
  {
    if (pd[i].info.present && pd[i].info.page_size == 0)
    {
      uint64_t addr = pd[i].as_pointer.pt_physical << 12;

      paging_phy_lin_map_set(&paging_kernel_pageset.table_map, addr,
          (void *) (addr + KERNEL_OFFSET));
    }
  }
}

bool paging_phy_lin_map_get(paging_phy_lin_map_t *map,
    uint64_t physical_address, void **linear_address)
{
  paging_phy_lin_map_node_t *node =
    (paging_phy_lin_map_node_t *) map->tree.root;

  // Search by page frame, and add page offset later.
  uint64_t page_frame  = physical_address >> 12;
  uint64_t page_offset = physical_address & ~(-1 << 12);

  // Simple search through tree.
  while (node != NULL && node->page_frame != page_frame)
  {
    if (node->page_frame < page_frame)
      node = (paging_phy_lin_map_node_t *) node->node.right;
    else
      node = (paging_phy_lin_map_node_t *) node->node.left;
  }

  // Set linear_address and return true if found. Otherwise, return false.
  if (node != NULL)
  {
    *linear_address = (void *) ((node->page_number << 12) | page_offset);
    return true;
  }
  else
  {
    return false;
  }
}

void paging_phy_lin_map_set(paging_phy_lin_map_t *map,
    uint64_t physical_address, void *linear_address)
{
  paging_phy_lin_map_node_t *parent = NULL;

  paging_phy_lin_map_node_t *node =
    (paging_phy_lin_map_node_t *) map->tree.root;

  // Search and insert by page frame -> number.
  uint64_t page_frame  = physical_address >> 12;
  uint64_t page_number = ((uint64_t) linear_address) >> 12;

  // Simple search through tree.
  while (node != NULL && node->page_frame != page_frame)
  {
    // Keep a reference to the old node as the parent.
    parent = node;

    if (node->page_frame < page_frame)
      node = (paging_phy_lin_map_node_t *) node->node.right;
    else
      node = (paging_phy_lin_map_node_t *) node->node.left;
  }

  if (node == NULL)
  {
    // Not found; insert.
    node = memory_alloc(sizeof(paging_phy_lin_map_node_t));
    memory_set(node, 0, sizeof(paging_phy_lin_map_node_t));

    node->node.parent = (rbtree_node_t *) parent;
    node->page_frame  = page_frame;
    node->page_number = page_number;

    if (parent == NULL)
    {
      map->tree.root = &node->node;
    }
    else
    {
      // Insert it on the right side.
      if (page_frame < parent->page_frame)
        parent->node.left = (rbtree_node_t *) node;
      else
        parent->node.right = (rbtree_node_t *) node;

      rbtree_balance_insert(&map->tree, &node->node);
    }
  }
  else
  {
    // Found. Just update page number.
    node->page_number = page_number;
  }
}

void paging_phy_lin_map_delete(paging_phy_lin_map_t *map,
    uint64_t physical_address)
{
  paging_phy_lin_map_node_t *node =
    (paging_phy_lin_map_node_t *) map->tree.root;

  // Search by page frame.
  uint64_t page_frame = physical_address >> 12;

  // Simple search through tree.
  while (node != NULL && node->page_frame != page_frame)
  {
    if (node->page_frame < page_frame)
      node = (paging_phy_lin_map_node_t *) node->node.right;
    else
      node = (paging_phy_lin_map_node_t *) node->node.left;
  }

  // Delete if found.
  if (node != NULL)
  {
    rbtree_delete(&map->tree, &node->node);
    memory_free(node);
  }
}

bool paging_get_entry_pointers(paging_pageset_t *pageset,
    paging_linear64_t linear, paging_entries_t *entries)
{
  // Initialize
  memory_set(entries, 0, sizeof(paging_entries_t));

  // PML4 -> PDPT...
  paging_pml4_entry_t *pml4_entry = &pageset->pml4[linear.pml4_index];

  if (pml4_entry->present)
  {
    entries->pml4_entry = pml4_entry;

    paging_pdpt_entry_t *pdpt;

    DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
          pml4_entry->pdpt_physical << 12, (void *) &pdpt));

    // PDPT -> PD or page...
    paging_pdpt_entry_t *pdpt_entry = &pdpt[linear.pdpt_index];

    if (pdpt_entry->info.present)
    {
      entries->pdpt_entry = pdpt_entry;

      // If this is a 1 GB page, return immediately; otherwise continue
      if (pdpt_entry->info.page_size == 1)
        return true;

      paging_pd_entry_t *pd;

      DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
            pdpt_entry->as_pointer.pd_physical << 12, (void *) &pd));

      // PD -> PT or page...
      paging_pd_entry_t *pd_entry = &pd[linear.pd_index];

      if (pd_entry->info.present)
      {
        entries->pd_entry = pd_entry;

        // If this is a 2 MB page, return result immediately; otherwise continue
        if (pd_entry->info.page_size == 1)
          return true;

        paging_pt_entry_t *pt;

        DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
              pd_entry->as_pointer.pt_physical << 12, (void *) &pt));

        // PT -> page...
        paging_pt_entry_t *pt_entry = &pt[linear.pt_index];

        if (pt_entry->present)
        {
          entries->pt_entry = pt_entry;
          return true;
        }
      }
    }
  }

  return false;
}

bool paging_resolve_linear_address(paging_pageset_t *pageset,
    void *linear_address, uint64_t *physical_address)
{
  paging_linear64_t linear = paging_pointer_to_linear64(linear_address);

  paging_entries_t entries;

  if (!paging_get_entry_pointers(pageset, linear, &entries))
  {
    return false;
  }

  if (entries.pt_entry != NULL)
  {
    // Normal 4 kB page
    // Split at 12 bits
    *physical_address = (entries.pt_entry->page_physical << 12) | linear.offset;
  }
  else if (entries.pd_entry != NULL)
  {
    // Large 2 MB page
    // Split at 21 bits (= 9 + 12)
    *physical_address = (entries.pd_entry->as_page.page_physical << 21) |
      (((uint64_t) linear_address) & ~(-1 << 21));
  }
  else
  {
    // Huge 1 GB page
    // Split at 30 bits (= 9 + 9 + 12)
    *physical_address = (entries.pdpt_entry->as_page.page_physical << 30) |
      (((uint64_t) linear_address) & ~(-1 << 30));
  }

  return true;
}
