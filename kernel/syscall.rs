/*******************************************************************************
 *
 * kit/kernel/syscall.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use crate::process;
use crate::memory;
use crate::scheduler;
use crate::c_ffi::*;
use crate::terminal::console;

use core::slice;

static mut INITIALIZED: bool = false;

pub unsafe fn initialize() {
    // Setup the syscall table (can't be done at compile time)
    table_init();

    // FIXME: C
    extern {
        fn syscall_initialize();
    }

    syscall_initialize();

    INITIALIZED = true;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SyscallTableEntry(usize);

#[derive(Debug, Clone)]
#[repr(C)]
struct SyscallTable([SyscallTableEntry; SYSCALL_MAX + 1]);

#[export_name = "syscall_table_size"]
static TABLE_SIZE: usize = SYSCALL_MAX + 1;

#[export_name = "syscall_table"]
static mut TABLE: SyscallTable =
    SyscallTable([SyscallTableEntry(0); SYSCALL_MAX + 1]);

macro_rules! syscalls {
    ($table:ident; $table_init:ident;
     $($num:expr, $const:ident, $function:path);* $(;)*) => {
        $(
            pub const $const: usize = $num;
        )*

        unsafe fn $table_init() {
            $(
                $table.0[$num] = SyscallTableEntry($function as usize);
            )*
        }
    }
}

pub const SYSCALL_MAX: usize = 9;

syscalls!(TABLE; table_init;
    0, SYSCALL_EXIT, syscall_exit;
    1, SYSCALL_TWRITE, syscall_twrite;
    2, SYSCALL_KEY_GET, syscall_key_get;
    3, SYSCALL_YIELD, syscall_yield;
    4, SYSCALL_SLEEP, syscall_sleep;
    5, SYSCALL_SPAWN, syscall_spawn;
    6, SYSCALL_WAIT_PROCESS, syscall_wait_process;
    7, SYSCALL_ADJUST_HEAP, syscall_adjust_heap;
    8, SYSCALL_MMAP_ARCHIVE, syscall_mmap_archive;
    9, SYSCALL_DEBUG, syscall_debug;
);

pub extern fn syscall_exit(status: c_int) -> ! {
    process::exit(status as i32);
}

// FIXME: unsafe user ptr handling
#[no_mangle]
pub unsafe extern fn syscall_twrite(length: usize, buffer: *const u8) -> c_int {
    let bytes = slice::from_raw_parts(buffer, length);

    console().write_raw_bytes(bytes)
        .and_then(|_| console().flush())
        .map(|_| 0)
        .unwrap_or(-1)
}

extern {
    // FIXME: C
    // FIXME: unsafe user ptr handling
    pub fn syscall_key_get(event: *mut u8) -> c_int;
}

#[no_mangle]
pub extern fn syscall_yield() -> c_int {
    scheduler::r#yield();
    0
}

#[no_mangle]
pub extern fn syscall_sleep() -> c_int {
    process::sleep();
    0
}

// FIXME: unsafe user ptr handling
#[no_mangle]
pub unsafe extern fn syscall_spawn(
    file: *const c_char,
    argc: c_int,
    argv: *const *const c_char
) -> int64_t {
    crate::archive::ffi::archive_utils_spawn(file, argc, argv)
}

// FIXME: unsafe user ptr handling
#[no_mangle]
pub unsafe extern fn syscall_wait_process(
    id: process::Id,
    exit_status: *mut c_int
) -> c_int {
    process::ffi::process_wait_exit_status(id, exit_status)
}

#[no_mangle]
pub unsafe extern fn syscall_adjust_heap(amount: isize) -> *mut c_void {
    process::ffi::process_adjust_heap(amount as int64_t)
}

extern {
    // FIXME: C
    pub fn syscall_mmap_archive() -> *mut u8;
}

pub const SYSCALL_DEBUG_PRINT_PROCESSES: u32 = 1;
pub const SYSCALL_DEBUG_PRINT_ALLOCATOR_STATS: u32 = 2;
pub const SYSCALL_DEBUG_TEST_KERNEL_THREAD: u32 = 9001;

/// Interface not stable.
#[no_mangle]
#[allow(unused_variables)]
pub unsafe extern fn syscall_debug(operation: u32, argument: usize) -> i32 {
    match operation {
        SYSCALL_DEBUG_PRINT_PROCESSES => {
            process::debug_print_processes();
        },
        SYSCALL_DEBUG_PRINT_ALLOCATOR_STATS => {
            memory::debug_print_allocator_stats();
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
