// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/oom.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016320

//! Out-of-memory killer adjustment UAPI constants.

use core::ffi::c_int;

/// Disables OOM killing when assigned to `/proc/<pid>/oom_score_adj`.
pub const OOM_SCORE_ADJ_MIN: c_int = -1000;
pub const OOM_SCORE_ADJ_MAX: c_int = 1000;

/// Legacy `/proc/<pid>/oom_adj` value that protects a process from OOM killing.
pub const OOM_DISABLE: c_int = -17;
/// Inclusive legacy `/proc/<pid>/oom_adj` lower bound.
pub const OOM_ADJUST_MIN: c_int = -16;
pub const OOM_ADJUST_MAX: c_int = 15;
