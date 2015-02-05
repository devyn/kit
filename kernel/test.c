/*******************************************************************************
 *
 * kit/kernel/test.c
 * - runtime unit tests
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#define TEST_C

#include <stdint.h>

#include "terminal.h"
#include "memory.h"
#include "interrupt.h"
#include "rbtree.h"
#include "paging.h"
#include "archive.h"
#include "elf.h"

#include "debug.h"
#include "x86_64.h"

#include "test.h"

bool test_run(const test_unit_t *unit)
{
  terminal_setcolor(COLOR_LIGHT_CYAN, COLOR_BLACK);

  terminal_writestring("\n[TEST] ");
  terminal_writestring(unit->name);

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_writechar('\n');

  bool result = unit->run();

  if (result)
  {
    terminal_setcolor(COLOR_LIGHT_GREEN, COLOR_BLACK);
    terminal_writestring("[PASS] ");
  }
  else
  {
    terminal_setcolor(COLOR_LIGHT_RED, COLOR_BLACK);
    terminal_writestring("[FAIL] ");
  }

  terminal_writestring(unit->name);

  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK);
  terminal_writechar('\n');

  return result;
}

#define HEADING(heading)                           \
  terminal_setcolor(COLOR_WHITE, COLOR_BLACK);     \
  terminal_writestring((heading));                 \
  terminal_setcolor(COLOR_LIGHT_GREY, COLOR_BLACK)

bool test_memory_c()
{
  HEADING("memory_alloc(512) returns a non-NULL pointer\n");

  char *ptr = memory_alloc(512);

  if (ptr == NULL)
  {
    terminal_writestring("  E: returned NULL");
    return false;
  }
  else
  {
    terminal_printf("  - returned pointer: %p\n", ptr);
  }

  HEADING("memory_set() sets memory\n");

  int64_t i;

  terminal_writestring("  - writing varied data to allocated memory\n");
  
  for (i = 0; i < 512; i++) ptr[i] = i;

  terminal_writestring("  - invoking memory_set()\n");
  memory_set(ptr, 0, 512);

  terminal_writestring("  - verifying that the memory has been set\n");

  for (i = 0; i < 512; i++)
  {
    if (ptr[i] != 0)
    {
      terminal_printf("  E: memory not set at byte %ld; value is %#hhx\n",
          i, ptr[i]);
      return false;
    }
  }

  HEADING("memory_alloc_aligned(1, 1024) returns an aligned pointer\n");

  char *aligned_ptr = memory_alloc_aligned(1, 1024);

  terminal_printf("  - returned pointer: %p\n", aligned_ptr);

  if ((uint64_t) aligned_ptr % 1024 > 0)
  {
    terminal_writestring("  E: aligned pointer does not divide by 1024\n");
    return false;
  }

  HEADING("memory_free_region_acquire(pages=16) returns 16 fresh pages\n");

  uint64_t physical_base;
  uint64_t pages;

  uint64_t total_free_1 = memory_get_total_free();

  pages = memory_free_region_acquire(16, &physical_base);

  uint64_t total_free_2 = memory_get_total_free();

  if (pages == 16)
  {
    terminal_writestring("  - pages = 16\n");
  }
  else
  {
    terminal_printf("  E: pages = %lu\n", pages);
    return false;
  }

  terminal_printf("  - physical_base = %#lx\n", physical_base);

  if (physical_base >= 0x200000)
  {
    terminal_writestring("  - fresh (>= 0x200000)\n");
  }
  else
  {
    terminal_writestring("  E: not fresh (< 0x200000)\n");
    return false;
  }

  if (physical_base % 4096 == 0)
  {
    terminal_writestring("  - aligned to 4 kB\n");
  }
  else
  {
    terminal_writestring("  E: not aligned to 4 kB\n");
    return false;
  }

  if (total_free_1 - 16 == total_free_2)
  {
    terminal_writestring("  - 16 pages have been subtracted from total_free\n");
  }
  else
  {
    terminal_printf("  E: total_free difference = %li, should be 16\n",
        total_free_1 - total_free_2);
    return false;
  }

  HEADING("memory_free_region_release() reclaims 16 pages\n");

  memory_free_region_release(physical_base, pages);

  uint64_t total_free_3 = memory_get_total_free();

  if (total_free_1 == total_free_3)
  {
    terminal_writestring("  - total_free_1 == total_free_3\n");
  }
  else
  {
    terminal_printf(
        "  E: total_free_1 != total_free_3\n"
        "     total_free_1 = %lu\n"
        "     total_free_3 = %lu\n",
        total_free_1, total_free_3);
    return false;
  }

  HEADING("memory_free_region_acquire(pages=16) selects the same 16 pages\n");

  uint64_t new_physical_base;
  uint64_t new_pages;

  new_pages = memory_free_region_acquire(16, &new_physical_base);

  if (new_pages != 16)
  {
    terminal_writestring("  E: pages == 16\n");
    return false;
  }

  if (physical_base == new_physical_base)
  {
    terminal_writestring("  - physical_base == new_physical_base\n");
  }
  else
  {
    terminal_printf("  E: new_physical_base = %#lx\n", new_physical_base);
  }

  return true;
}

bool test_interrupt_c()
{
  HEADING("handles two software interrupts and comes back\n");

  terminal_writestring("  - sending interrupt 0x1f\n");
  __asm__ __volatile__("int $0x1f");

  terminal_writestring("  - sending interrupt 0x3\n");
  __asm__ __volatile__("int $0x3");

  return true;
}

typedef struct test_rbtree
{
  rbtree_t tree;
} test_rbtree_t;

typedef struct test_rbtree_node
{
  rbtree_node_t node;
  int           key;
  char          value;
} test_rbtree_node_t;

static test_rbtree_node_t
  *test_rbtree_search(const test_rbtree_t *tree, int key)
{
  test_rbtree_node_t *node = (test_rbtree_node_t *) tree->tree.root;

  while (node != NULL)
  {
    if (node->key < key)
      node = (test_rbtree_node_t *) node->node.right;
    else if (node->key > key)
      node = (test_rbtree_node_t *) node->node.left;
    else
      return node;
  }

  return NULL;
}

static
test_rbtree_node_t *test_rbtree_insert(test_rbtree_t *tree, int key, char value)
{
  test_rbtree_node_t *parent = (test_rbtree_node_t *) tree->tree.root;

  test_rbtree_node_t *node = memory_alloc(sizeof(test_rbtree_node_t));
  memory_set(node, 0, sizeof(test_rbtree_node_t));

  node->key   = key;
  node->value = value;

  if (parent == NULL)
  {
    tree->tree.root   = (rbtree_node_t *) node;
    node->node.parent = NULL;
  }
  else
  {
    while (true)
    {
      if (parent->key < node->key && parent->node.right != NULL)
        parent = (test_rbtree_node_t *) parent->node.right;
      else if (parent->key > node->key && parent->node.left != NULL)
        parent = (test_rbtree_node_t *) parent->node.left;
      else
        break;
    }

    node->node.parent = (rbtree_node_t *) parent;

    if (parent->key < node->key)
      parent->node.right = (rbtree_node_t *) node;
    else if (parent->key > node->key)
      parent->node.left  = (rbtree_node_t *) node;
    else
    {
      memory_free(node);
      parent->value = value;
      return parent;
    }

    rbtree_balance_insert(&tree->tree, &node->node);
  }

  return node;
}

static void __test_rbtree_inspect_1(const test_rbtree_node_t *node, int indent,
  const char *identifier)
{
  for (int i = 0; i <= indent; i++)
  {
    terminal_writechar(' ');
    terminal_writechar(' ');
  }

  if (node->node.color == RBTREE_COLOR_RED)
    terminal_writechar('R');
  else
    terminal_writechar('B');

  terminal_writechar(node->value);
  terminal_writechar(' ');
  terminal_writestring(identifier);
  terminal_writechar('\n');

  if (node->node.left != NULL)
    __test_rbtree_inspect_1((test_rbtree_node_t *) node->node.left,
      indent + 1, "left");

  if (node->node.right != NULL)
    __test_rbtree_inspect_1((test_rbtree_node_t *) node->node.right,
      indent + 1, "right");
}

static void test_rbtree_inspect(const test_rbtree_t *tree)
{
  const test_rbtree_node_t *node = (test_rbtree_node_t *) tree->tree.root;

  if (node != NULL)
    __test_rbtree_inspect_1(node, 0, "root");
}

static bool test_rbtree_is_valid(test_rbtree_t *tree)
{
  rbtree_node_t *node = tree->tree.root;

  // Property 2, 3: the root is black and all leaves are black.
  if (node == NULL)
    return true;

  // Property 2: the root is black.
  if (node->color != RBTREE_COLOR_BLACK)
  {
    terminal_writestring("  ! property 2 violated\n");
    return false;
  }

  node = rbtree_first_node(&tree->tree);

  int max_black_nodes = 0;

  while (node != NULL)
  {
    // Property 4: every red node must have two black child nodes.
    if (node->color == RBTREE_COLOR_RED)
    {
      if (node->left != NULL && node->left->color != RBTREE_COLOR_BLACK)
      {
        terminal_writestring("  ! property 4 violated\n");
        return false;
      }
      if (node->right != NULL && node->right->color != RBTREE_COLOR_BLACK)
      {
        terminal_writestring("  ! property 4 violated\n");
        return false;
      }
    }

    // Property 5: the number of black nodes on the path from a given node to any
    // of its descendant leaves must be the same.
    if (node->left == NULL || node->right == NULL)
    {
      int black_nodes = 0;

      const rbtree_node_t *test_node = node;

      //terminal_writestring("  - ");
      while (test_node != NULL)
      {
        if (test_node->color == RBTREE_COLOR_BLACK)
          black_nodes++;

        //terminal_writechar(((test_rbtree_node_t *) test_node)->value);

        test_node = test_node->parent;
      }
      //terminal_writestring(" = ");
      //terminal_writeuint64(black_nodes, 10);
      //terminal_writestring(" black nodes");
      //terminal_writechar('\n');

      if (max_black_nodes == 0)
      {
        max_black_nodes = black_nodes;
      }
      else if (max_black_nodes != black_nodes)
      {
        test_rbtree_inspect(tree);

        terminal_printf(
            "  E: property 5 violated\n"
            "     max black nodes: %i\n"
            "     black nodes:     %i\n"
            "     in:              %c\n",
            max_black_nodes, black_nodes,
            ((test_rbtree_node_t *) node)->value);
        return false;
      }
    }

    node = rbtree_node_next(node);
  }

  return true;
}

bool test_rbtree_c()
{
  HEADING("all keys are present and searchable after insertion\n");

  test_rbtree_t tree;
  tree.tree.root = NULL;

  int keys_to_insert[10] = {123980, 12983, 38288, 493282, 290810,
    290811, 290812, 290813, 290814, 290815};

  for (int i = 0; i < 10; i++)
  {
    test_rbtree_insert(&tree, keys_to_insert[i], 'a' + i);
  }

  for (int i = 0; i < 10; i++)
  {
    if (test_rbtree_search(&tree, keys_to_insert[i]) == NULL)
      return false;
  }

  HEADING("the tree produced is valid and thus O(log n)\n");

  if (!test_rbtree_is_valid(&tree))
    return false;

  HEADING("the tree is valid and contains remaining values "
          "after deleting each value\n");

  for (int i = 0; i < 10; i++)
  {
    test_rbtree_node_t *node = test_rbtree_search(&tree, keys_to_insert[i]);

    rbtree_delete((rbtree_t *) &tree, (rbtree_node_t *) node);

    if (!test_rbtree_is_valid(&tree))
      return false;

    for (int j = i + 1; j < 10; j++)
    {
      if (test_rbtree_search(&tree, keys_to_insert[j]) == NULL)
      {
        return false;
      }
    }
  }

  return true;
}

bool test_paging_c()
{
  HEADING("resolve linear address of the kernel pageset in the kernel pageset\n");

  void     *f_linear_address   = (void *) &paging_kernel_pageset;
  uint64_t  f_physical_address = 0;

  terminal_printf("  - linear address: %p\n", f_linear_address);

  if (paging_resolve_linear_address(&paging_kernel_pageset, f_linear_address,
        &f_physical_address))
  {
    terminal_printf("  - physical address: %#lx\n", f_physical_address);

    if ((((uint64_t) f_linear_address) & 0xffffff) != f_physical_address)
    {
      terminal_writestring("  E: lin & 0xffffff != phy\n");
      return false;
    }
  }
  else
  {
    terminal_writestring("  E: failed to resolve address\n");
  }

  HEADING("create pageset\n");

  paging_pageset_t pageset;

  if (paging_create_pageset(&pageset))
  {
    terminal_writestring("  - ok\n");
  }
  else
  {
    terminal_writestring("  E: creation failed (out of memory?)\n");
    return false;
  }

  HEADING("map a single page\n");

  uint64_t physical_base;

  DEBUG_ASSERT(memory_free_region_acquire(1, &physical_base) == 1);

  terminal_printf("  - physical base: %#lx\n", physical_base);

  char *pointer_1 = (void *) 0xdeadb000;

  terminal_printf("  - linear base: %p\n", pointer_1);

  uint64_t mapped_1 = paging_map(&pageset, pointer_1, physical_base, 1, 0);

  if (mapped_1 == 1)
  {
    terminal_writestring("  - ok, got one page\n");
  }
  else
  {
    terminal_printf("  E: requested 1 page, but mapped %lu pages.\n", mapped_1);
    return false;
  }

  HEADING("resolve linear address we just mapped\n");

  uint64_t physical_1;

  if (paging_resolve_linear_address(&pageset, pointer_1, &physical_1))
  {
    terminal_printf("  - physical address: %#lx\n", physical_1);

    if (physical_1 != physical_base)
    {
      terminal_writestring("  E: wrong physical address\n");
      return false;
    }
  }
  else
  {
    terminal_writestring("  E: failed to resolve address\n");
    return false;
  }

  HEADING("switch to the created pageset\n");

  paging_set_current_pageset(&pageset);

  if (paging_get_current_pageset() == &pageset)
  {
    terminal_writestring("  - ok\n");
  }
  else if (paging_get_current_pageset() == &paging_kernel_pageset)
  {
    terminal_writestring("  E: current pageset is still kernel pageset\n");
    return false;
  }
  else
  {
    terminal_printf("  E: current pageset is unknown: %p\n",
        (void *) paging_get_current_pageset());
    return false;
  }

  HEADING("make sure we can access the mapped memory\n");

  char buf[9] = "in a pan";

  memory_copy(buf, pointer_1 + 0xeef, 9);

  terminal_printf("  - 0xdeadbeef = %s\n", (char *) 0xdeadbeef);

  HEADING("unmap the page\n");

  uint64_t unmapped_1 = paging_unmap(&pageset, pointer_1, 1);

  if (unmapped_1 == 1)
  {
    terminal_writestring("  - ok, unmapped one page\n");
  }
  else
  {
    terminal_printf("  E: requested 1 page, but unmapped %lu pages.\n",
        unmapped_1);
    return false;
  }

  HEADING("switch back to the kernel pageset and then destroy this one\n");

  paging_set_current_pageset(&paging_kernel_pageset);

  DEBUG_ASSERT(paging_get_current_pageset() == &paging_kernel_pageset);

  if (paging_destroy_pageset(&pageset))
  {
    terminal_writestring("  - ok\n");
  }
  else
  {
    terminal_writestring("  E: destruction failed\n");
    return false;
  }

  // Don't forget this!
  memory_free_region_release(physical_base, 1);

  return true;
}

bool test_elf_c()
{
  HEADING("prerequisite: usertest.bin exists\n");

  char     *buffer;
  uint64_t  length;

  if (archive_get(archive_system, "usertest.bin", &buffer, &length))
  {
    terminal_writestring("  - ok\n");
  }
  else
  {
    terminal_writestring("  E: usertest.bin not found\n");
    return false;
  }

  HEADING("elf_verify() on usertest.bin returns true\n");

  elf_header_64_t *elf = (elf_header_64_t *) buffer;

  if (elf_verify(elf))
  {
    terminal_writestring("  - ok, compatible with kit\n");
  }
  else
  {
    terminal_writestring("  E: incompatible\n");
    return false;
  }

  HEADING("program headers are present and make sense\n");

  if (elf->e_phnum == 0)
  {
    terminal_writestring("  E: no program headers\n");
    return false;
  }

  elf_program_header_iterator_t iterator = elf_program_header_iterate(elf);

  elf_program_header_t *ph;

  while ((ph = elf_program_header_next(&iterator)) != NULL)
  {
    elf_program_header_print(ph);
  }

  HEADING("entry point is within an executable segment\n");

  bool found = false;

  iterator = elf_program_header_iterate(elf);

  terminal_printf("  - entry point: %#lx\n", elf->e_entry);

  while ((ph = elf_program_header_next(&iterator)) != NULL)
  {
    if (elf->e_entry >= ph->p_vaddr &&
        elf->e_entry <= ph->p_vaddr + ph->p_memsz)
    {
      if (ph->p_flags & ELF_P_FLAG_EXECUTE)
      {
        found = true;
        break;
      }
      else
      {
        terminal_writestring("  E: matching segment is not executable\n");
        return false;
      }
    }
  }

  if (!found)
  {
    terminal_writestring("  E: no matching segment\n");
    return false;
  }

  return true;
}

bool test_all()
{
  for (size_t i = 0; i < TEST_UNITS_SIZE; i++)
  {
    if (!test_run(&test_units[i]))
    {
      terminal_printf("\n %lu/%lu tests passed.\n", i, TEST_UNITS_SIZE);
      return false;
    }
  }

  terminal_printf("\n %lu/%lu tests passed.\n",
      TEST_UNITS_SIZE, TEST_UNITS_SIZE);

  return true;
}
