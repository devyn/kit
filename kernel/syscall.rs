/*******************************************************************************
 *
 * kit/kernel/syscall.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use crate::process;
use crate::scheduler;

pub unsafe fn initialize() {
    extern {
        fn syscall_initialize();
    }

    syscall_initialize();
}

pub const SYSCALL_DEBUG_PRINT_PROCESSES: u32 = 1;
pub const SYSCALL_DEBUG_TEST_KERNEL_THREAD: u32 = 9001;

/// Interface not stable.
#[no_mangle]
#[allow(unused_variables)]
pub unsafe extern fn syscall_debug(operation: u32, argument: usize) -> i32 {
    match operation {
        SYSCALL_DEBUG_PRINT_PROCESSES => {
            process::debug_print_processes();
        },
        SYSCALL_DEBUG_TEST_KERNEL_THREAD => {
            let name = format!("TEST_KERNEL_THREAD-{}", argument);

            process::spawn_kthread(name, move || {
                for i in 0..argument {
                    debug!("TEST_KERNEL_THREAD-{0} alive i={1}/{0}", argument, i);
                    for _ in 0..100000 { scheduler::r#yield(); }
                }
            });
        },
        _ => { return -1; }
    }

    0
}
