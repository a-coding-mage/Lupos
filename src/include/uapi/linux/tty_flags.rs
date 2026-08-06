// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/tty_flags.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016428

//! UAPI flag definitions for the `async_struct`, `serial_struct`, and tty-port
//! flag fields.

use core::ffi::c_int;

// Bits [0, ASYNCB_LAST_USER] are userspace-defined, visible, and changeable.
pub const ASYNCB_HUP_NOTIFY: c_int = 0;
pub const ASYNCB_FOURPORT: c_int = 1;
pub const ASYNCB_SAK: c_int = 2;
pub const ASYNCB_SPLIT_TERMIOS: c_int = 3;
pub const ASYNCB_SPD_HI: c_int = 4;
pub const ASYNCB_SPD_VHI: c_int = 5;
pub const ASYNCB_SKIP_TEST: c_int = 6;
pub const ASYNCB_AUTO_IRQ: c_int = 7;
pub const ASYNCB_SESSION_LOCKOUT: c_int = 8;
pub const ASYNCB_PGRP_LOCKOUT: c_int = 9;
pub const ASYNCB_CALLOUT_NOHUP: c_int = 10;
pub const ASYNCB_HARDPPS_CD: c_int = 11;
pub const ASYNCB_SPD_SHI: c_int = 12;
pub const ASYNCB_LOW_LATENCY: c_int = 13;
pub const ASYNCB_BUGGY_UART: c_int = 14;
pub const ASYNCB_AUTOPROBE: c_int = 15;
pub const ASYNCB_MAGIC_MULTIPLIER: c_int = 16;
pub const ASYNCB_LAST_USER: c_int = 16;

// The 10 obsolete ASYNCB_* names in the source's `#ifndef __KERNEL__` block
// are deliberately absent: both frozen configurations translate the kernel
// surface, for which upstream does not define those names.

pub const ASYNC_HUP_NOTIFY: u32 = 1u32 << ASYNCB_HUP_NOTIFY;
// Upstream declares this macro outside the `__KERNEL__` guard as
// `(1U << ASYNCB_SUSPENDED)`; its UAPI expansion is `1U << 30`.  The operand
// macro itself is excluded from the frozen kernel surface.
pub const ASYNC_SUSPENDED: u32 = 1u32 << 30;
pub const ASYNC_FOURPORT: u32 = 1u32 << ASYNCB_FOURPORT;
pub const ASYNC_SAK: u32 = 1u32 << ASYNCB_SAK;
pub const ASYNC_SPLIT_TERMIOS: u32 = 1u32 << ASYNCB_SPLIT_TERMIOS;
pub const ASYNC_SPD_HI: u32 = 1u32 << ASYNCB_SPD_HI;
pub const ASYNC_SPD_VHI: u32 = 1u32 << ASYNCB_SPD_VHI;
pub const ASYNC_SKIP_TEST: u32 = 1u32 << ASYNCB_SKIP_TEST;
pub const ASYNC_AUTO_IRQ: u32 = 1u32 << ASYNCB_AUTO_IRQ;
pub const ASYNC_SESSION_LOCKOUT: u32 = 1u32 << ASYNCB_SESSION_LOCKOUT;
pub const ASYNC_PGRP_LOCKOUT: u32 = 1u32 << ASYNCB_PGRP_LOCKOUT;
pub const ASYNC_CALLOUT_NOHUP: u32 = 1u32 << ASYNCB_CALLOUT_NOHUP;
pub const ASYNC_HARDPPS_CD: u32 = 1u32 << ASYNCB_HARDPPS_CD;
pub const ASYNC_SPD_SHI: u32 = 1u32 << ASYNCB_SPD_SHI;
pub const ASYNC_LOW_LATENCY: u32 = 1u32 << ASYNCB_LOW_LATENCY;
pub const ASYNC_BUGGY_UART: u32 = 1u32 << ASYNCB_BUGGY_UART;
pub const ASYNC_AUTOPROBE: u32 = 1u32 << ASYNCB_AUTOPROBE;
pub const ASYNC_MAGIC_MULTIPLIER: u32 = 1u32 << ASYNCB_MAGIC_MULTIPLIER;

pub const ASYNC_FLAGS: u32 = (1u32 << (ASYNCB_LAST_USER + 1)) - 1;
pub const ASYNC_DEPRECATED: u32 = ASYNC_SPLIT_TERMIOS
    | ASYNC_SESSION_LOCKOUT
    | ASYNC_PGRP_LOCKOUT
    | ASYNC_CALLOUT_NOHUP
    | ASYNC_AUTOPROBE;
pub const ASYNC_USR_MASK: u32 = ASYNC_SPD_MASK | ASYNC_CALLOUT_NOHUP | ASYNC_LOW_LATENCY;
pub const ASYNC_SPD_CUST: u32 = ASYNC_SPD_HI | ASYNC_SPD_VHI;
pub const ASYNC_SPD_WARP: u32 = ASYNC_SPD_HI | ASYNC_SPD_SHI;
pub const ASYNC_SPD_MASK: u32 = ASYNC_SPD_HI | ASYNC_SPD_VHI | ASYNC_SPD_SHI;

// The nine obsolete ASYNC_* names in the source's second `#ifndef
// __KERNEL__` block are deliberately absent for the same frozen kernel
// surface.
