/*******************************************************************************
 *
 * kit/kernel/memory.c
 * - kernel memory management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdbool.h>

#include "memory.h"
#include "paging.h"
#include "multiboot.h"
#include "rbtree.h"
#include "debug.h"

/* reserve space for an initial heap (128 KiB) */
static uint8_t memory_initial_heap[128 * 1024];

/* uint8_t in order to operate byte-by-byte. */
uint8_t *memory_heap_start = memory_initial_heap;
uint8_t *memory_heap_end   = memory_initial_heap + sizeof(memory_initial_heap);

uint64_t memory_heap_length = 0;

bool memory_large_heap_enabled = false;
bool memory_grow_enabled       = false;

/* free region tree */

typedef struct memory_free_region_tree {
  rbtree_t tree;

  uint64_t total_free;
} memory_free_region_tree_t;

typedef struct memory_free_region_node {
  rbtree_node_t node;

  uint64_t physical_base;
  uint64_t pages;
} memory_free_region_node_t;

static memory_free_region_tree_t memory_free_region_tree = {{NULL}, 0};

void oaisoi(UNUSED multiboot_info_t lalkak) {
}

void memory_initialize(const char *mmap_buffer, const uint32_t mmap_length)
{
  static const uint64_t PAGE_SIZE    = 0x1000   /* 4 KiB */;
  static const uint64_t PREALLOCATED = 0x400000 /* 4 MiB */;

  const char *current_mmap = mmap_buffer;

  while (current_mmap < mmap_buffer + mmap_length) {
    multiboot_memory_map_t *entry = (multiboot_memory_map_t *) current_mmap;

    current_mmap += entry->size + 4;

    // Ensure the entry is marked as available.
    //
    // Also ensure that the length is going to be large enough after we shear
    // it to align to 4 kB page boundaries.
    if (entry->type == MULTIBOOT_MEMORY_AVAILABLE &&
        entry->len >= PAGE_SIZE + (entry->addr % PAGE_SIZE))
    {
      // Align to 4 kB page boundaries.
      uint64_t physical_base = entry->addr % PAGE_SIZE != 0 ?
                               ((entry->addr / PAGE_SIZE) + 1) * PAGE_SIZE :
                               entry->addr;

      uint64_t pages = (entry->len - (entry->addr % PAGE_SIZE)) / PAGE_SIZE;

      // If the base starts before our preallocated region, remove the pages
      // before that (and make sure we still have pages left).
      if (physical_base < PREALLOCATED)
      {
        uint64_t diff = (PREALLOCATED - physical_base) / PAGE_SIZE;

        if (diff < pages)
        {
          physical_base += diff * PAGE_SIZE;
          pages         -= diff;
        }
        else
        {
          continue; // skip this entry
        }
      }

      // Create the region.
      memory_free_region_release(physical_base, pages);
    }
  }

  memory_heap_length = 0;
}

#define MEMORY_LARGE_HEAP_START 0xffffffff81000000
#define MEMORY_BUFZONE_SIZE     (4 * 4096)

void memory_enable_large_heap()
{
  if (!memory_large_heap_enabled)
  {
    uint64_t physical_base;

    DEBUG_ASSERT(memory_free_region_acquire(
          MEMORY_BUFZONE_SIZE/4096, &physical_base) ==
        MEMORY_BUFZONE_SIZE/4096);

    paging_map(&paging_kernel_pageset, (void *) MEMORY_LARGE_HEAP_START,
        physical_base, MEMORY_BUFZONE_SIZE/4096, 0);

    memory_heap_start  = (uint8_t *) MEMORY_LARGE_HEAP_START;
    memory_heap_end    = memory_heap_start + MEMORY_BUFZONE_SIZE;
    memory_heap_length = 0;

    memory_large_heap_enabled = true;
    memory_grow_enabled       = true;
  }
}

void *memory_alloc(const size_t size)
{
  void *result = memory_heap_start + memory_heap_length;

  memory_heap_length += size;

  // If we don't have enough memory, loop through the following:
  while (memory_heap_start + memory_heap_length >
         memory_heap_end   - (memory_grow_enabled ? MEMORY_BUFZONE_SIZE : 0))
  {
    if (memory_large_heap_enabled && memory_grow_enabled)
    {
      // The large heap is enabled and we're allowed to grow it.
      uint64_t physical_base;

      uint64_t grow  = memory_heap_length + MEMORY_BUFZONE_SIZE -
        ((uint64_t) memory_heap_end -
         (uint64_t) memory_heap_start);

      uint64_t pages = grow / 4096;

      if (grow % 4096 != 0) pages++;

      if (!memory_free_region_acquire(pages, &physical_base))
      {
        DEBUG_MESSAGE("out of memory");
        return NULL;
      }

      // Map the pages we just got, being careful to avoid ending up in a loop.
      memory_grow_enabled = false;
      paging_map(&paging_kernel_pageset, (void *) memory_heap_end,
          physical_base, pages, 0);
      memory_grow_enabled = true;

      // Update memory_heap_end.
      memory_heap_end += pages * 4096;
    }
    else
    {
      if (!memory_large_heap_enabled)
      {
        // We don't have the large heap enabled yet, so we can't try to allocate
        // more.
        DEBUG_FORMAT("ran out of initial heap (%lu + %lu)",
            memory_heap_length, size);
        return NULL;
      }
      else
      {
        // We aren't allowed to grow the heap right now.
        DEBUG_MESSAGE("tried to grow the heap recursively");
        return NULL;
      }
    }
  }

#ifdef MEMORY_LOG_ALLOC
  DEBUG_FORMAT("(%lu) => %p", size, result);
#endif

  // All okay.
  return result;
}

void memory_free(void *pointer) {
  // Do nothing.
#ifdef MEMORY_LOG_FREE
  DEBUG_FORMAT("(%p)", pointer);
#endif
  pointer = NULL;
}

void *memory_alloc_aligned(size_t size, size_t alignment)
{
  size_t pointer_value = (size_t) (memory_heap_start + memory_heap_length);

  if (pointer_value % alignment != 0)
  {
    memory_heap_length += alignment - (pointer_value % alignment);
  }

  return memory_alloc(size);
}

uint64_t memory_get_total_free()
{
  return memory_free_region_tree.total_free;
}

static void memory_free_region_insert(memory_free_region_node_t *node)
{
  // Physical base must be aligned to boundary
  DEBUG_ASSERT(node->physical_base % 4096 == 0);

  // Node must have more than zero pages
  DEBUG_ASSERT(node->pages > 0);

  // Find the center node of the tree
  memory_free_region_node_t *parent =
    (memory_free_region_node_t *) memory_free_region_tree.tree.root;

  // If there is no parent, this might as well just be the root
  if (parent == NULL)
  {
    memory_free_region_tree.tree.root = (rbtree_node_t *) node;
    node->node.parent = NULL;
  }
  else
  {
    // Otherwise we need to look for where to insert this, by length
    while (true)
    {
      if (parent->pages <= node->pages && parent->node.right != NULL)
        parent = (memory_free_region_node_t *) parent->node.right;
      else if (parent->pages > node->pages && parent->node.left != NULL)
        parent = (memory_free_region_node_t *) parent->node.left;
      else
        break;
    }

    // Now attach node -> parent
    node->node.parent = (rbtree_node_t *) parent;

    // And parent -> node, based on which side it should go on
    if (parent->pages <= node->pages)
      parent->node.right = (rbtree_node_t *) node;
    else
      parent->node.left  = (rbtree_node_t *) node;

    // And then balance it
    rbtree_balance_insert(&memory_free_region_tree.tree, &node->node);
  }

  // Finally, we can add this amount to the total free size of the tree
  memory_free_region_tree.total_free += node->pages;
}

uint64_t memory_free_region_acquire(const uint64_t pages,
                                    uint64_t *physical_base)
{

  // Find the center node of the tree.
  memory_free_region_node_t *node =
    (memory_free_region_node_t *) memory_free_region_tree.tree.root;

  // Ensure that the tree is not empty (out of memory).
  if (node == NULL) return 0;

  // Search for the nearest fitting node, ideally the same size or larger than
  // 'bytes'.
  //
  // First, go left until we find a node with pages equal to or fewer than what
  // we need.
  while (node->pages > pages && node->node.left != NULL)
  {
    node = (memory_free_region_node_t *) node->node.left;
  }

  // Then, iterate through the tree until we find a node of larger or equal
  // pages than we need.
  rbtree_node_t *next;

  while (node->pages < pages &&
         (next = rbtree_node_next(&node->node)) != NULL)
  {
    node = (memory_free_region_node_t *) next;
  }

  // Then, delete that node from the tree and update the tree's total_free
  // attribute.
  rbtree_delete(&memory_free_region_tree.tree, &node->node);

  memory_free_region_tree.total_free -= node->pages;

  // If the node has more pages than what we requested, just take a bunch
  // off the end and re-insert. Otherwise, return the entire node's space and
  // discard it.
  if (node->pages > pages)
  {
    // Reset rbtree attributes.
    memory_set(&node->node, 0, sizeof(rbtree_node_t));

    // Subtract from pages and re-insert,
    node->pages -= pages;
    memory_free_region_insert(node);

    // and then calculate the resulting physical base of the new region,
    *physical_base = node->physical_base + (node->pages << 12);

    // which is exactly the size requested.
    return pages;
  }
  else
  {
    // Just return the node's attributes and free it.
    uint64_t actual_pages = node->pages;
    *physical_base        = node->physical_base;

    memory_free(node);

    return actual_pages;
  }
}

void memory_free_region_release(const uint64_t physical_base,
                                const uint64_t pages)
{
  // Create a new node
  memory_free_region_node_t *node =
    memory_alloc(sizeof(memory_free_region_node_t));

  memory_set(node, 0, sizeof(memory_free_region_node_t));

  // Set attributes
  node->physical_base = physical_base;
  node->pages         = pages;

  // Insert
  memory_free_region_insert(node);
}
