/*******************************************************************************
 *
 * kit/kernel/keyboard.c
 * - generic keyboard input handler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stddef.h>

#include "keyboard.h"
#include "debug.h"
#include "memory.h"
#include "x86_64.h"

/**
 * +-----+--------+
 * | 0:4 | 5:7    |
 * +-----+--------+
 * | row | column |
 * +-----+--------+
 * <--- 1 byte --->
 *
 * row    = code >> 5;
 * column = code & 0x1f;
 */
static const char keyboard_qwerty_char_map[256] = {
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  '`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-',  '=',  '\b', 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[',  ']',  '\\', 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '\n', 0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
};
static const char keyboard_qwerty_char_shift_map[256] = {
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  '~', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_',  '+',  0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{',  '}',  '|',  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"',  '\n', 0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   'Z', 'X', 'C', 'V', 'B', 'N', 'M', '<', '>', '?', 0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
  0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,    0,    0,    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
};

static bool keyboard_ctrl_down;
static bool keyboard_alt_down;
static bool keyboard_shift_down;

static struct
{
  keyboard_event_t *buffer;
  size_t            length;
  size_t            start;
  size_t            end;
} keyboard_queue;

void keyboard_initialize()
{
  // Reset boolean state flags.
  keyboard_ctrl_down    = false;
  keyboard_alt_down     = false;
  keyboard_shift_down   = false;

  // Allocate 1024-entry ring buffer.
  keyboard_queue.buffer = memory_alloc(sizeof(keyboard_event_t) * 1024);
  keyboard_queue.length = 1024;
  keyboard_queue.start  = 0;
  keyboard_queue.end    = 0;

  DEBUG_ASSERT(keyboard_queue.buffer != NULL);
}

static inline char keyboard_get_keychar(uint8_t keycode)
{
  char keychar;
  
  if (keyboard_shift_down)
  {
    keychar = keyboard_qwerty_char_shift_map[keycode];
  }
  else
  {
    keychar = keyboard_qwerty_char_map[keycode];
  }

  return keychar;
}

bool keyboard_enqueue(const keyboard_event_t *event)
{
  if (keyboard_queue.end != keyboard_queue.start - 1)
  {
    memory_copy(event, keyboard_queue.buffer + keyboard_queue.end,
        sizeof(keyboard_event_t));

    // Wrap around.
    if (++keyboard_queue.end >= keyboard_queue.length)
      keyboard_queue.end = 0;

    return true;
  }
  else
  {
    DEBUG_FORMAT("dropping event due to full queue; max %lu entries",
        keyboard_queue.length);
    return false;
  }
}

bool keyboard_dequeue(keyboard_event_t *event)
{
  if (keyboard_queue.end != keyboard_queue.start)
  {
    memory_copy(keyboard_queue.buffer + keyboard_queue.start, event,
        sizeof(keyboard_event_t));

    // Wrap around.
    if (++keyboard_queue.start >= keyboard_queue.length)
      keyboard_queue.start = 0;

    return true;
  }
  else
  {
    return false;
  }
}

void keyboard_wait_dequeue(keyboard_event_t *event)
{
  while (!keyboard_dequeue(event)) hlt();
}

void keyboard_handle_keypress(uint8_t keycode)
{
  // First, check if this is a modifier key, and set the appropriate state flag
  // if so.
  switch (keycode)
  {
    case KEYBOARD_KC_SHIFT:
      keyboard_shift_down = true;
      break;
  }

  // Next, generate the event.
  keyboard_event_t event;

  event.keycode    = keycode;
  event.keychar    = keyboard_get_keychar(keycode);

  event.pressed    = true;

  event.ctrl_down  = keyboard_ctrl_down;
  event.alt_down   = keyboard_alt_down;
  event.shift_down = keyboard_shift_down;

  keyboard_enqueue(&event);
}

void keyboard_handle_keyrelease(uint8_t keycode)
{
  // First, check if this is a modifier key, and clear the appropriate state
  // flag if so.
  switch (keycode)
  {
    case KEYBOARD_KC_SHIFT:
      keyboard_shift_down = false;
      break;
  }

  // Next, generate the event.
  keyboard_event_t event;

  event.keycode    = keycode;
  event.keychar    = keyboard_get_keychar(keycode);

  event.pressed    = false;

  event.ctrl_down  = keyboard_ctrl_down;
  event.alt_down   = keyboard_alt_down;
  event.shift_down = keyboard_shift_down;

  keyboard_enqueue(&event);
}
