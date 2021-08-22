/*******************************************************************************
 *
 * kit/kernel/include/scheduler.h
 * - time and event based task scheduler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#ifndef SCHEDULER_H
#define SCHEDULER_H

#include <stdbool.h>

#include "process.h"

int scheduler_initialized();

/**
 * When scheduler_tick() returns, process_current is guaranteed to be the same
 * as it was before, but many things may have happened in between. Thus, the
 * entire return path leading from scheduler_tick() must be reentrant.
 */
void scheduler_tick();

void scheduler_sleep();

int scheduler_wake(process_id_t pid);

#endif
