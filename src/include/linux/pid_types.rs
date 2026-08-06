// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/pid_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014640

/// Kinds of PID references maintained by a task.
///
/// The ordered values are the C `enum pid_type` discriminants.  In
/// particular, `PIDTYPE_MAX` is both the final discriminant and the bound for
/// PID-type-indexed Linux arrays.
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum pid_type {
    PIDTYPE_PID = 0,
    PIDTYPE_TGID = 1,
    PIDTYPE_PGID = 2,
    PIDTYPE_SID = 3,
    PIDTYPE_MAX = 4,
}

/// Opaque declaration corresponding to C's forward-declared
/// `struct pid_namespace` in this header.
///
/// Its layout is deliberately not defined here: Linux callers that need its
/// fields include `pid_namespace.h`, while this declaration only permits the
/// address of `init_pid_ns` to be named.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct pid_namespace {
    _private: [u8; 0],
}

unsafe extern "C" {
    /// The initial PID namespace, defined by the PID namespace implementation.
    pub static mut init_pid_ns: pid_namespace;
}
