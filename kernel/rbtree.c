/*******************************************************************************
 *
 * kit/kernel/rbtree.c
 * - generic red-black tree implementation
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
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
#include "debug.h"

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

static inline rbtree_node_t *rbtree_node_sibling(rbtree_node_t *node)
{
  if (node->parent != NULL)
  {
    if (node->parent->left == node)
      return node->parent->right;
    else
      return node->parent->left;
  }
  else
  {
    return NULL;
  }
}

/**
 * Intended for replacing in preparation for deletion; doesn't write to old at
 * all.
 */
static inline void rbtree_replace_node(rbtree_t *tree,
                                       rbtree_node_t *new,
                                       const rbtree_node_t *old)
{
  // If new is just NULL, then we can ignore it; otherwise:
  if (new != NULL)
  {
    // First detach new from its old parent:
    if (new->parent != NULL)
    {
      if (new->parent->left == new)
        new->parent->left = NULL;
      else
        new->parent->right = NULL;
    }

    // Then set new parent to old parent.
    new->parent = old->parent;
  }

  // Really simple if old parent is NULL: this is the root
  if (old->parent == NULL)
  {
    tree->root = new;
  }
  else
  {
    // Otherwise we need to figure out where old was attached and then attach
    // new there instead.
    if (old->parent->left == old)
      old->parent->left = new;
    else
      old->parent->right = new;
  }
}

static inline void rbtree_rotate_left(rbtree_t *tree, rbtree_node_t *node)
{
  DEBUG_ASSERT(node->right != NULL);

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

  if (saved_right_left != NULL)
    saved_right_left->parent = node;
}

static inline void rbtree_rotate_right(rbtree_t *tree, rbtree_node_t *node)
{
  DEBUG_ASSERT(node->left != NULL);

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

  if (saved_left_right != NULL)
    saved_left_right->parent = node;
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

void rbtree_delete(rbtree_t *tree, rbtree_node_t *node) {
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

  /**
   * If the node has two non-null children, we'll just replace it with the
   * minimum value on the right side and call it a day.
   */
  if (node->left != NULL && node->right != NULL)
  {
    rbtree_node_t *new = node->right;

    while (new->left != NULL)
    {
      new = new->left;
    }

    rbtree_replace_node(tree, new, node);
    return;
  }

  /**
   * Otherwise, it has either zero or one non-null child, so get that if it's
   * present.
   */
  rbtree_node_t *child = node->left == NULL ? node->right : node->left;

  // If the node is red, we can just replace it with its child and quit.
  // Property 5 is still satisfied because we haven't affected the number of
  // black nodes in the path.
  if (node->color == RBTREE_COLOR_RED)
  {
    rbtree_replace_node(tree, child, node);
    return;
  }

  /**
   * Otherwise, the node is black. If its child is red, though, we can just
   * replace the node with its child and then paint it black and we're still
   * happy.
   */
  if (child != NULL && child->color == RBTREE_COLOR_RED)
  {
    child->color = RBTREE_COLOR_BLACK;
    rbtree_replace_node(tree, child, node);
    return;
  }

  /**
   * Now comes the tricky part. The node is black and its child is black, so by
   * removing the node, we shorten this path by one black node, which violates
   * property 5.
   *
   * Also note that child is actually guaranteed to be NULL now: if node had
   * NULL on one side and a black child on the other side, the paths would have
   * unequal black heights as the black child's leaves would also be black. As
   * such, we'll pretend that 'node' is actually the NULL leaf we're trying to
   * balance for while we run through the first iteration of the balancing
   * algorithm.
   */
  rbtree_node_t *current = node;
  rbtree_node_t *sibling;

  DEBUG_ASSERT(child == NULL);

  while (true)
  {
    /**
     * Case 1: current is the new root, in which case there isn't anything more
     * to be done. The heights won't differ anymore.
     */
    if (current->parent == NULL)
    {
      goto finish;
    }

    /**
     * Case 2: current's sibling is red, so the parent must be black. Swap their
     * colors and then rotate the parent to push the sibling closer to the root.
     */
    sibling = rbtree_node_sibling(current);

    if (sibling->color == RBTREE_COLOR_RED)
    {
      current->parent->color = RBTREE_COLOR_RED;
      sibling->color = RBTREE_COLOR_BLACK;

      if (current == current->parent->left)
        rbtree_rotate_left(tree, current->parent);
      else
        rbtree_rotate_right(tree, current->parent);
    }

    sibling = rbtree_node_sibling(current);

    /**
     * Case 3: current's parent, sibling, and the sibling's children are black.
     * If we just repaint the sibling red, our parent's subtree is now balanced.
     * However, it means that we need to repeat this algorithm again on our
     * parent, because this subtree now has one fewer black node on both its
     * paths.
     */
    if (current->parent->color == RBTREE_COLOR_BLACK &&
        sibling->color         == RBTREE_COLOR_BLACK &&
        ((sibling->left == NULL ||
          sibling->left->color == RBTREE_COLOR_BLACK) &&
         (sibling->right == NULL ||
          sibling->right->color == RBTREE_COLOR_BLACK)))
    {
      sibling->color = RBTREE_COLOR_RED;
      current = current->parent;
    }
    else
    {
      // Otherwise, we need to exit this loop and move on to case 4, 5, or 6 in
      // order to figure out which node is red and how to deal with that.
      goto case456;
    }
  }

case456:

  sibling = rbtree_node_sibling(current);

  DEBUG_ASSERT(sibling->color == RBTREE_COLOR_BLACK);

  /**
   * Case 4: sibling and sibling's children are black, but parent is red. Swap
   * the colors of sibling and parent. This doesn't affect sibling's path,
   * but it does affect current's path by removing one black node. Then we're
   * done.
   */
  if (current->parent->color == RBTREE_COLOR_RED &&
      ((sibling->left == NULL ||
        sibling->left->color == RBTREE_COLOR_BLACK) &&
       (sibling->right == NULL ||
        sibling->right->color == RBTREE_COLOR_BLACK)))
  {
    sibling->color         = RBTREE_COLOR_RED;
    current->parent->color = RBTREE_COLOR_BLACK;
    goto finish;
  }

  /**
   * Case 5: sibling is black, one of sibling's children is black and the other
   * is red, and current is on the same side of its parent as sibling's red
   * child is of sibling.
   *
   * We can rotate around the sibling and then swap colors in order to pull the
   * tree such that the new sibling's red node is on the opposite side. This
   * allows Case 6 to rotate correctly.
   *
   *   P               P
   *  / \             / \
   * C   S    -->    C   L
   *    / \               \
   *   L   R               S
   *                        \
   *                         R
   */
  DEBUG_ASSERT(
      (sibling->left != NULL &&
       sibling->left->color == RBTREE_COLOR_RED) ||
      (sibling->right != NULL &&
       sibling->right->color == RBTREE_COLOR_RED));

  if (current == current->parent->left &&
      (sibling->right == NULL ||
       sibling->right->color == RBTREE_COLOR_BLACK))
  {
    sibling->color       = RBTREE_COLOR_RED;
    sibling->left->color = RBTREE_COLOR_BLACK;
    rbtree_rotate_right(tree, sibling);

    sibling = sibling->parent;
  }
  else if (current == current->parent->right &&
           (sibling->left == NULL ||
            sibling->left->color == RBTREE_COLOR_BLACK))
  {
    sibling->color        = RBTREE_COLOR_RED;
    sibling->right->color = RBTREE_COLOR_BLACK;
    rbtree_rotate_left(tree, sibling);

    sibling = sibling->parent;
  }

  /**
   * Case 6: sibling is black, and sibling's child opposite current's side is
   * red. Rotate around the parent such that sibling moves closer to the root.
   * Then, swap the colors of our original sibling and parent, and paint our new
   * uncle black.
   *
   * Properties 4 and 5 are not affected because the subtree root color is still
   * the same, but now current has gained either a black parent or a black
   * grandparent. So, current has one more black node on its path.
   *
   * As for the other paths in the subtree:
   *
   * - current's new sibling: path contains the same number of black nodes as it
   *   did before; parent and original sibling have swapped colors and places
   *   but the path still goes through both
   *
   * - current's new uncle: original sibling's child. Number of black nodes is
   *   the same:
   *
   *       Px                       Sx
   *      / \                      / \
   *     Cb  Sb      -->          Pb  Ub
   *          \                  /
   *           Ur               Cb
   *
   * Thus we have restored all properties.
   */
  sibling->color = current->parent->color;
  current->parent->color = RBTREE_COLOR_BLACK;

  if (current == current->parent->left)
  {
    DEBUG_ASSERT(sibling->right != NULL &&
        sibling->right->color == RBTREE_COLOR_RED);

    sibling->right->color = RBTREE_COLOR_BLACK;
    rbtree_rotate_left(tree, current->parent);
  }
  else
  {
    DEBUG_ASSERT(sibling->left != NULL &&
        sibling->left->color == RBTREE_COLOR_RED);

    sibling->left->color = RBTREE_COLOR_BLACK;
    rbtree_rotate_right(tree, current->parent);
  }

finish:

  /**
   * Finally, we can just delete our node -- it's guaranteed to just stand in
   * for a leaf.
   */
  rbtree_replace_node(tree, NULL, node);
}

rbtree_node_t *rbtree_first_node(rbtree_t *tree)
{
  rbtree_node_t *node = tree->root;

  if (!node)
    return NULL;

  while (node->left != NULL)
    node = node->left;

  return node;
}

rbtree_node_t *rbtree_node_next(rbtree_node_t *node)
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
