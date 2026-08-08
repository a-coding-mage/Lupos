// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/net/tcp_states.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S015666

pub const TCP_ESTABLISHED: i32 = 1;
pub const TCP_SYN_SENT: i32 = 2;
pub const TCP_SYN_RECV: i32 = 3;
pub const TCP_FIN_WAIT1: i32 = 4;
pub const TCP_FIN_WAIT2: i32 = 5;
pub const TCP_TIME_WAIT: i32 = 6;
pub const TCP_CLOSE: i32 = 7;
pub const TCP_CLOSE_WAIT: i32 = 8;
pub const TCP_LAST_ACK: i32 = 9;
pub const TCP_LISTEN: i32 = 10;
pub const TCP_CLOSING: i32 = 11;
pub const TCP_NEW_SYN_RECV: i32 = 12;
pub const TCP_BOUND_INACTIVE: i32 = 13;
pub const TCP_MAX_STATES: i32 = 14;

pub const TCP_STATE_MASK: i32 = 0xF;
pub const TCP_ACTION_FIN: i32 = 1 << TCP_CLOSE;

pub const TCPF_ESTABLISHED: i32 = 1 << TCP_ESTABLISHED;
pub const TCPF_SYN_SENT: i32 = 1 << TCP_SYN_SENT;
pub const TCPF_SYN_RECV: i32 = 1 << TCP_SYN_RECV;
pub const TCPF_FIN_WAIT1: i32 = 1 << TCP_FIN_WAIT1;
pub const TCPF_FIN_WAIT2: i32 = 1 << TCP_FIN_WAIT2;
pub const TCPF_TIME_WAIT: i32 = 1 << TCP_TIME_WAIT;
pub const TCPF_CLOSE: i32 = 1 << TCP_CLOSE;
pub const TCPF_CLOSE_WAIT: i32 = 1 << TCP_CLOSE_WAIT;
pub const TCPF_LAST_ACK: i32 = 1 << TCP_LAST_ACK;
pub const TCPF_LISTEN: i32 = 1 << TCP_LISTEN;
pub const TCPF_CLOSING: i32 = 1 << TCP_CLOSING;
pub const TCPF_NEW_SYN_RECV: i32 = 1 << TCP_NEW_SYN_RECV;
pub const TCPF_BOUND_INACTIVE: i32 = 1 << TCP_BOUND_INACTIVE;
