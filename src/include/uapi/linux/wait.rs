// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/wait.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016476

/// Do not block while waiting for a child state change.
pub const WNOHANG: i32 = 0x0000_0001;
/// Report stopped children.
pub const WUNTRACED: i32 = 0x0000_0002;
/// Alias for `WUNTRACED` used by `waitid`.
pub const WSTOPPED: i32 = WUNTRACED;
/// Report exited children.
pub const WEXITED: i32 = 0x0000_0004;
/// Report children continued by `SIGCONT`.
pub const WCONTINUED: i32 = 0x0000_0008;
/// Do not reap a child; only report its status.
pub const WNOWAIT: i32 = 0x0100_0000;

/// Do not wait on children of other threads in this thread group.
pub const __WNOTHREAD: i32 = 0x2000_0000;
/// Wait on all children regardless of their type.
pub const __WALL: i32 = 0x4000_0000;
/// Wait only on children whose exit signal is not `SIGCHLD`.
///
/// The C macro is the unsigned `int` literal `0x80000000`.  Selected Linux
/// wait paths store options in a signed `int`; `i32::MIN` preserves that
/// literal's 32-bit bit pattern at the Rust options boundary.
pub const __WCLONE: i32 = i32::MIN;

/// `waitid` selector: all children.
pub const P_ALL: i32 = 0;
/// `waitid` selector: a process ID.
pub const P_PID: i32 = 1;
/// `waitid` selector: a process group ID.
pub const P_PGID: i32 = 2;
/// `waitid` selector: a pid file descriptor.
pub const P_PIDFD: i32 = 3;
