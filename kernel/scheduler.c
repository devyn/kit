/*******************************************************************************
 *
 * kit/kernel/scheduler.c
 * - time and event based task scheduler
 *
 * vim:ts=2:sw=2:et:tw=80:ft=c
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

#include "scheduler.h"
#include "interrupt.h"
#include "x86_64.h"
#include "debug.h"

process_t *run_queue_front = NULL;
process_t *run_queue_back  = NULL;

void scheduler_tick()
{
  process_t *next;

  if (process_current != NULL)
  {
    if (process_current->sched.waiting)
    {
      // Don't do anything.
      return;
    }

    while ((next = scheduler_dequeue_run()) == NULL)
    {
      if (process_current->state == PROCESS_STATE_RUNNING)
      {
        // Nothing to do. Continue running.
        return;
      }
      else
      {
        // Wait for an interrupt to wake us up.
        process_current->sched.waiting = true;

        interrupt_enable();
        hlt();
        interrupt_disable();

        process_current->sched.waiting = false;
      }
    }

    if (next != process_current)
    {
      if (process_current->state == PROCESS_STATE_RUNNING)
      {
        // Enqueue to be run again later.
        scheduler_enqueue_run(process_current);
      }

      process_switch(next);
    }
  }
  else
  {
    DEBUG_ASSERT((next = scheduler_dequeue_run()) != NULL);

    process_switch(next);
  }
}

void scheduler_enqueue_run(process_t *process)
{
  if (run_queue_back == NULL)
  {
    run_queue_front = run_queue_back = process;
    process->sched.run_queue_next = NULL;
  }
  else
  {
    run_queue_back->sched.run_queue_next = process;
    run_queue_back = process;
  }
}

process_t *scheduler_dequeue_run()
{
  if (run_queue_front != NULL)
  {
    process_t *process = run_queue_front;

    run_queue_front = process->sched.run_queue_next;

    process->sched.run_queue_next = NULL;

    if (run_queue_front == NULL)
    {
      run_queue_back = NULL;
    }

    return process;
  }
  else
  {
    return NULL;
  }
}

void scheduler_sleep()
{
  DEBUG_ASSERT(process_current != NULL);
  DEBUG_ASSERT(process_current->state == PROCESS_STATE_RUNNING);

  process_current->state = PROCESS_STATE_SLEEPING;
  scheduler_tick();
}

bool scheduler_wake(process_t *process)
{
  if (process->state == PROCESS_STATE_SLEEPING)
  {
    process->state = PROCESS_STATE_RUNNING;
    scheduler_enqueue_run(process);

    return true;
  }
  else
  {
    return false;
  }
}
