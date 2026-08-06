// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/membarrier.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016241

//! `membarrier` system-call UAPI definitions.

use core::ffi::c_int;

// Copyright (c) 2010, 2015 Mathieu Desnoyers <mathieu.desnoyers@efficios.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

/// C `enum membarrier_cmd`, represented by its C `int` ABI.
pub type membarrier_cmd = c_int;

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

/// C `enum membarrier_cmd_flag`, represented by its C `int` ABI.
pub type membarrier_cmd_flag = c_int;

pub const MEMBARRIER_CMD_FLAG_CPU: membarrier_cmd_flag = 1 << 0;
