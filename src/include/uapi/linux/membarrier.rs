// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/membarrier.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016241

// Copyright (c) 2010, 2015 Mathieu Desnoyers <mathieu.desnoyers@efficios.com>

/// C-compatible underlying type of `enum membarrier_cmd`.
pub type membarrier_cmd = i32;

pub const MEMBARRIER_CMD_QUERY: membarrier_cmd = 0;
pub const MEMBARRIER_CMD_GLOBAL: membarrier_cmd = 1 << 0;
pub const MEMBARRIER_CMD_GLOBAL_EXPEDITED: membarrier_cmd = 1 << 1;
pub const MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED: membarrier_cmd = 1 << 2;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED: membarrier_cmd = 1 << 3;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED: membarrier_cmd = 1 << 4;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: membarrier_cmd = 1 << 5;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE: membarrier_cmd = 1 << 6;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED_RSEQ: membarrier_cmd = 1 << 7;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ: membarrier_cmd = 1 << 8;
pub const MEMBARRIER_CMD_GET_REGISTRATIONS: membarrier_cmd = 1 << 9;

/// Alias for header backward compatibility.
pub const MEMBARRIER_CMD_SHARED: membarrier_cmd = MEMBARRIER_CMD_GLOBAL;

/// C-compatible underlying type of `enum membarrier_cmd_flag`.
pub type membarrier_cmd_flag = i32;

pub const MEMBARRIER_CMD_FLAG_CPU: membarrier_cmd_flag = 1 << 0;
