/*******************************************************************************
 *
 * kit/system/util/key.c
 * - inspects key events until Ctrl+D is pressed
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2013, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <kit/syscall.h>

#define UNUSED __attribute__((__unused__))

int main(UNUSED int argc, UNUSED char **argv)
{
  keyboard_event_t event;

  event.keychar = '\0';

  while (!(event.ctrl_down && event.keychar == 'd'))
  {
    syscall_key_get(&event);

    char event_info[6];

    event_info[0] = event.keychar;
    event_info[1] = event.pressed    ? 'P' : '-';
    event_info[2] = event.ctrl_down  ? 'C' : '-';
    event_info[3] = event.alt_down   ? 'A' : '-';
    event_info[4] = event.shift_down ? 'S' : '-';
    event_info[5] = '\n';

    syscall_twrite(sizeof(event_info), event_info);
  }

  return 0;
}
