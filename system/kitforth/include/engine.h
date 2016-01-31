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

extern void see_stub();
extern void dump_stub();
extern void evaluate_stub();

extern void add();
extern void sub();
extern void mul();
extern void divmod();

extern void bit_xor();
extern void bit_and();
extern void bit_or();
extern void bit_lshift();
extern void bit_rshift();

extern void equal();
extern void gt();
extern void gte();
extern void lt();
extern void lte();

extern void fetch();
extern void store();
extern void fetch_char();
extern void store_char();
extern void move();
extern void allocate_f();
extern void free_f();
extern void resize_f();

extern void dup();
extern void swap();
extern void over();
extern void rot();
extern void drop();
extern void to_rstack();
extern void from_rstack();
extern void fetch_rstack();

extern void state();
extern void cp_stub();
extern void cp_comma_stub();
extern void branch();
extern void branch_if_zero();

extern void display();
extern void emit();
extern void in_char();
extern void get_string();

extern void compiler_off();
extern void compiler_on();

extern void literal_stub();
extern void postpone_stub();
extern void postponed();
extern void immediate_stub();
extern void create_stub();
extern void defword_stub();
extern void endword_stub();

extern void parse_stub();

extern void syscall_from_forth();

extern void call();
extern void ret();
extern void ret_quit();

#endif
