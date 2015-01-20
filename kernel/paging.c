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

static paging_pageset_t paging_kernel_pageset;

static const uint64_t KERNEL_OFFSET = 0xffff800000000000;

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

  // Debug
  DEBUG_BEGIN_VALUES();
    DEBUG_HEX(paging_kernel_pageset.pml4_physical);
  DEBUG_END_VALUES();
  DEBUG_BEGIN_VALUES();
    DEBUG_HEX(paging_kernel_pageset.pml4);
  DEBUG_END_VALUES();

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

  // Debug: print results
  void *address;

  if (paging_phy_lin_map_get(&paging_kernel_pageset.table_map,
        0x5000, &address))
  {
    DEBUG_MESSAGE_HEX("found 0x5000", (uint64_t) address);
  }
  else
  {
    DEBUG_MESSAGE("not found");
  }

  paging_phy_lin_map_node_t *node = (paging_phy_lin_map_node_t *)
    rbtree_first_node(&paging_kernel_pageset.table_map.tree);

  while (node != NULL)
  {
    DEBUG_BEGIN_VALUES();
      DEBUG_HEX(node->page_frame);
      DEBUG_HEX(node->page_number);
    DEBUG_END_VALUES();

    node = (paging_phy_lin_map_node_t *) rbtree_node_next(&node->node);
  }
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
