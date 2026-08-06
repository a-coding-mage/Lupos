// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/uts.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S015284

pub const UTS_SYSNAME: [u8; 6] = *b"Linux\0";
pub const UTS_NODENAME: [u8; 7] = *b"(none)\0";
pub const UTS_DOMAINNAME: [u8; 7] = *b"(none)\0";
