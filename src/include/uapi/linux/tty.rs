// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/tty.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016427

//! UAPI line-discipline numbers.

use core::ffi::c_int;

pub const N_TTY: c_int = 0;
pub const N_SLIP: c_int = 1;
pub const N_MOUSE: c_int = 2;
pub const N_PPP: c_int = 3;
pub const N_STRIP: c_int = 4;
pub const N_AX25: c_int = 5;
pub const N_X25: c_int = 6;
pub const N_6PACK: c_int = 7;
pub const N_MASC: c_int = 8;
pub const N_R3964: c_int = 9;
pub const N_PROFIBUS_FDL: c_int = 10;
pub const N_IRDA: c_int = 11;
pub const N_SMSBLOCK: c_int = 12;
pub const N_HDLC: c_int = 13;
pub const N_SYNC_PPP: c_int = 14;
pub const N_HCI: c_int = 15;
pub const N_GIGASET_M101: c_int = 16;
pub const N_SLCAN: c_int = 17;
pub const N_PPS: c_int = 18;
pub const N_V253: c_int = 19;
pub const N_CAIF: c_int = 20;
pub const N_GSM0710: c_int = 21;
pub const N_TI_WL: c_int = 22;
pub const N_TRACESINK: c_int = 23;
pub const N_TRACEROUTER: c_int = 24;
pub const N_NCI: c_int = 25;
pub const N_SPEAKUP: c_int = 26;
pub const N_NULL: c_int = 27;
pub const N_MCTP: c_int = 28;
pub const N_DEVELOPMENT: c_int = 29;
pub const N_CAN327: c_int = 30;

/// One greater than the newest line-discipline number.
pub const NR_LDISCS: c_int = 31;
