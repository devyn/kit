/*******************************************************************************
 *
 * kit/kernel/memory.c
 * - kernel memory management
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdbool.h>

#include "memory.h"
#include "multiboot.h"
#include "rbtree.h"
#include "debug.h"

/* reserve space for an initial heap (512 KiB) */
static uint8_t memory_initial_heap[512 * 1024];

/* uint8_t in order to operate byte-by-byte. */
uint8_t *memory_stack_base    = memory_initial_heap;
uint8_t *memory_stack_pointer = memory_initial_heap;

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

void memory_initialize(const char *mmap_buffer, const uint32_t mmap_length)
{
  static const uint64_t PAGE_SIZE    = 0x1000   /* 4 kB */;
  static const uint64_t PREALLOCATED = 0x200000 /* 2 MB */;

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

  // Now print our tree.
  rbtree_t *tree = &memory_free_region_tree.tree;

  rbtree_node_t *node = rbtree_first_node(tree);
  while (node != NULL) {
    memory_free_region_node_t *region = (memory_free_region_node_t *) node;

    DEBUG_BEGIN_VALUES();
      DEBUG_HEX(region->physical_base);
      DEBUG_DEC(region->pages);
    DEBUG_END_VALUES();

    node = rbtree_node_next(node);
  }

  DEBUG_BEGIN_VALUES();
    DEBUG_DEC(memory_free_region_tree.total_free);
  DEBUG_END_VALUES();
}

void *memory_alloc(const size_t size)
{
  /**
   * TODO: Proper memory management and bounds checking.
   * As it is, this function can easily "allocate" memory
   * outside of the hilariously puny page that we have set
   * up for our kernel (first 2MB).
   */

  void *result = memory_stack_pointer;

  memory_stack_pointer += size;

  return result;
}

void memory_free(void *pointer) {
  // Do nothing.
  DEBUG_FORMAT("stub, pointer=%p", pointer);
  pointer = NULL;
}

void *memory_alloc_aligned(size_t size, size_t alignment)
{
  size_t pointer_value = (size_t) memory_stack_pointer;

  if (pointer_value % alignment != 0)
  {
    memory_stack_pointer += alignment - (pointer_value % alignment);
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
