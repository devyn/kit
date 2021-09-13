/*******************************************************************************
 *
 * kit/kernel/kernel.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 * Based on OSDev Bare Bones tutorial
 * http://wiki.osdev.org/Bare_Bones
 *
 ******************************************************************************/

//! The Kit kernel.

#![crate_name="kernel"]
#![crate_type="lib"]

#![feature(lang_items, asm, box_syntax, alloc_error_handler)]
#![feature(box_patterns, panic_info_message)]
#![feature(repr_simd, const_panic, inline_const)]
#![feature(maybe_uninit_slice, vec_spare_capacity)]

#![allow(improper_ctypes)]

#![cfg_attr(not(test), no_std)]

// These rust libs are specifically configured for Kit.
#[macro_use] extern crate alloc;
#[macro_use] extern crate static_assertions;
#[macro_use] extern crate log as log_crate;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[macro_use] pub mod sync;
#[macro_use] pub mod util;

pub mod log;
pub mod cmdline;
pub mod serial;
pub mod terminal;
pub mod framebuffer;
pub mod constants;
pub mod cpu;
pub mod multiboot;
pub mod memory;
pub mod interrupt;
pub mod paging;
pub mod keyboard;
pub mod archive;
pub mod process;
pub mod elf;
pub mod scheduler;
pub mod syscall;
pub mod c_ffi;
pub mod error;
pub mod ptr;

use terminal::*;
use cmdline::Cmdline;
use memory::InitMemoryMap;

use alloc::string::String;

/// Main kernel entry point.
#[no_mangle]
pub extern fn kernel_main() -> ! {

    let mb_info = unsafe { multiboot::get_info() };

    let cmdline_utf8 = unsafe { mb_info.cmdline() }
        .map(|cstr| String::from(cstr))
        .unwrap_or("".into());

    let cmdline = Cmdline::new(&cmdline_utf8);

    if !cmdline.iter().any(|p| p == ("serial", "disable")) {
        serial::com1().initialize().unwrap();
    }

    log::initialize(&cmdline);

    info!("BOOT: Hello, I'm Kit.");

    debug!("Kernel command line: {}", cmdline);
    debug!("Multiboot info: {:08X?}", mb_info);

    let mut init_memory_map = InitMemoryMap::default();

    if mb_info.flags & multiboot::info_flags::MEM_MAP != 0 {
        unsafe {
            init_memory_map.load_from_multiboot(&mb_info);
        }
    } else {
        panic!("Bootloader did not provide memory map!");
    }

    unsafe {
        memory::initialize(&init_memory_map);
        interrupt::initialize();
        paging::initialize(&init_memory_map);
        terminal::initialize(&mb_info);
    }

    console().reset().unwrap();
    console().set_color(Color::Red, Color::White).unwrap();

    console().write_str("+ Hello, I'm Kit.\n").unwrap();

    console().set_color(Color::White, Color::Red).unwrap();
    console().write_char('\n').unwrap();

    match mb_info.mem_sizes() {
        Some((lower, upper)) => {
            write!(console(),
                   "{:<20} {:<10} KiB\n\
                    {:<20} {:<10} KiB\n",
                   "Lower memory:", lower,
                   "Upper memory:", upper).unwrap();
        },
        None => {
            write!(console(),
                   "W: Bootloader did not provide valid memory information!")
                .unwrap();
        }
    }

    write!(console(), "{:<20} {}\n", "Kernel command line:", cmdline).unwrap();

    console().write_char('\n').unwrap();
    console().set_color(Color::LightGrey, Color::Black).unwrap();
    console().write_char('\n').unwrap();

    unsafe {
        memory::enable_large_heap();
    }

    // We have to wait until enable_large_heap() is called before we can
    // allocate the shadow buffer.
    console().set_double_buffer(true);

    debug!("Pre-keyboard checkpoint. Multiboot info: {:08X?}", mb_info);

    unsafe {
        keyboard::initialize().unwrap();
    }

    if mb_info.flags & multiboot::info_flags::MODS != 0 {
        unsafe {
            let mods = constants::translate_low_addr(mb_info.mods_addr)
                .expect("mods pointer outside low region");

            if !archive::initialize(mods, mb_info.mods_count) {
                panic!("Archive initialization failed. Are you sure \
                        system.kit was provided?");
            }
        }
    } else {
        panic!("Bootloader did not provide modules!");
    }

    unsafe {
        process::initialize();
    }

    let pid;

    {
        let init = cmdline.iter().find(|(key, _)| *key == "init")
            .map(|(_, value)| value);

        if let Some(init) = init {
            let init_cstring = c_ffi::cstring_from_str(init);

            pid = archive::utils::spawn(
                c_ffi::CStr::new(&init_cstring), &[&init_cstring]).unwrap();

            process::wait(pid).unwrap();
        } else {
            panic!("No initial program specified on kernel command line!");
        }
    }

    // In case init exits
    {
        let process = process::by_id(pid).unwrap();

        let name = process.lock().name();
        let exit_status = process.lock().exit_status();

        panic!("Initial process ({}:{}) exited with code {}",
            pid, name, exit_status.unwrap());
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic_handler(panic_info: &PanicInfo) -> ! {
    error!("Kernel panic: {}", panic_info);

    let _ = console().set_color(Color::White, Color::Red);

    let _ = write!(console(), "\nKernel panic");

    if let Some(location) = panic_info.location() {
        let _ = write!(console(), " in {}", location);
    }

    if let Some(message) = panic_info.message() {
        let _ = write!(console(), ": {}", message);
    }

    unsafe {
        loop { asm!("cli; hlt"); }
    }
}
