/*******************************************************************************
 *
 * kit/kernel/ps2_8042.c
 * - 8042 PS/2 controller driver
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "ps2_8042.h"
#include "ps2key.h"
#include "x86_64.h"
#include "debug.h"

/**
 * Warning: ensure interrupts are disabled before calling!
 *
 * TODO: disable USB legacy support
 * TODO: make sure PS/2 controller exists
 * TODO: support 2-channel controller
 */
bool ps2_8042_initialize()
{
  // Disable channel(s).
  ps2_8042_send_command(0xAD); // disable 1
  ps2_8042_send_command(0xA7); // disable 2 (or ignore)

  // Flush the output buffer.
  inb(PS2_8042_DATA_PORT);

  // Load config.
  ps2_8042_config_t config = ps2_8042_read_config();

  // Disable IRQs.
  config.device1_irq_enabled = 0;
  config.device2_irq_enabled = 0;

  // Disable translation.
  config.device1_translate   = 0;

  // Save config.
  ps2_8042_write_config(config);

  // Perform self test.
  ps2_8042_send_command(0xAA);

  if (ps2_8042_wait_for_output_buffer())
  {
    uint8_t response = ps2_8042_read_data();

    if (response != 0x55)
    {
      DEBUG_FORMAT("expected self test response 0x55, got %#x", response);
      return false;
    }
  }
  else
  {
    DEBUG_MESSAGE("no self test response received");
    return false;
  }

  // Perform interface test on first channel.
  ps2_8042_send_command(0xAB);

  if (ps2_8042_wait_for_output_buffer())
  {
    uint8_t response = ps2_8042_read_data();

    switch (response)
    {
      case 0x00:
        // Pass.
        break;

      case 0x01:
        DEBUG_MESSAGE("clock line stuck low on PS/2 channel 1");
        return false;
      case 0x02:
        DEBUG_MESSAGE("clock line stuck high on PS/2 channel 1");
        return false;
      case 0x03:
        DEBUG_MESSAGE("data line stuck low on PS/2 channel 1");
        return false;
      case 0x04:
        DEBUG_MESSAGE("data line stuck high on PS/2 channel 1");
        return false;
      default:
        DEBUG_FORMAT("unknown interface test response %#x on PS/2 channel 1",
            response);
        return false;
    }
  }
  else
  {
    DEBUG_MESSAGE("no response received while testing PS/2 channel 1");
    return false;
  }

  // Enable first channel.
  ps2_8042_send_command(0xAE);

  // Reset device 1.
  ps2_8042_write_data(0xFF);

  if (ps2_8042_wait_for_output_buffer())
  {
    if (ps2_8042_read_data() != 0xFA)
    {
      DEBUG_MESSAGE("PS/2 device 1 reset failure");
      return false;
    }
  }
  else
  {
    DEBUG_MESSAGE("PS/2 device 1 not present");
    return false;
  }

  // Wait for the self-test to pass.
  if (ps2_8042_wait_for_output_buffer() && ps2_8042_read_data() == 0xAA)
  {
    //DEBUG_MESSAGE("PS/2 device 1 self-test passed");
  }
  else
  {
    DEBUG_MESSAGE("PS/2 device 1 self-test failed");
    return false;
  }

  // Enable IRQ for device 1.
  config = ps2_8042_read_config();

  config.device1_irq_enabled = 1;

  ps2_8042_write_config(config);

  return true;
}

uint8_t ps2_8042_read_data()
{
  return inb(PS2_8042_DATA_PORT);
}

void ps2_8042_write_data(uint8_t data)
{
  DEBUG_ASSERT(ps2_8042_wait_for_input_buffer());

  outb(data, PS2_8042_DATA_PORT);
}

void ps2_8042_write_to_keyboard(uint8_t data)
{
  // FIXME: should decide *which* device is the keyboard first
  ps2_8042_write_data(data);
}

ps2_8042_status_t ps2_8042_read_status()
{
  union
  {
    uint8_t           byte;
    ps2_8042_status_t status;
  } intermediate;

  intermediate.byte = inb(PS2_8042_COMMAND_PORT);

  return intermediate.status;
}

bool ps2_8042_wait_for_input_buffer()
{
  ps2_8042_status_t status;
  int timeout = 400000;

  do {
    status = ps2_8042_read_status();
    timeout--;
  }
  while (status.input_full && timeout > 0);

  return !status.input_full;
}

bool ps2_8042_wait_for_output_buffer()
{
  ps2_8042_status_t status;
  int timeout = 400000;

  do {
    status = ps2_8042_read_status();
    timeout--;
  }
  while (!status.output_full && timeout > 0);

  return status.output_full;
}

void ps2_8042_send_command(uint8_t command)
{
  DEBUG_ASSERT(ps2_8042_wait_for_input_buffer());

  outb(command, PS2_8042_COMMAND_PORT);
}

void ps2_8042_cpu_reset()
{
  ps2_8042_send_command(0xFE); // pulse reset line
}

ps2_8042_config_t ps2_8042_read_config()
{
  union
  {
    uint8_t           byte;
    ps2_8042_config_t config;
  } intermediate;

  ps2_8042_send_command(0x20); // read config byte

  DEBUG_ASSERT(ps2_8042_wait_for_output_buffer());

  intermediate.byte = ps2_8042_read_data();

  return intermediate.config;
}

void ps2_8042_write_config(ps2_8042_config_t config)
{
  union
  {
    uint8_t           byte;
    ps2_8042_config_t config;
  } intermediate;

  intermediate.config = config;

  ps2_8042_send_command(0x60); // write config byte

  ps2_8042_write_data(intermediate.byte);

  DEBUG_ASSERT(ps2_8042_wait_for_input_buffer());
}

void ps2_8042_handle_irq1()
{
  uint8_t data = ps2_8042_read_data();

  ps2key_handle_irq(data);
}
