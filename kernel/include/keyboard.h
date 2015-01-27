/*******************************************************************************
 *
 * kit/kernel/include/keyboard.h
 * - generic keyboard input handler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef KEYBOARD_H
#define KEYBOARD_H

#include <stdint.h>
#include <stdbool.h>

#define KEYBOARD_KC_SHIFT ((4 << 5) + 0)

typedef struct keyboard_event
{
  uint8_t keycode;
  char    keychar; // ignore if '\0'

  bool    pressed    : 1;
  bool    ctrl_down  : 1;
  bool    alt_down   : 1;
  bool    shift_down : 1;
} keyboard_event_t;

void keyboard_initialize();

bool keyboard_enqueue(const keyboard_event_t *event);
bool keyboard_dequeue(keyboard_event_t *event);
void keyboard_wait_dequeue(keyboard_event_t *event);

void keyboard_handle_keypress(uint8_t keycode);
void keyboard_handle_keyrelease(uint8_t keycode);

#endif
