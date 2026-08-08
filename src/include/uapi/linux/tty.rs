// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/tty.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016427

// 'tty.h' defines some structures used by tty_io.c and some defines.

// line disciplines
pub const N_TTY: i32 = 0;
pub const N_SLIP: i32 = 1;
pub const N_MOUSE: i32 = 2;
pub const N_PPP: i32 = 3;
pub const N_STRIP: i32 = 4;
pub const N_AX25: i32 = 5;
pub const N_X25: i32 = 6; // X.25 async
pub const N_6PACK: i32 = 7;
pub const N_MASC: i32 = 8; // Reserved for Mobitex module <kaz@cafe.net>
pub const N_R3964: i32 = 9; // Reserved for Simatic R3964 module
pub const N_PROFIBUS_FDL: i32 = 10; // Reserved for Profibus
pub const N_IRDA: i32 = 11; // Linux IrDa - http://irda.sourceforge.net/
pub const N_SMSBLOCK: i32 = 12; // SMS block mode - for talking to GSM data
                                 // cards about SMS messages
pub const N_HDLC: i32 = 13; // synchronous HDLC
pub const N_SYNC_PPP: i32 = 14; // synchronous PPP
pub const N_HCI: i32 = 15; // Bluetooth HCI UART
pub const N_GIGASET_M101: i32 = 16; // Siemens Gigaset M101 serial DECT adapter
pub const N_SLCAN: i32 = 17; // Serial / USB serial CAN Adaptors
pub const N_PPS: i32 = 18; // Pulse per Second
pub const N_V253: i32 = 19; // Codec control over voice modem
pub const N_CAIF: i32 = 20; // CAIF protocol for talking to modems
pub const N_GSM0710: i32 = 21; // GSM 0710 Mux
pub const N_TI_WL: i32 = 22; // for TI's WL BT, FM, GPS combo chips
pub const N_TRACESINK: i32 = 23; // Trace data routing for MIPI P1149.7
pub const N_TRACEROUTER: i32 = 24; // Trace data routing for MIPI P1149.7
pub const N_NCI: i32 = 25; // NFC NCI UART
pub const N_SPEAKUP: i32 = 26; // Speakup communication with synths
pub const N_NULL: i32 = 27; // Null ldisc used for error handling
pub const N_MCTP: i32 = 28; // MCTP-over-serial
pub const N_DEVELOPMENT: i32 = 29; // Manual out-of-tree testing
pub const N_CAN327: i32 = 30; // ELM327 based OBD-II interfaces

// Always the newest line discipline + 1
pub const NR_LDISCS: i32 = 31;
