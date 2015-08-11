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

pub unsafe fn initialize() {
    extern {
        fn syscall_initialize();
    }

    syscall_initialize();
}
