// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/linux/audit_arch.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013482

/*
 * audit_arch.h -- Arch layer specific support for audit
 *
 * Copyright 2021 Red Hat Inc., Durham, North Carolina.
 * All Rights Reserved.
 *
 * Author: Richard Guy Briggs <rgb@redhat.com>
 */

pub type auditsc_class_t = i32;

pub const AUDITSC_NATIVE: auditsc_class_t = 0;
pub const AUDITSC_COMPAT: auditsc_class_t = 1;
pub const AUDITSC_OPEN: auditsc_class_t = 2;
pub const AUDITSC_OPENAT: auditsc_class_t = 3;
pub const AUDITSC_SOCKETCALL: auditsc_class_t = 4;
pub const AUDITSC_EXECVE: auditsc_class_t = 5;
pub const AUDITSC_OPENAT2: auditsc_class_t = 6;

pub const AUDITSC_NVALS: auditsc_class_t = 7;

unsafe extern "C" {
    pub fn audit_classify_compat_syscall(abi: i32, syscall: u32) -> i32;

    /* Only for compat system calls.  The zero-length arrays preserve C's
     * incomplete-array declarations: callers use their symbol addresses as
     * raw `*mut u32` pointers and never as Rust array or scalar storage. */
    pub static mut compat_write_class: [u32; 0];
    pub static mut compat_read_class: [u32; 0];
    pub static mut compat_dir_class: [u32; 0];
    pub static mut compat_chattr_class: [u32; 0];
    pub static mut compat_signal_class: [u32; 0];
}
