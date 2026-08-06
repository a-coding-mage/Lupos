// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/sem_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014944

/// Opaque declaration corresponding to C's forward-declared
/// `struct sem_undo_list`.
///
/// The definition is private to the System V semaphore implementation.  This
/// header carries only a pointer to it, so its layout is intentionally not
/// defined here.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct sem_undo_list {
    _private: [u8; 0],
}

/// Per-task System V semaphore state.
///
/// `undo_list` is present for the frozen x86_64 and AArch64 configurations,
/// both of which enable `CONFIG_SYSVIPC`.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct sysv_sem {
    pub undo_list: *mut sem_undo_list,
}
