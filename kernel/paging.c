/*******************************************************************************
 *
 * kit/kernel/paging.c
 * - kernel page management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "paging.h"
#include "memory.h"
#include "x86_64.h"
#include "debug.h"

static paging_pageset_t *paging_current_pageset;

static void __paging_initialize_scan_pdpt(paging_pdpt_entry_t *pdpt);
static void __paging_initialize_scan_pd(paging_pd_entry_t *pd);

void paging_initialize()
{
  // Set current pageset to kernel pageset.
  paging_current_pageset = &paging_kernel_pageset;

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

  // FIXME: This is a hack to make 0xffff888800000000 available, but really we
  // need to create all of the PDPTs we ever want to use in the kernel right
  // here, since any changes to the PML4 further on won't propagate.
  paging_map  (&paging_kernel_pageset, (void *) 0xffff888800000000, 0, 1, 0);
  paging_unmap(&paging_kernel_pageset, (void *) 0xffff888800000000, 1);
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

    map->entries++;
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

    map->entries--;
  }
}

static void __paging_phy_lin_map_free_node_recursive(
    paging_phy_lin_map_node_t *node)
{
  if (node->node.left != NULL)
    __paging_phy_lin_map_free_node_recursive(
        (paging_phy_lin_map_node_t *) node->node.left);

  if (node->node.right != NULL)
    __paging_phy_lin_map_free_node_recursive(
        (paging_phy_lin_map_node_t *) node->node.right);

  memory_free(node);
}

void paging_phy_lin_map_clear(paging_phy_lin_map_t *map)
{
  if (map->tree.root != NULL)
    __paging_phy_lin_map_free_node_recursive(
        (paging_phy_lin_map_node_t *) map->tree.root);

  map->tree.root = NULL;
  map->entries = 0;
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

    paging_pdpt_entry_t *pdpt = NULL;

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

      paging_pd_entry_t *pd = NULL;

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

        paging_pt_entry_t *pt = NULL;

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

  // If the address is in the higher half, this must be kernel memory,
  // so we'll silently use the kernel pageset instead. Otherwise we'd cause an
  // assertion to fail once we get to the PML4 and try to look up the virtual
  // address of the PDPT.
  if (linear.prefix == 0xffff && pageset != &paging_kernel_pageset)
  {
    pageset = &paging_kernel_pageset;
  }

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

/**
 * Allocate a page in the kernel heap and get the physical address in addition
 * to the linear address.
 */
static inline void *__paging_alloc_page_phy_lin(uint64_t *physical_address)
{
  void *page = memory_alloc_aligned(4096, 4096);

  if (page != NULL)
  {
    DEBUG_ASSERT(paging_resolve_linear_address(&paging_kernel_pageset,
          page, physical_address));
  }

  return page;
}

bool paging_create_pageset(paging_pageset_t *pageset)
{
  // Clear memory.
  memory_set(pageset, 0, sizeof(paging_pageset_t));

  // Allocate space for a PML4 table and get its physical address.
  pageset->pml4 = __paging_alloc_page_phy_lin(&pageset->pml4_physical);

  if (pageset->pml4 != NULL)
  {
    // Zero the lower half of the PML4.
    const int half = PAGING_PML4_HALF;
    memory_set(pageset->pml4, 0, sizeof(paging_pml4_entry_t) * half);

    // Copy the kernel's PML4 higher half to this PML4.
    memory_copy(paging_kernel_pageset.pml4 + half, pageset->pml4 + half,
        sizeof(paging_pml4_entry_t) * half);

    return true;
  }
  else
  {
    return false;
  }
}

static void __paging_destroy_pageset_pml4(paging_pageset_t *pageset);

static void __paging_destroy_pageset_pdpt(paging_pageset_t *pageset,
    paging_pdpt_entry_t *pdpt);

static void __paging_destroy_pageset_pd(paging_pageset_t *pageset,
    paging_pd_entry_t *pd);

static void __paging_destroy_pageset_pt(paging_pt_entry_t *pt);

bool paging_destroy_pageset(paging_pageset_t *pageset)
{
  if (pageset != &paging_kernel_pageset)
  {
    // Free the tables.
    __paging_destroy_pageset_pml4(pageset);

    // Clear out the table map.
    paging_phy_lin_map_clear(&pageset->table_map);

    return true;
  }
  else
  {
    // Refuse to destroy the kernel pageset.
    return false;
  }
}

static void __paging_destroy_pageset_pml4(paging_pageset_t *pageset)
{
  // Only go up to the higher half, since everything above that is kernel space.
  for (int i = 0; i < PAGING_PML4_HALF; i++)
  {
    if (pageset->pml4[i].present)
    {
      paging_pdpt_entry_t *pdpt = NULL;

      DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
            pageset->pml4[i].pdpt_physical << 12, (void *) &pdpt));

      __paging_destroy_pageset_pdpt(pageset, pdpt);
    }
  }

  memory_free(pageset->pml4);
}

static void __paging_destroy_pageset_pdpt(paging_pageset_t *pageset,
    paging_pdpt_entry_t *pdpt)
{
  for (int i = 0; i < PAGING_PDPT_SIZE; i++)
  {
    if (pdpt[i].info.present && pdpt[i].info.page_size == 0)
    {
      paging_pd_entry_t *pd = NULL;

      DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
            pdpt[i].as_pointer.pd_physical << 12, (void *) &pd));

      __paging_destroy_pageset_pd(pageset, pd);
    }
  }

  memory_free(pdpt);
}

static void __paging_destroy_pageset_pd(paging_pageset_t *pageset,
    paging_pd_entry_t *pd)
{
  for (int i = 0; i < PAGING_PD_SIZE; i++)
  {
    if (pd[i].info.present && pd[i].info.page_size == 0)
    {
      paging_pt_entry_t *pt = NULL;

      DEBUG_ASSERT(paging_phy_lin_map_get(&pageset->table_map,
            pd[i].as_pointer.pt_physical << 12, (void *) &pt));

      __paging_destroy_pageset_pt(pt);
    }
  }

  memory_free(pd);
}

static void __paging_destroy_pageset_pt(paging_pt_entry_t *pt)
{
  memory_free(pt);
}

typedef struct paging_map_state {
  paging_pageset_t  *pageset;

  union {
    paging_linear64_t  indices;
    uint8_t           *pointer;
  } linear;

  uint64_t           physical;
  uint64_t           mapped;
  uint64_t           requested;
  paging_flags_t     flags;
  bool               error;
} paging_map_state_t;

static void __paging_map_pml4(paging_map_state_t *state);

static void __paging_map_pdpt(paging_map_state_t *state,
    paging_pdpt_entry_t *pdpt);

static void __paging_map_pd(paging_map_state_t *state, paging_pd_entry_t *pd);

static void __paging_map_pt(paging_map_state_t *state, paging_pt_entry_t *pt);

uint64_t paging_map(paging_pageset_t *pageset, void *linear_address,
    uint64_t physical_address, uint64_t pages, paging_flags_t flags)
{
  paging_map_state_t state;

  state.pageset        = pageset;
  state.linear.pointer = (uint8_t *) linear_address;
  state.physical       = physical_address;
  state.mapped         = 0;
  state.requested      = pages;
  state.flags          = flags;
  state.error          = false;

  __paging_map_pml4(&state);

  return state.mapped;
}

static void __paging_map_pml4(paging_map_state_t *state)
{
  // Establish the max index: refuse to go past the higher half if this isn't
  // the kernel pageset.
  int max_index;
  if (state->pageset == &paging_kernel_pageset)
    max_index = PAGING_PML4_SIZE - 1;
  else
    max_index = PAGING_PML4_HALF - 1;

  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pml4_index;

  // Start mapping PDPTs.
  while (!state->error &&
      index <= max_index &&
      state->mapped < state->requested)
  {
    paging_pml4_entry_t *pml4_entry =
      state->pageset->pml4 + state->linear.indices.pml4_index;

    paging_pdpt_entry_t *pdpt = NULL;

    // If not present, we need to allocate it.
    if (!pml4_entry->present)
    {
      uint64_t pdpt_physical = 0;

      pdpt = __paging_alloc_page_phy_lin(&pdpt_physical);

      if (pdpt == NULL)
      {
        // Allocator error. Probably out of memory.
        state->error = true;
        break;
      }

      // Clear the PML4 entry.
      memory_set(pml4_entry, 0, sizeof(paging_pml4_entry_t));

      // Clear the PDPT.
      memory_set(pdpt, 0, sizeof(paging_pdpt_entry_t) * PAGING_PDPT_SIZE);

      // Map PML4 entry to PDPT.
      pml4_entry->pdpt_physical = pdpt_physical >> 12;

      // Set flags and mark as present.
      // Only page mapping entries should be affected by state->flags;
      // everything should be permitted at higher levels.
      pml4_entry->writable = 1;
      pml4_entry->user     = 1;
      pml4_entry->present  = 1;

      // Insert into table map.
      paging_phy_lin_map_set(&state->pageset->table_map, pdpt_physical, pdpt);
    }
    else
    {
      // Otherwise, we just need to look it up.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pml4_entry->pdpt_physical << 12, (void **) &pdpt));
    }

    // Now we can map the PDPT.
    __paging_map_pdpt(state, pdpt);
    index++;
  }

  // Log if we attempted to exceed the max index.
  if (index > max_index && state->mapped < state->requested)
  {
    DEBUG_MESSAGE("attempted to exceed max index");
  }
}

static void __paging_map_pdpt(paging_map_state_t *state,
    paging_pdpt_entry_t *pdpt)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pdpt_index;

  // Start mapping PDs.
  while (!state->error &&
      index < PAGING_PDPT_SIZE &&
      state->mapped < state->requested)
  {
    paging_pdpt_entry_t *pdpt_entry = pdpt + state->linear.indices.pdpt_index;

    paging_pd_entry_t *pd = NULL;

    // If not present, we need to allocate it.
    if (!pdpt_entry->info.present)
    {
      uint64_t pd_physical = 0;

      pd = __paging_alloc_page_phy_lin(&pd_physical);

      if (pd == NULL)
      {
        // Allocator error. Probably out of memory.
        state->error = true;
        break;
      }

      // Clear the PDPT entry.
      memory_set(pdpt_entry, 0, sizeof(paging_pdpt_entry_t));

      // Clear the PD.
      memory_set(pd, 0, sizeof(paging_pd_entry_t) * PAGING_PD_SIZE);

      // Map PDPT entry to PD.
      pdpt_entry->as_pointer.pd_physical = pd_physical >> 12;

      // Set flags and mark as present.
      // Only page mapping entries should be affected by state->flags;
      // everything should be permitted at higher levels.
      pdpt_entry->as_pointer.writable = 1;
      pdpt_entry->as_pointer.user     = 1;
      pdpt_entry->as_pointer.present  = 1;

      // Insert into table map.
      paging_phy_lin_map_set(&state->pageset->table_map, pd_physical, pd);
    }
    else if (pdpt_entry->info.page_size == 1)
    {
      // Can't map into a 1 GB page PDPT entry.
      DEBUG_MESSAGE_HEX("tried to map into a 1 GB page PDPT entry",
          (uint64_t) pdpt_entry);

      state->error = true;
      break;
    }
    else
    {
      // Otherwise, we just need to look it up.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pdpt_entry->as_pointer.pd_physical << 12, (void **) &pd));
    }

    // Now we can map the PD.
    __paging_map_pd(state, pd);
    index++;
  }
}

static void __paging_map_pd(paging_map_state_t *state, paging_pd_entry_t *pd)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pd_index;

  // Start mapping PTs.
  while (!state->error &&
      index < PAGING_PD_SIZE &&
      state->mapped < state->requested)
  {
    paging_pd_entry_t *pd_entry = pd + state->linear.indices.pd_index;

    paging_pt_entry_t *pt = NULL;

    // If not present, we need to allocate it.
    if (!pd_entry->info.present)
    {
      uint64_t pt_physical = 0;

      pt = __paging_alloc_page_phy_lin(&pt_physical);

      if (pt == NULL)
      {
        // Allocator error. Probably out of memory.
        state->error = true;
        break;
      }

      // Clear the PD entry.
      memory_set(pd_entry, 0, sizeof(paging_pd_entry_t));

      // Clear the PT.
      memory_set(pt, 0, sizeof(paging_pt_entry_t) * PAGING_PT_SIZE);

      // Map PD entry to PT.
      pd_entry->as_pointer.pt_physical = pt_physical >> 12;

      // Set flags and mark as present.
      // Only page mapping entries should be affected by state->flags;
      // everything should be permitted at higher levels.
      pd_entry->as_pointer.writable = 1;
      pd_entry->as_pointer.user     = 1;
      pd_entry->as_pointer.present  = 1;

      // Insert into table map.
      paging_phy_lin_map_set(&state->pageset->table_map, pt_physical, pt);
    }
    else if (pd_entry->info.page_size == 1)
    {
      // Can't map into a 2 MB page PD entry.
      DEBUG_MESSAGE_HEX("tried to map into a 2 MB page PD entry",
          (uint64_t) pd_entry);

      state->error = true;
      break;
    }
    else
    {
      // Otherwise, we just need to look it up.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pd_entry->as_pointer.pt_physical << 12, (void **) &pt));
    }

    // Now we can map the PT.
    __paging_map_pt(state, pt);
    index++;
  }
}

static void __paging_map_pt(paging_map_state_t *state, paging_pt_entry_t *pt)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pt_index;

  // Start mapping pages.
  while (!state->error &&
      index < PAGING_PT_SIZE &&
      state->mapped < state->requested)
  {
    paging_pt_entry_t *pt_entry = pt + state->linear.indices.pt_index;

    // If present, abort. We must refuse to map pages that are already mapped.
    if (pt_entry->present)
    {
      DEBUG_MESSAGE_HEX("tried to map into a present PT entry",
          (uint64_t) pt_entry);

      state->error = true;
      break;
    }
    else
    {
      // Clear the PT entry.
      memory_set(pt_entry, 0, sizeof(paging_pt_entry_t));

      // Map PT entry to page.
      pt_entry->page_physical = state->physical >> 12;

      // Set flags and mark as present.
      if (!(state->flags & PAGING_READONLY))
        pt_entry->writable = 1;

      if (state->flags & PAGING_USER)
        pt_entry->user = 1;

      pt_entry->present = 1;

      // Increment mapped by 1, and advance linear and physical by 1 page.
      state->mapped         += 1;
      state->linear.pointer += 0x1000;
      state->physical       += 0x1000;

      index++;
    }
  }
}

typedef struct paging_unmap_state {
  paging_pageset_t  *pageset;

  union {
    paging_linear64_t  indices;
    uint8_t           *pointer;
  } linear;

  uint64_t           unmapped;
  uint64_t           requested;
  bool               error;
} paging_unmap_state_t;

static void __paging_unmap_pml4(paging_unmap_state_t *state);

static void __paging_unmap_pdpt(paging_unmap_state_t *state,
    paging_pdpt_entry_t *pdpt);

static void __paging_unmap_pd(paging_unmap_state_t *state,
    paging_pd_entry_t *pd);

static void __paging_unmap_pt(paging_unmap_state_t *state,
    paging_pt_entry_t *pt);

uint64_t paging_unmap(paging_pageset_t *pageset, void *linear_address,
    uint64_t pages)
{
  paging_unmap_state_t state;

  state.pageset        = pageset;
  state.linear.pointer = (uint8_t *) linear_address;
  state.unmapped       = 0;
  state.requested      = pages;
  state.error          = false;

  __paging_unmap_pml4(&state);

  return state.unmapped;
}

static void __paging_unmap_pml4(paging_unmap_state_t *state)
{
  // Establish the max index: refuse to go past the higher half if this isn't
  // the kernel pageset.
  int max_index;
  if (state->pageset == &paging_kernel_pageset)
    max_index = PAGING_PML4_SIZE - 1;
  else
    max_index = PAGING_PML4_HALF - 1;

  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pml4_index;

  // Start unmapping PDPTs.
  while (!state->error &&
      index <= max_index &&
      state->unmapped < state->requested)
  {
    paging_pml4_entry_t *pml4_entry =
      state->pageset->pml4 + state->linear.indices.pml4_index;

    paging_pdpt_entry_t *pdpt = NULL;

    // If not present, we need to skip it.
    if (!pml4_entry->present)
    {
      state->linear.pointer += (uint64_t) PAGING_PDPT_4KPAGES * 0x1000;

      if (state->unmapped + PAGING_PDPT_4KPAGES > state->requested)
        state->unmapped = state->requested;
      else
        state->unmapped += PAGING_PDPT_4KPAGES;
    }
    else
    {
      // Otherwise, we just need to look it up and unmap it.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pml4_entry->pdpt_physical << 12, (void **) &pdpt));

      __paging_unmap_pdpt(state, pdpt);
    }

    index++;
  }

  // Log if we attempted to exceed the max index.
  if (index > max_index && state->unmapped < state->requested)
  {
    DEBUG_MESSAGE("attempted to exceed max index");
  }
}

static void __paging_unmap_pdpt(paging_unmap_state_t *state,
    paging_pdpt_entry_t *pdpt)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pdpt_index;

  // Start unmapping PDs or pages.
  while (!state->error &&
      index < PAGING_PDPT_SIZE &&
      state->unmapped < state->requested)
  {
    paging_pdpt_entry_t *pdpt_entry = pdpt + state->linear.indices.pdpt_index;

    paging_pd_entry_t *pd = NULL;

    // If not present, we need to skip it.
    if (!pdpt_entry->info.present)
    {
      state->linear.pointer += PAGING_PD_4KPAGES * 0x1000;

      if (state->unmapped + PAGING_PD_4KPAGES > state->requested)
        state->unmapped = state->requested;
      else
        state->unmapped += PAGING_PD_4KPAGES;
    }
    else if (pdpt_entry->info.page_size == 1)
    {
      // If this is a 1 GB page, then we need to figure out if the number of 4
      // kB pages to unmap is equal to or greater than the number of 4 kB pages
      // that would be unmapped by unmapping this page.
      if ((state->requested - state->unmapped) >= PAGING_PD_4KPAGES)
      {
        pdpt_entry->as_page.present = 0;

        invlpg(state->linear.pointer);

        state->linear.pointer += PAGING_PD_4KPAGES * 0x1000;
        state->unmapped       += PAGING_PD_4KPAGES;
      }
      else
      {
        DEBUG_MESSAGE("tried to unmap into a 1 GB page");
        state->error = true;
        break;
      }
    }
    else
    {
      // Otherwise, we just need to look it up and unmap it.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pdpt_entry->as_pointer.pd_physical << 12, (void **) &pd));

      __paging_unmap_pd(state, pd);
    }

    index++;
  }
}

static void __paging_unmap_pd(paging_unmap_state_t *state,
    paging_pd_entry_t *pd)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pd_index;

  // Start unmapping PTs or pages.
  while (!state->error &&
      index < PAGING_PD_SIZE &&
      state->unmapped < state->requested)
  {
    paging_pd_entry_t *pd_entry = pd + state->linear.indices.pd_index;

    paging_pt_entry_t *pt = NULL;

    // If not present, we need to skip it.
    if (!pd_entry->info.present)
    {
      state->linear.pointer += PAGING_PT_4KPAGES * 0x1000;

      if (state->unmapped + PAGING_PT_4KPAGES > state->requested)
        state->unmapped = state->requested;
      else
        state->unmapped += PAGING_PT_4KPAGES;
    }
    else if (pd_entry->info.page_size == 1)
    {
      // If this is a 2 MB page, then we need to figure out if the number of 4
      // kB pages to unmap is equal to or greater than the number of 4 kB pages
      // that would be unmapped by unmapping this page.
      if ((state->requested - state->unmapped) >= PAGING_PT_4KPAGES)
      {
        pd_entry->as_page.present = 0;

        invlpg(state->linear.pointer);

        state->linear.pointer += PAGING_PT_4KPAGES * 0x1000;
        state->unmapped       += PAGING_PT_4KPAGES;
      }
      else
      {
        DEBUG_MESSAGE("tried to unmap into a 2 MB page");
        state->error = true;
        break;
      }
    }
    else
    {
      // Otherwise, we just need to look it up and unmap it.
      DEBUG_ASSERT(paging_phy_lin_map_get(&state->pageset->table_map,
            pd_entry->as_pointer.pt_physical << 12, (void **) &pt));

      __paging_unmap_pt(state, pt);
    }

    index++;
  }
}

static void __paging_unmap_pt(paging_unmap_state_t *state, paging_pt_entry_t *pt)
{
  // Establish the current index here in order to detect overflows.
  int index = state->linear.indices.pt_index;

  // Start unmapping pages.
  while (!state->error &&
      index < PAGING_PT_SIZE &&
      state->unmapped < state->requested)
  {
    paging_pt_entry_t *pt_entry = pt + state->linear.indices.pt_index;

    pt_entry->present = 0;

    invlpg(state->linear.pointer);

    state->linear.pointer += 0x1000;
    state->unmapped++;
    index++;
  }
}

/*
bool paging_get_flags(paging_pageset_t *pageset, void *linear_address,
    paging_flags_t *flags)
{
  *flags = 0;
  return false;
}

uint64_t paging_set_flags(paging_pageset_t *pageset, void *linear_address,
    uint64_t pages, paging_flags_t flags)
{
  return 0;
}
*/

paging_pageset_t *paging_get_current_pageset()
{
  return paging_current_pageset;
}

void paging_set_current_pageset(paging_pageset_t *pageset)
{
  // Write its PML4 to CR3.
  __asm__ volatile("mov %0, %%cr3" : : "r" (pageset->pml4_physical));

  // Set the current pageset.
  paging_current_pageset = pageset;
}
