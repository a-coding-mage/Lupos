// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/mman-common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016011

//! The C include guard `__ASM_GENERIC_MMAN_COMMON_H` maps to this Rust module
//! being declared once at its path.

/*
 * Author: Michael S. Tsirkin <mst@mellanox.co.il>, Mellanox Technologies Ltd.
 * Based on: asm-xxx/mman.h
 */

pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const PROT_EXEC: i32 = 0x4;
pub const PROT_SEM: i32 = 0x8;
pub const PROT_NONE: i32 = 0x0;
pub const PROT_GROWSDOWN: i32 = 0x01000000;
pub const PROT_GROWSUP: i32 = 0x02000000;

pub const MAP_TYPE: i32 = 0x0f;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANONYMOUS: i32 = 0x20;

pub const MAP_POPULATE: i32 = 0x008000;
pub const MAP_NONBLOCK: i32 = 0x010000;
pub const MAP_STACK: i32 = 0x020000;
pub const MAP_HUGETLB: i32 = 0x040000;
pub const MAP_SYNC: i32 = 0x080000;
pub const MAP_FIXED_NOREPLACE: i32 = 0x100000;
pub const MAP_UNINITIALIZED: i32 = 0x4000000;

pub const MLOCK_ONFAULT: i32 = 0x01;

pub const MS_ASYNC: i32 = 1;
pub const MS_INVALIDATE: i32 = 2;
pub const MS_SYNC: i32 = 4;

pub const MADV_NORMAL: i32 = 0;
pub const MADV_RANDOM: i32 = 1;
pub const MADV_SEQUENTIAL: i32 = 2;
pub const MADV_WILLNEED: i32 = 3;
pub const MADV_DONTNEED: i32 = 4;

pub const MADV_FREE: i32 = 8;
pub const MADV_REMOVE: i32 = 9;
pub const MADV_DONTFORK: i32 = 10;
pub const MADV_DOFORK: i32 = 11;
pub const MADV_HWPOISON: i32 = 100;
pub const MADV_SOFT_OFFLINE: i32 = 101;

pub const MADV_MERGEABLE: i32 = 12;
pub const MADV_UNMERGEABLE: i32 = 13;

pub const MADV_HUGEPAGE: i32 = 14;
pub const MADV_NOHUGEPAGE: i32 = 15;

pub const MADV_DONTDUMP: i32 = 16;
pub const MADV_DODUMP: i32 = 17;

pub const MADV_WIPEONFORK: i32 = 18;
pub const MADV_KEEPONFORK: i32 = 19;

pub const MADV_COLD: i32 = 20;
pub const MADV_PAGEOUT: i32 = 21;

pub const MADV_POPULATE_READ: i32 = 22;
pub const MADV_POPULATE_WRITE: i32 = 23;

pub const MADV_DONTNEED_LOCKED: i32 = 24;
pub const MADV_COLLAPSE: i32 = 25;

pub const MADV_GUARD_INSTALL: i32 = 102;
pub const MADV_GUARD_REMOVE: i32 = 103;

pub const MAP_FILE: i32 = 0;

pub const PKEY_UNRESTRICTED: i32 = 0x0;
pub const PKEY_DISABLE_ACCESS: i32 = 0x1;
pub const PKEY_DISABLE_WRITE: i32 = 0x2;
pub const PKEY_ACCESS_MASK: i32 = PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE;
