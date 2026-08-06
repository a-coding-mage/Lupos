// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/net_namespace.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016264

// Copyright (c) 2015 6WIND S.A.
// Author: Nicolas Dichtel <nicolas.dichtel@6wind.com>
//
// This program is free software; you can redistribute it and/or modify it
// under the terms and conditions of the GNU General Public License,
// version 2, as published by the Free Software Foundation.

/// Attributes of RTM_NEWNSID/RTM_GETNSID messages.
pub const NETNSA_NONE: i32 = 0;
pub const NETNSA_NSID_NOT_ASSIGNED: i32 = -1;
pub const NETNSA_NSID: i32 = 1;
pub const NETNSA_PID: i32 = 2;
pub const NETNSA_FD: i32 = 3;
pub const NETNSA_TARGET_NSID: i32 = 4;
pub const NETNSA_CURRENT_NSID: i32 = 5;
pub const __NETNSA_MAX: i32 = 6;

pub const NETNSA_MAX: i32 = __NETNSA_MAX - 1;
