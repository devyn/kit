/*******************************************************************************
 *
 * kit/system/kitforth/include/engine.h
 * - kitFORTH low-level operations & execution engine
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef ENGINE_H
#define ENGINE_H

// Callable with instruction pointer & data pointer
// Returns new data pointer
extern uint64_t *execute(void (**ip)(), uint64_t *dp);

// NOT CALLABLE! These are forth-native asm routines to be included in threaded
// code arrays
extern void push();

extern void add();
extern void bit_xor();
extern void equal();

extern void load_cell();
extern void store_cell();

extern void dup();
extern void swap();
extern void over();
extern void rot();
extern void drop();
extern void to_rstack();
extern void from_rstack();
extern void fetch_rstack();

extern void here_stub();
extern void here_incr_stub();
extern void branch();
extern void branch_if_zero();

extern void display();
extern void emit();
extern void in_char();

extern void compiler_off();
extern void compiler_on();

extern void call();
extern void ret();
extern void ret_quit();

extern void literal_stub();
extern void postpone_stub();
extern void postponed();
extern void immediate_stub();
extern void defword_stub();
extern void endword_stub();

#endif
