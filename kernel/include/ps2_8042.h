/*******************************************************************************
 *
 * kit/kernel/include/ps2_8042.h
 * - 8042 PS/2 controller driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef PS2_8042_H
#define PS2_8042_H

#include <stdbool.h>
#include <stdint.h>

#include "config.h"

#define PS2_8042_DATA_PORT    0x60
#define PS2_8042_COMMAND_PORT 0x64

bool ps2_8042_initialize();

uint8_t ps2_8042_read_data();

void ps2_8042_write_data(uint8_t data);

void ps2_8042_write_to_keyboard(uint8_t data);

typedef struct PACKED ps2_8042_status
{
  uint8_t output_full : 1;
  uint8_t input_full  : 1;
  uint8_t system_ok   : 1;

  enum {
    PS2_8042_MODE_COMMAND = 0,
    PS2_8042_MODE_DATA    = 1
  } data_mode : 1;

  uint8_t unknown1    : 1;
  uint8_t unknown2    : 1;

  uint8_t timeout_err : 1;
  uint8_t parity_err  : 1;
} ps2_8042_status_t;

ps2_8042_status_t ps2_8042_read_status();

bool ps2_8042_wait_for_input_buffer();
bool ps2_8042_wait_for_output_buffer();

void ps2_8042_send_command(uint8_t command);

void ps2_8042_cpu_reset();

typedef struct PACKED ps2_8042_config
{
  uint8_t device1_irq_enabled : 1;
  uint8_t device2_irq_enabled : 1;
  uint8_t system_ok           : 1;
  uint8_t zero1               : 1;
  uint8_t device1_clock       : 1;
  uint8_t device2_clock       : 1;
  uint8_t device1_translate   : 1;
  uint8_t zero2               : 1;
} ps2_8042_config_t;

ps2_8042_config_t ps2_8042_read_config();

void ps2_8042_write_config(ps2_8042_config_t config);

void ps2_8042_handle_irq1();

#endif
