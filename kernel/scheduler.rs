/*******************************************************************************
 *
 * kit/kernel/scheduler.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Time and event based task scheduler.

use core::cell::RefCell;
use alloc::boxed::Box;
use collections::VecDeque;

use process::{self, RcProcess};
use interrupt;

struct GlobalState {
    run_queue: VecDeque<RcProcess>,
    entered: bool,
    sleeping: bool,
}

static mut GLOBAL_STATE: Option<*const RefCell<GlobalState>> = None;
static mut INITIALIZED: bool = false;

pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}

pub unsafe fn initialize() {
    if INITIALIZED {
        panic!("scheduler already initialized");
    }

    GLOBAL_STATE = Some(Box::into_raw(box RefCell::new(GlobalState {
        run_queue: VecDeque::new(),
        entered: false,
        sleeping: false,
    })));

    INITIALIZED = true;
}

fn global_state<'a>() -> &'a RefCell<GlobalState> {
    unsafe {
        GLOBAL_STATE.as_ref().and_then(|ptr| ptr.as_ref())
            .expect("Scheduler not initialized!")
    }
}

pub fn entered() -> bool {
    initialized() && global_state().borrow().entered
}

/// Enter the scheduler loop.
///
/// This function will not return until the last process dies.
///
/// # Panics
///
/// Panics if we are already in the scheduler loop.
pub unsafe fn enter() {
    assert!(!global_state().borrow().entered, "scheduler already entered");

    global_state().borrow_mut().entered = true;
    tick();
    global_state().borrow_mut().entered = false;
}

/// Exit the scheduler loop.
///
/// This function may not return unless the scheduler loop is entered again,
/// which probably won't happen.
///
/// # Panics
///
/// Panics unless called from within the scheduler loop.
pub unsafe fn exit() {
    assert!(entered(), "tried to exit scheduler loop from outside scheduler
                        loop");

    process::switch_to_noproc();
}

/// Pushes a process on to the end of the run queue.
///
/// # Panics
///
/// Panics if the process is not in the `Running` state.
pub fn push(process: RcProcess) {
    assert!(process.borrow().is_running());

    global_state().borrow_mut().run_queue.push_back(process);
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

    let state = process.borrow().state();

    match state {
        Running => Ok(false),
        Sleeping => {
            process.borrow_mut().awaken();
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
/// # Unsafety
///
/// Call stack must be reentrant. Don't call `tick()` if you can't guarantee
/// that the entire call chain back to the interrupt or system call handler
/// knows that you might call `tick()`.
///
/// # Panics
///
/// Panics if the scheduler loop has not been entered.
pub unsafe fn tick() {
    assert!(entered(), "tick() called outside scheduler loop");

    // If the scheduler is currently sleeping, don't do anything;
    // another instance of `tick()` is already waiting for an event and will
    // handle it soon.
    if global_state().borrow().sleeping {
        return;
    }

    let current_process = process::current();

    // If there's no current process (we are coming from the kernel), just
    // switch to the first process on the queue. If the queue is empty, do
    // nothing and return.
    if current_process.is_none() {
        let first_process = global_state().borrow_mut().run_queue.pop_front();

        if let Some(process) = first_process {
            process::switch_to(process);
        }

        return;
    }

    let current_process = current_process.unwrap();

    while global_state().borrow().run_queue.is_empty() {
        let current_process_is_running = current_process.borrow().is_running();

        if current_process_is_running {
            // We can just continue running the current process, since it has
            // more work to do, and no one else does.
            return;
        } else {
            // Wait for an interrupt. An interrupt handler may result in a
            // process being scheduled. This scheduler state is called
            // 'sleeping', not to be confused with (but similar to) a process's
            // 'sleeping' state.
            global_state().borrow_mut().sleeping = true;
            interrupt::wait();
            global_state().borrow_mut().sleeping = false;
        }
    }

    let next_process = global_state().borrow_mut().run_queue.pop_front()
        .expect("run queue is empty even though we just proved that it wasn't");

    // If the process we're about to execute is not Running (for example, it
    // changed while it was on the queue), discard it with a tail-recursive
    // call.
    let next_process_is_running = next_process.borrow().is_running();
    if !next_process_is_running {
        return tick();
    }

    if next_process != current_process {
        let current_process_is_running = current_process.borrow().is_running();

        if current_process_is_running {
            // The process we're leaving was running, so let's put the current
            // process on the queue.
            push(current_process);
        }

        process::switch_to(next_process);
    } else {
        // The next process on the queue is this process. Just return.
        return;
    }
}

/// C interface. See `kit/kernel/include/scheduler.h`.
pub mod ffi {
    use process;
    use c_ffi::{c_int, uint32_t};

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
        let current_process = process::current()
            .expect("tried to call scheduler_sleep() without a process");

        current_process.borrow_mut().sleep();

        super::tick();
    }

    #[no_mangle]
    pub unsafe extern fn scheduler_tick() {
        if super::entered() {
            super::tick();
        }
    }
}
