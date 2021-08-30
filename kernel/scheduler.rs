/*******************************************************************************
 *
 * kit/kernel/scheduler.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Time and event based task scheduler.

use alloc::collections::VecDeque;

use crate::process::{self, RcProcess};
use crate::interrupt;
use crate::sync::Spinlock;

struct GlobalState {
    run_queue: Spinlock<VecDeque<RcProcess>>,
    preempt_lock: Spinlock<()>,
}

static mut GLOBAL_STATE: Option<GlobalState> = None;
static mut INITIALIZED: bool = false;

pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}

pub unsafe fn initialize() {
    if INITIALIZED {
        panic!("scheduler already initialized");
    }

    GLOBAL_STATE = Some(GlobalState {
        run_queue: Spinlock::new(VecDeque::new()),
        preempt_lock: Spinlock::new(()),
    });

    INITIALIZED = true;
}

fn global_state<'a>() -> &'a GlobalState {
    unsafe {
        GLOBAL_STATE.as_ref()
            .unwrap_or_else(|| panic!("Scheduler not initialized!"))
    }
}

/// Pushes a process on to the end of the run queue.
///
/// # Panics
///
/// Panics if the process is not in the `Running` state.
pub fn push(process: RcProcess) {
    assert!(process.lock().is_running());

    global_state().run_queue.lock().push_back(process);
}

/// Gets the first running process from the queue, discarding non-running
/// processes.
fn pop_running() -> Option<RcProcess> {
    let mut run_queue = global_state().run_queue.lock();

    while let Some(process) = run_queue.pop_front() {
        if process.lock().is_running() {
            return Some(process);
        }
    }

    None
}

/// Given a sleeping process, wakes it up and pushes it on to the end of the run
/// queue.
///
/// # Returns
///
/// - `Ok(true)` if the process was awoken
/// - `Ok(false)` if the process was already running
/// - `Err(state)` if the process was neither `Running` nor `Sleeping`.
pub fn awaken(process: RcProcess) -> Result<bool, process::State> {
    use process::State::{Running, Sleeping};

    let state = process.lock().state();

    match state {
        Running => Ok(false),
        Sleeping => {
            process.lock().awaken();
            push(process);
            Ok(true)
        },
        _ => Err(state)
    }
}


/// Iterates the scheduler loop so that other processes may execute.
///
/// If no other processes are ready to execute, and the current process is still
/// ready for execution, control will be returned to the current process
/// immediately.
///
/// If no processes are ready for execution, `tick()` will halt the processor
/// and accept interrupts until a process is ready.
///
/// # Panics
///
/// Panics if the scheduler has not been initialized.
pub fn r#yield() {
    assert!(initialized(), "yield() called before scheduler initialized");

    'switched: loop {
        // Wait for the preempt lock.
        let preempt_lock = global_state().preempt_lock.lock();

        // Allow pending interrupts to run.
        unsafe { interrupt::accept(); }

        let current_process = process::current();
        let next_process;

        // Get a process that we're allowed to run
        'got_process: loop {
            if let Some(next) = pop_running() {
                // Ready process on queue
                next_process = next;
                break 'got_process;
            } else if current_process.lock().is_running() {
                // Current process can be run
                next_process = current_process;
                break 'got_process;
            } else {
                // Maybe something will change if we wait for an interrupt.
                unsafe { interrupt::wait(); }
            }
        }

        drop(preempt_lock);

        // Try to switch, loop again if we couldn't.
        if switch(next_process) {
            break 'switched;
        }
    }
}

/// A weaker version of [r#yield], which will return immediately if:
///
/// 1. the preempt lock can't be acquired
/// 2. there are no other processes waiting
/// 3. or the scheduler hasn't been initialized yet.
///
/// This is intended to be called by the timer to enable timesharing.
///
/// Returns true if a switch occurred.
pub fn preempt() -> bool {
    if !initialized() { return false; }

    let next_process;

    if let Some(preempt_lock) = global_state().preempt_lock.try_lock() {
        if let Some(next) = pop_running() {
            next_process = next;
        } else {
            return false;
        }

        drop(preempt_lock);
    } else {
        return false;
    }

    switch(next_process)
}

/// Do a scheduler-aware process switch - put current process back on run queue
/// if it's still running.
///
/// Returns true if `next_process` could be switched to. `false` if
/// `next_process` was not ready to run.
fn switch(next_process: RcProcess) -> bool {
    let current_process = process::current();

    let current_process_is_running;

    {
        struct Info {
            id: process::Id,
            running: bool
        }

        fn extract(process: &RcProcess) -> Info {
            let p = process.lock();
            Info { id: p.id(), running: p.is_running() }
        }

        let current_process = extract(&current_process);
        let next_process = extract(&next_process);

        current_process_is_running = current_process.running;

        // Can't switch to non-running process
        if !next_process.running { return false; }

        // Same process, no switch
        if current_process.id == next_process.id { return true; }
    }

    if current_process_is_running {
        push(current_process);
    }

    process::switch_to(next_process);

    true
}

/// C interface. See `kit/kernel/include/scheduler.h`.
pub mod ffi {
    use crate::process;
    use crate::c_ffi::{c_int, uint32_t};

    #[no_mangle]
    pub extern fn scheduler_wake(pid: uint32_t) -> c_int {
        if let Some(process) = process::by_id(pid) {
            if super::awaken(process).is_ok() {
                return 1;
            } else {
                return 0;
            }
        } else {
            return 0;
        }
    }

    #[no_mangle]
    pub unsafe extern fn scheduler_sleep() {
        process::sleep();
    }

    #[no_mangle]
    pub unsafe extern fn scheduler_initialized() -> c_int {
        super::initialized() as c_int
    }

    #[no_mangle]
    pub unsafe extern fn scheduler_yield() {
        super::r#yield();
    }

    #[no_mangle]
    pub unsafe extern fn scheduler_preempt() -> c_int {
        super::preempt() as c_int
    }
}
