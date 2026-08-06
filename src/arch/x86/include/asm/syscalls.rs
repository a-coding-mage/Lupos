// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/syscalls.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000713

use core::ffi::{c_int, c_long, c_ulong};

unsafe extern "C" {
    pub fn ksys_ioperm(from: c_ulong, num: c_ulong, turn_on: c_int) -> c_long;
}
