/*******************************************************************************
 *
 * kit/kernel/include/rbtree.h
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
 * rbtree explicitly avoids any key comparison in order to remain generic. As
 * such, there is no rbtree_search() function. You are expected to extend rbtree
 * by implementing your own search function for your target key, as well as an
 * insert function that searches and inserts the node, then calls
 * rbtree_balance_insert() to balance the tree.
 *
 * This module's interface is inspired by the Linux kernel's rbtree.c.
 *
 ******************************************************************************/

typedef struct rbtree_node
{
  enum {
    RBTREE_COLOR_BLACK,
    RBTREE_COLOR_RED
  } color;

  struct rbtree_node *parent;

  struct rbtree_node *left;
  struct rbtree_node *right;
} rbtree_node_t;

typedef struct rbtree
{
  rbtree_node_t *root;
} rbtree_t;

/**
 * Use this after setting node->parent, and either node->parent->left or
 * node->parent->right to node depending on comparison key.
 */
void rbtree_balance_insert(rbtree_t *tree, rbtree_node_t *node);

/**
 * Unlike rbtree_balance_insert(), as no comparison is required for deletion,
 * this function can be called without any special setup. It detaches the node
 * from the tree and balances automatically. Of course, any management of memory
 * and destruction is left up to the user.
 */
//void rbtree_delete(rbtree_t *tree, rbtree_node_t *node);

const rbtree_node_t *rbtree_first_node(const rbtree_t *tree);

const rbtree_node_t *rbtree_node_next(const rbtree_node_t *node);