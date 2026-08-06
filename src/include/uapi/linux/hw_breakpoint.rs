// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hw_breakpoint.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016151

// Values from the anonymous C enum at include/uapi/linux/hw_breakpoint.h:5.
pub const HW_BREAKPOINT_LEN_1: i32 = 1;
pub const HW_BREAKPOINT_LEN_2: i32 = 2;
pub const HW_BREAKPOINT_LEN_3: i32 = 3;
pub const HW_BREAKPOINT_LEN_4: i32 = 4;
pub const HW_BREAKPOINT_LEN_5: i32 = 5;
pub const HW_BREAKPOINT_LEN_6: i32 = 6;
pub const HW_BREAKPOINT_LEN_7: i32 = 7;
pub const HW_BREAKPOINT_LEN_8: i32 = 8;

// Values from the anonymous C enum at include/uapi/linux/hw_breakpoint.h:16.
pub const HW_BREAKPOINT_EMPTY: i32 = 0;
pub const HW_BREAKPOINT_R: i32 = 1;
pub const HW_BREAKPOINT_W: i32 = 2;
pub const HW_BREAKPOINT_RW: i32 = HW_BREAKPOINT_R | HW_BREAKPOINT_W;
pub const HW_BREAKPOINT_X: i32 = 4;
pub const HW_BREAKPOINT_INVALID: i32 = HW_BREAKPOINT_RW | HW_BREAKPOINT_X;
