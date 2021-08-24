/*******************************************************************************
 *
 * kit/kernel/cpu.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use crate::process::target::HwState;

#[repr(C)]
pub struct CpuLocalData {
    /// 0x00 - a guaranteed valid place for a small amount of temporary data
    ///
    /// The syscall handler uses this in order to have a place to dump the user
    /// stack pointer
    scratch: [u64; 4],
    /// 0x20 - pointer to current process hardware state
    hwstate: *mut HwState,
    // In the future, every cpu needs its own IDT, GDT (incl. TSS)
}

/// The place for the boot CPU local data is compiled in.
#[no_mangle]
pub static mut CPU0_LOCAL_DATA: CpuLocalData = CpuLocalData {
    scratch: [0, 0, 0, 0],
    hwstate: 0 as *mut HwState,
};
