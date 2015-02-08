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
  /*00*/ 0xFF,            // (none)
  /*01*/ (0  << 5) + 9,   // F9
  /*02*/ (0  << 5) + 7,   // F7
  /*03*/ (0  << 5) + 5,   // F5
  /*04*/ (0  << 5) + 3,   // F3
  /*05*/ (0  << 5) + 1,   // F1
  /*06*/ (0  << 5) + 2,   // F2
  /*07*/ (0  << 5) + 12,  // F12
  /*08*/ 0xFF,            // (none)
  /*09*/ (0  << 5) + 10,  // F10
  /*0A*/ (0  << 5) + 8,   // F8
  /*0B*/ (0  << 5) + 6,   // F6
  /*0C*/ (0  << 5) + 4,   // F4
  /*0D*/ 0xFF,            // TODO
  /*0E*/ (1  << 5) + 0,   // ` (backtick)
  /*0F*/ 0xFF,            // TODO
  /*10*/ 0xFF,            // TODO
  /*11*/ (5  << 5) + 2,   // Left Alt
  /*12*/ (4  << 5) + 0,   // Left Shift
  /*13*/ 0xFF,            // TODO
  /*14*/ (5  << 5) + 0,   // Left Control
  /*15*/ (2  << 5) + 1,   // q
  /*16*/ (1  << 5) + 1,   // 1
  /*17*/ 0xFF,            // TODO
  /*18*/ 0xFF,            // TODO
  /*19*/ 0xFF,            // TODO
  /*1A*/ (4  << 5) + 1,   // z
  /*1B*/ (3  << 5) + 2,   // s
  /*1C*/ (3  << 5) + 1,   // a
  /*1D*/ (2  << 5) + 2,   // w
  /*1E*/ (1  << 5) + 2,   // 2
  /*1F*/ 0xFF,            // TODO
  /*20*/ 0xFF,            // TODO
  /*21*/ (4  << 5) + 3,   // c
  /*22*/ (4  << 5) + 2,   // x
  /*23*/ (3  << 5) + 3,   // d
  /*24*/ (2  << 5) + 3,   // e
  /*25*/ (1  << 5) + 4,   // 4
  /*26*/ (1  << 5) + 3,   // 3
  /*27*/ 0xFF,            // TODO
  /*28*/ 0xFF,            // TODO
  /*29*/ (5  << 5) + 3,   // Space
  /*2A*/ (4  << 5) + 4,   // v
  /*2B*/ (3  << 5) + 4,   // f
  /*2C*/ (2  << 5) + 5,   // t
  /*2D*/ (2  << 5) + 4,   // r
  /*2E*/ (1  << 5) + 5,   // 5
  /*2F*/ 0xFF,            // TODO
  /*30*/ 0xFF,            // TODO
  /*31*/ (4  << 5) + 6,   // n
  /*32*/ (4  << 5) + 5,   // b
  /*33*/ (3  << 5) + 6,   // h
  /*34*/ (3  << 5) + 5,   // g
  /*35*/ (2  << 5) + 6,   // y
  /*36*/ (1  << 5) + 6,   // 6
  /*37*/ 0xFF,            // TODO
  /*38*/ 0xFF,            // TODO
  /*39*/ 0xFF,            // TODO
  /*3A*/ (4  << 5) + 7,   // m
  /*3B*/ (3  << 5) + 7,   // j
  /*3C*/ (2  << 5) + 7,   // u
  /*3D*/ (1  << 5) + 7,   // 7
  /*3E*/ (1  << 5) + 8,   // 8
  /*3F*/ 0xFF,            // TODO
  /*40*/ 0xFF,            // TODO
  /*41*/ (4  << 5) + 8,   // , (comma)
  /*42*/ (3  << 5) + 8,   // k
  /*43*/ (2  << 5) + 8,   // i
  /*44*/ (2  << 5) + 9,   // o
  /*45*/ (1  << 5) + 10,  // 0
  /*46*/ (1  << 5) + 9,   // 9
  /*47*/ 0xFF,            // TODO
  /*48*/ 0xFF,            // TODO
  /*49*/ (4  << 5) + 9,   // . (period)
  /*4A*/ (4  << 5) + 10,  // / (slash)
  /*4B*/ (3  << 5) + 9,   // l
  /*4C*/ (3  << 5) + 10,  // ; (semicolon)
  /*4D*/ (2  << 5) + 10,  // p
  /*4E*/ (1  << 5) + 11,  // - (hyphen)
  /*4F*/ 0xFF,            // TODO
  /*50*/ 0xFF,            // TODO
  /*51*/ 0xFF,            // TODO
  /*52*/ (3  << 5) + 11,  // ' (single quote)
  /*53*/ 0xFF,            // TODO
  /*54*/ (2  << 5) + 11,  // [ (left square bracket)
  /*55*/ (1  << 5) + 12,  // = (equal sign)
  /*56*/ 0xFF,            // TODO
  /*57*/ 0xFF,            // TODO
  /*58*/ 0xFF,            // TODO
  /*59*/ (4  << 5) + 11,  // Right Shift
  /*5A*/ (3  << 5) + 12,  // Enter
  /*5B*/ (2  << 5) + 12,  // ] (right square bracket)
  /*5C*/ 0xFF,            // TODO
  /*5D*/ (2  << 5) + 13,  // \ (backslash)
  /*5E*/ 0xFF,            // TODO
  /*5F*/ 0xFF,            // TODO
  /*60*/ 0xFF,            // TODO
  /*61*/ 0xFF,            // TODO
  /*62*/ 0xFF,            // TODO
  /*63*/ 0xFF,            // TODO
  /*64*/ 0xFF,            // TODO
  /*65*/ 0xFF,            // TODO
  /*66*/ (1  << 5) + 13,  // Backspace
  /*67*/ 0xFF,            // TODO
  /*68*/ 0xFF,            // TODO
  /*69*/ 0xFF,            // TODO
  /*6A*/ 0xFF,            // TODO
  /*6B*/ 0xFF,            // TODO
  /*6C*/ (1  << 5) + 15,  // Home
  /*6D*/ 0xFF,            // TODO
  /*6E*/ 0xFF,            // TODO
  /*6F*/ 0xFF,            // TODO
  /*70*/ (1  << 5) + 14,  // Insert
  /*71*/ 0xFF,            // TODO
  /*72*/ 0xFF,            // TODO
  /*73*/ 0xFF,            // TODO
  /*74*/ 0xFF,            // TODO
  /*75*/ 0xFF,            // TODO
  /*76*/ (0  << 5) + 0,   // Escape
  /*77*/ (1  << 5) + 17,  // Num Lock
  /*78*/ (0  << 5) + 11,  // F11
  /*79*/ 0xFF,            // TODO
  /*7A*/ 0xFF,            // TODO
  /*7B*/ (1  << 5) + 20,  // - (hyphen) [numpad]
  /*7C*/ (1  << 5) + 19,  // * (asterisk) [numpad]
  /*7D*/ (1  << 5) + 16,  // Page Up
  /*7E*/ (0  << 5) + 14,  // Scroll Lock
  /*7F*/ 0xFF,            // TODO
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
