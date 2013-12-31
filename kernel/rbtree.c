/*******************************************************************************
 *
 * kit/kernel/rbtree.c
 * - generic red-black tree implementation
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Take a look at http://en.wikipedia.org/wiki/Red%E2%80%93black_tree for more
 * information.
 *
 ******************************************************************************/

#include <stdbool.h>
#include <stddef.h>

#include "rbtree.h"

static inline rbtree_node_t *rbtree_node_grandparent(rbtree_node_t *node)
{
  if (node != NULL && node->parent != NULL)
    return node->parent->parent;
  else
    return NULL;
}

static inline rbtree_node_t *rbtree_node_uncle(rbtree_node_t *node)
{
  rbtree_node_t *grandparent = rbtree_node_grandparent(node);

  if (grandparent != NULL)
  {
    if (node->parent == grandparent->left)
      return grandparent->right;
    else
      return grandparent->left;
  }
  else
  {
    return NULL;
  }
}

static inline void rbtree_rotate_left(rbtree_t *tree, rbtree_node_t *node)
{
  rbtree_node_t *saved_right_left = node->right->left;

  node->right->left = node;
  node->right->parent = node->parent;

  if (node->parent == NULL)
    tree->root = node->right;
  else if (node == node->parent->left)
    node->parent->left = node->right;
  else
    node->parent->right = node->right;

  node->parent = node->right;
  node->right  = saved_right_left;

  saved_right_left->parent = node;
}

static inline void rbtree_rotate_right(rbtree_t *tree, rbtree_node_t *node)
{
  rbtree_node_t *saved_left_right = node->left->right;

  node->left->right = node;
  node->left->parent = node->parent;

  if (node->parent == NULL)
    tree->root = node->left;
  else if (node == node->parent->left)
    node->parent->left = node->left;
  else
    node->parent->right = node->left;

  node->parent = node->left;
  node->left   = saved_left_right;
}

void rbtree_balance_insert(rbtree_t *tree, rbtree_node_t *node)
{
  /**
   * Properties of red-black trees:
   *
   * 1. A node is either red or black.
   * 2. The root is black.
   * 3. All leaves (NULL) are black.
   * 4. Every red node must have two black child nodes.
   * 5. The number of black nodes on the path from a given node to any of its
   *    descendant leaves must be the same.
   */

  // The new node should initially be red.
  node->color = RBTREE_COLOR_RED;

  // Loop for cases 1-3
  while (true)
  {
    // At the beginning of this loop, the node is always red.

    if (node->parent == NULL)
    {
      /**
       * Case 1: the node is the root of the tree.
       */
      node->color = RBTREE_COLOR_BLACK;
      tree->root  = node;
      return;
    }

    if (node->parent->color == RBTREE_COLOR_BLACK)
    {
      /**
       * Case 2: the node's parent is black, thus no balancing is required.
       *
       * Appending a red node does not violate any of the properties.
       */
      return;
    }

    rbtree_node_t *uncle = rbtree_node_uncle(node);

    if ((uncle != NULL) && (uncle->color == RBTREE_COLOR_RED))
    {
      /**
       * Case 3: the node's parent and uncle are both red. Because this node is
       * red too, property 4 is violated.
       *
       * In order to correct for property 4, the parent and uncle may be
       * repainted black, and the grandparent red. However, the grandparent now
       * either violates property 2 or 4, so the grandparent must now be checked
       * against cases 1-3 (continuing the loop).
       */

      rbtree_node_t *grandparent = rbtree_node_grandparent(node);

      node->parent->color = RBTREE_COLOR_BLACK;
      uncle->color        = RBTREE_COLOR_BLACK;
      grandparent->color  = RBTREE_COLOR_RED;

      node = grandparent;
    }
    else
    {
      break;
    }
  }

  /**
   * Case 4: the node's parent is red, and the node's uncle is black. The node
   * is red, so property 4 is violated.
   *
   * If the node is on the opposite side of the parent as the parent is of the
   * grandparent, then we should first rotate the parent with the node:
   *
   *     Gb              Gb
   *    / \             / \
   *   Pr  Ub   -->    Nr  Ub
   *    \             /
   *     Nr          Pr
   *
   * If this is the case, then node is set to the parent.
   *
   * We can then rotate around the grandparent:
   *
   *     Gb              Pb
   *    / \             / \
   *   Pr  Ub   -->    Nr  Gr
   *  /                     \
   * Nr                      Ub
   *
   * All properties are now satistfied.
   */

  rbtree_node_t *grandparent = rbtree_node_grandparent(node);

  if (node->parent == grandparent->left && node == node->parent->right)
  {
    rbtree_rotate_left(tree, node->parent);

    node = node->left;
  }
  else if (node->parent == grandparent->right && node == node->parent->left)
  {
    rbtree_rotate_right(tree, node->parent);

    node = node->right;
  }

  node->parent->color = RBTREE_COLOR_BLACK;
  grandparent->color  = RBTREE_COLOR_RED;

  if (node == node->parent->left)
    rbtree_rotate_right(tree, grandparent);
  else
    rbtree_rotate_left(tree, grandparent);
}

const rbtree_node_t *rbtree_first_node(const rbtree_t *tree)
{
  const rbtree_node_t *node = tree->root;

  if (!node)
    return NULL;

  while (node->left != NULL)
    node = node->left;

  return node;
}

const rbtree_node_t *rbtree_node_next(const rbtree_node_t *node)
{
  if (node->right != NULL)
  {
    // Go right and then left as far as possible.
    node = node->right;

    while (node->left != NULL)
      node = node->left;

    return node;
  }
  else
  {
    /**
     * No children on the right, so we must go up until we find a node that is a
     * left-hand child of its parent (therefore the parent's key is greater).
     * The parent of that node is the next node.
     */
    while (node->parent != NULL && node == node->parent->right)
      node = node->parent;

    return node->parent;
  }
}
