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

void scheduler_yield();

int scheduler_preempt();

void scheduler_sleep();

int scheduler_wake(process_id_t pid);

#endif
