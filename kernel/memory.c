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

/**
 * Not actually a uint8_t; just a location, and uint8_t is convenient
 * because it matches.
 */
extern uint8_t _kernel_end;

/* uint8_t in order to operate byte-by-byte. */
uint8_t *memory_stack_base    = &_kernel_end;
uint8_t *memory_stack_pointer = &_kernel_end;

/* free region tree */

typedef struct memory_free_region_tree {
  rbtree_t tree;

  uint64_t total_free;
} memory_free_region_tree_t;

typedef struct memory_free_region_node {
  rbtree_node_t node;

  uint64_t physical_base;
  uint64_t length;
} memory_free_region_node_t;

memory_free_region_tree_t memory_free_region_tree = {{NULL}, 0};

static void memory_free_region_create(const uint64_t physical_base,
                                      const uint64_t length)
{
  // Find the center node of the tree
  memory_free_region_node_t *parent =
    (memory_free_region_node_t *) memory_free_region_tree.tree.root;

  // Create a new node
  memory_free_region_node_t *node =
    memory_alloc(sizeof(memory_free_region_node_t));

  memory_set(node, 0, sizeof(memory_free_region_node_t));

  // Set attributes
  node->physical_base = physical_base;
  node->length        = length;

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
      if (parent->length <= node->length && parent->node.right != NULL)
        parent = (memory_free_region_node_t *) parent->node.right;
      else if (parent->length > node->length && parent->node.left != NULL)
        parent = (memory_free_region_node_t *) parent->node.left;
      else
        break;
    }

    // Now attach node -> parent
    node->node.parent = (rbtree_node_t *) parent;

    // And parent -> node, based on which side it should go on
    if (parent->length <= node->length)
      parent->node.right = (rbtree_node_t *) node;
    else
      parent->node.left  = (rbtree_node_t *) node;

    // And then balance it
    rbtree_balance_insert(&memory_free_region_tree.tree, &node->node);
  }

  // Finally, we can add this amount to the total free size of the tree
  memory_free_region_tree.total_free += length;
}

void memory_initialize(const char *mmap_buffer, const uint32_t mmap_length)
{
  static const uint64_t PREALLOCATED = 0x200000 /* 2 MB */;

  const char *current_mmap = mmap_buffer;

  while (current_mmap < mmap_buffer + mmap_length) {
    multiboot_memory_map_t *entry = (multiboot_memory_map_t *) current_mmap;

    // If the region contains memory beyond our preallocated 2 MB, and the
    // memory is available for use, then insert it into the free region tree.
    if (entry->addr + entry->len > PREALLOCATED &&
        entry->type == MULTIBOOT_MEMORY_AVAILABLE)
    {
      if (entry->addr < PREALLOCATED)
      {
        // If the block starts before our preallocated region, then we have to
        // crop it so that it doesn't intersect that.
        uint64_t diff = PREALLOCATED - entry->addr;

        memory_free_region_create(PREALLOCATED, entry->len - diff);
      }
      else
      {
        // Otherwise we can just go ahead and create it.
        memory_free_region_create(entry->addr, entry->len);
      }
    }

    current_mmap += entry->size + 4;
  }

  // Now print our tree.
  rbtree_t *tree = &memory_free_region_tree.tree;

  const rbtree_node_t *node = rbtree_first_node(tree);
  while (node != NULL) {
    memory_free_region_node_t *region = (memory_free_region_node_t *) node;

    DEBUG_BEGIN_VALUES();
      DEBUG_HEX(region->physical_base);
      DEBUG_DEC(region->length);
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
  DEBUG_MESSAGE_HEX("stub: memory_free", pointer);
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
