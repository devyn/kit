/*******************************************************************************
 *
 * kit/kernel/ps2key.c
 * - PS/2 keyboard driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "ps2key.h"
#include "keyboard.h"
#include "debug.h"

static const uint8_t ps2key_noprefix_usqwerty_map[128] = {
  [0x00] = 0xFF,            // (none)
  [0x01] = (0  << 5) + 9,   // F9
  [0x02] = (0  << 5) + 7,   // F7
  [0x03] = (0  << 5) + 5,   // F5
  [0x04] = (0  << 5) + 3,   // F3
  [0x05] = (0  << 5) + 1,   // F1
  [0x06] = (0  << 5) + 2,   // F2
  [0x07] = (0  << 5) + 12,  // F12
  [0x08] = 0xFF,            // (none)
  [0x09] = (0  << 5) + 10,  // F10
  [0x0A] = (0  << 5) + 8,   // F8
  [0x0B] = (0  << 5) + 6,   // F6
  [0x0C] = (0  << 5) + 4,   // F4
  [0x0D] = 0xFF,            // TODO
  [0x0E] = (1  << 5) + 0,   // ` (backtick)
  [0x0F] = 0xFF,            // TODO
  [0x10] = 0xFF,            // TODO
  [0x11] = (5  << 5) + 2,   // Left Alt
  [0x12] = (4  << 5) + 0,   // Left Shift
  [0x13] = 0xFF,            // TODO
  [0x14] = (5  << 5) + 0,   // Left Control
  [0x15] = (2  << 5) + 1,   // q
  [0x16] = (1  << 5) + 1,   // 1
  [0x17] = 0xFF,            // TODO
  [0x18] = 0xFF,            // TODO
  [0x19] = 0xFF,            // TODO
  [0x1A] = (4  << 5) + 1,   // z
  [0x1B] = (3  << 5) + 2,   // s
  [0x1C] = (3  << 5) + 1,   // a
  [0x1D] = (2  << 5) + 2,   // w
  [0x1E] = (1  << 5) + 2,   // 2
  [0x1F] = 0xFF,            // TODO
  [0x20] = 0xFF,            // TODO
  [0x21] = (4  << 5) + 3,   // c
  [0x22] = (4  << 5) + 2,   // x
  [0x23] = (3  << 5) + 3,   // d
  [0x24] = (2  << 5) + 3,   // e
  [0x25] = (1  << 5) + 4,   // 4
  [0x26] = (1  << 5) + 3,   // 3
  [0x27] = 0xFF,            // TODO
  [0x28] = 0xFF,            // TODO
  [0x29] = (5  << 5) + 3,   // Space
  [0x2A] = (4  << 5) + 4,   // v
  [0x2B] = (3  << 5) + 4,   // f
  [0x2C] = (2  << 5) + 5,   // t
  [0x2D] = (2  << 5) + 4,   // r
  [0x2E] = (1  << 5) + 5,   // 5
  [0x2F] = 0xFF,            // TODO
  [0x30] = 0xFF,            // TODO
  [0x31] = (4  << 5) + 6,   // n
  [0x32] = (4  << 5) + 5,   // b
  [0x33] = (3  << 5) + 6,   // h
  [0x34] = (3  << 5) + 5,   // g
  [0x35] = (2  << 5) + 6,   // y
  [0x36] = (1  << 5) + 6,   // 6
  [0x37] = 0xFF,            // TODO
  [0x38] = 0xFF,            // TODO
  [0x39] = 0xFF,            // TODO
  [0x3A] = (4  << 5) + 7,   // m
  [0x3B] = (3  << 5) + 7,   // j
  [0x3C] = (2  << 5) + 7,   // u
  [0x3D] = (1  << 5) + 7,   // 7
  [0x3E] = (1  << 5) + 8,   // 8
  [0x3F] = 0xFF,            // TODO
  [0x40] = 0xFF,            // TODO
  [0x41] = (4  << 5) + 8,   // , (comma)
  [0x42] = (3  << 5) + 8,   // k
  [0x43] = (2  << 5) + 8,   // i
  [0x44] = (2  << 5) + 9,   // o
  [0x45] = (1  << 5) + 10,  // 0
  [0x46] = (1  << 5) + 9,   // 9
  [0x47] = 0xFF,            // TODO
  [0x48] = 0xFF,            // TODO
  [0x49] = (4  << 5) + 9,   // . (period)
  [0x4A] = (4  << 5) + 10,  // / (slash)
  [0x4B] = (3  << 5) + 9,   // l
  [0x4C] = (3  << 5) + 10,  // ; (semicolon)
  [0x4D] = (2  << 5) + 10,  // p
  [0x4E] = (1  << 5) + 11,  // - (hyphen)
  [0x4F] = 0xFF,            // TODO
  [0x50] = 0xFF,            // TODO
  [0x51] = 0xFF,            // TODO
  [0x52] = (3  << 5) + 11,  // ' (single quote)
  [0x53] = 0xFF,            // TODO
  [0x54] = (2  << 5) + 11,  // [ (left square bracket)
  [0x55] = (1  << 5) + 12,  // = (equal sign)
  [0x56] = 0xFF,            // TODO
  [0x57] = 0xFF,            // TODO
  [0x58] = 0xFF,            // TODO
  [0x59] = (4  << 5) + 11,  // Right Shift
  [0x5A] = (3  << 5) + 12,  // Enter
  [0x5B] = (2  << 5) + 12,  // ] (right square bracket)
  [0x5C] = 0xFF,            // TODO
  [0x5D] = (2  << 5) + 13,  // \ (backslash)
  [0x5E] = 0xFF,            // TODO
  [0x5F] = 0xFF,            // TODO
  [0x60] = 0xFF,            // TODO
  [0x61] = 0xFF,            // TODO
  [0x62] = 0xFF,            // TODO
  [0x63] = 0xFF,            // TODO
  [0x64] = 0xFF,            // TODO
  [0x65] = 0xFF,            // TODO
  [0x66] = (1  << 5) + 13,  // Backspace
  [0x67] = 0xFF,            // TODO
  [0x68] = 0xFF,            // TODO
  [0x69] = 0xFF,            // TODO
  [0x6A] = 0xFF,            // TODO
  [0x6B] = 0xFF,            // TODO
  [0x6C] = (1  << 5) + 15,  // Home
  [0x6D] = 0xFF,            // TODO
  [0x6E] = 0xFF,            // TODO
  [0x6F] = 0xFF,            // TODO
  [0x70] = (1  << 5) + 14,  // Insert
  [0x71] = 0xFF,            // TODO
  [0x72] = 0xFF,            // TODO
  [0x73] = 0xFF,            // TODO
  [0x74] = 0xFF,            // TODO
  [0x75] = 0xFF,            // TODO
  [0x76] = (0  << 5) + 0,   // Escape
  [0x77] = (1  << 5) + 17,  // Num Lock
  [0x78] = (0  << 5) + 11,  // F11
  [0x79] = 0xFF,            // TODO
  [0x7A] = 0xFF,            // TODO
  [0x7B] = (1  << 5) + 20,  // - (hyphen) [numpad]
  [0x7C] = (1  << 5) + 19,  // * (asterisk) [numpad]
  [0x7D] = (1  << 5) + 16,  // Page Up
  [0x7E] = (0  << 5) + 14,  // Scroll Lock
  [0x7F] = 0xFF,            // TODO
};

static enum
{
  PS2KEY_STATE_DEFAULT,
  PS2KEY_STATE_EXTEND_DEFAULT,
  PS2KEY_STATE_RELEASE,
  PS2KEY_STATE_EXTEND_RELEASE,

  PS2KEY_STATE_PAUSE,
} ps2key_state;

static int ps2key_special_counter;

void ps2key_initialize()
{
  ps2key_state           = PS2KEY_STATE_DEFAULT;
  ps2key_special_counter = 0;
}

void ps2key_handle_irq(uint8_t data)
{
  switch (ps2key_state)
  {
    case PS2KEY_STATE_DEFAULT:
      switch (data)
      {
        case 0xF0:
          ps2key_state = PS2KEY_STATE_RELEASE;
          break;
        case 0xE0:
          ps2key_state = PS2KEY_STATE_EXTEND_DEFAULT;
          break;
        case 0xE1:
          ps2key_state = PS2KEY_STATE_PAUSE;
          ps2key_special_counter = 7;
          break;
        default:
          keyboard_handle_keypress(ps2key_noprefix_usqwerty_map[data]);
      }
      break;

    case PS2KEY_STATE_EXTEND_DEFAULT:
      switch (data)
      {
        case 0xF0:
          ps2key_state = PS2KEY_STATE_EXTEND_RELEASE;
          break;
        default:
          ps2key_state = PS2KEY_STATE_DEFAULT;
          keyboard_handle_keypress(0xFE /* TODO */);
      }
      break;

    case PS2KEY_STATE_RELEASE:
      ps2key_state = PS2KEY_STATE_DEFAULT;
      keyboard_handle_keyrelease(ps2key_noprefix_usqwerty_map[data]);
      break;

    case PS2KEY_STATE_EXTEND_RELEASE:
      ps2key_state = PS2KEY_STATE_DEFAULT;
      keyboard_handle_keyrelease(0xFE /* TODO */);
      break;

    case PS2KEY_STATE_PAUSE:
      if (--ps2key_special_counter <= 0)
        ps2key_state = PS2KEY_STATE_DEFAULT;
      break;
  }
}
