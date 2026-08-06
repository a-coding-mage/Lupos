// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/asm-generic/mcs_spinlock.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012557

use core::cell::UnsafeCell;
use core::ffi::c_int;

/// ABI node used by the MCS hand-off queue.
///
/// `next` is linked with Linux one-copy access operations, `locked` is
/// initialized by the waiter and released by its predecessor, and `count` is
/// the per-CPU nesting count used by qspinlock.  These fields must be reached
/// through the corresponding raw Linux memory-operation translation; this
/// declaration does not provide a Rust synchronization operation.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mcs_spinlock {
    pub next: UnsafeCell<*mut mcs_spinlock>,
    pub locked: UnsafeCell<c_int>,
    pub count: UnsafeCell<c_int>,
}

// SAFETY: Linux permits an MCS node to be owned by one CPU while a predecessor
// publishes `next` or releases `locked`.  Cross-CPU access is valid only via
// the matching one-copy/acquire/release operations; `UnsafeCell` prevents
// ordinary shared Rust references from asserting non-aliasing or immutability.
unsafe impl Send for mcs_spinlock {}

// SAFETY: The same Linux hand-off protocol permits shared visibility of a node.
// Callers must use its raw synchronization operations and must not create
// unsynchronized Rust references to the cells.
unsafe impl Sync for mcs_spinlock {}
