// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/mman-common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016011

//! Generic UAPI memory-mapping, locking, synchronization, and advice flags.
//!
//! Author: Michael S. Tsirkin <mst@mellanox.co.il>, Mellanox Technologies Ltd.
//! Based on: asm-xxx/mman.h

/// Page can be read.
pub const PROT_READ: i32 = 0x1;
/// Page can be written.
pub const PROT_WRITE: i32 = 0x2;
/// Page can be executed.
pub const PROT_EXEC: i32 = 0x4;
/// Page may be used for atomic operations.
pub const PROT_SEM: i32 = 0x8;
/// No page access.
pub const PROT_NONE: i32 = 0x0;
/// Extend an `mprotect` change to the start of a grows-down VMA.
pub const PROT_GROWSDOWN: i32 = 0x0100_0000;
/// Extend an `mprotect` change to the end of a grows-up VMA.
pub const PROT_GROWSUP: i32 = 0x0200_0000;

/// Mask for the mapping type.
pub const MAP_TYPE: i32 = 0x0f;
/// Interpret the mapping address exactly.
pub const MAP_FIXED: i32 = 0x10;
/// Create a mapping without a file.
pub const MAP_ANONYMOUS: i32 = 0x20;
/// Populate (prefault) page tables.
pub const MAP_POPULATE: i32 = 0x008000;
/// Do not block on I/O while populating.
pub const MAP_NONBLOCK: i32 = 0x010000;
/// Request an address suitable for a process or thread stack.
pub const MAP_STACK: i32 = 0x020000;
/// Create a huge-page mapping.
pub const MAP_HUGETLB: i32 = 0x040000;
/// Perform synchronous page faults for the mapping.
pub const MAP_SYNC: i32 = 0x080000;
/// Fixed mapping that does not unmap an underlying mapping.
pub const MAP_FIXED_NOREPLACE: i32 = 0x100000;
/// Anonymous mapping memory may be uninitialized.
pub const MAP_UNINITIALIZED: i32 = 0x0400_0000;

/// Lock pages after they fault in, without prefaulting.
pub const MLOCK_ONFAULT: i32 = 0x01;

/// Synchronize memory asynchronously.
pub const MS_ASYNC: i32 = 1;
/// Invalidate caches.
pub const MS_INVALIDATE: i32 = 2;
/// Synchronize memory synchronously.
pub const MS_SYNC: i32 = 4;

/// No special memory-advice treatment.
pub const MADV_NORMAL: i32 = 0;
/// Expect random page references.
pub const MADV_RANDOM: i32 = 1;
/// Expect sequential page references.
pub const MADV_SEQUENTIAL: i32 = 2;
/// The pages will be needed.
pub const MADV_WILLNEED: i32 = 3;
/// The pages are not needed.
pub const MADV_DONTNEED: i32 = 4;
/// Free pages only under memory pressure.
pub const MADV_FREE: i32 = 8;
/// Remove pages and their resources.
pub const MADV_REMOVE: i32 = 9;
/// Do not inherit across `fork`.
pub const MADV_DONTFORK: i32 = 10;
/// Inherit across `fork`.
pub const MADV_DOFORK: i32 = 11;
/// Poison a page for testing.
pub const MADV_HWPOISON: i32 = 100;
/// Soft-offline a page for testing.
pub const MADV_SOFT_OFFLINE: i32 = 101;
/// KSM may merge identical pages.
pub const MADV_MERGEABLE: i32 = 12;
/// KSM may not merge identical pages.
pub const MADV_UNMERGEABLE: i32 = 13;
/// Prefer huge-page backing.
pub const MADV_HUGEPAGE: i32 = 14;
/// Do not prefer huge-page backing.
pub const MADV_NOHUGEPAGE: i32 = 15;
/// Exclude the range from core dumps.
pub const MADV_DONTDUMP: i32 = 16;
/// Clear the `MADV_DONTDUMP` setting.
pub const MADV_DODUMP: i32 = 17;
/// Zero memory in the child at `fork`.
pub const MADV_WIPEONFORK: i32 = 18;
/// Undo `MADV_WIPEONFORK`.
pub const MADV_KEEPONFORK: i32 = 19;
/// Deactivate these pages.
pub const MADV_COLD: i32 = 20;
/// Reclaim these pages.
pub const MADV_PAGEOUT: i32 = 21;
/// Populate readable page tables.
pub const MADV_POPULATE_READ: i32 = 22;
/// Populate writable page tables.
pub const MADV_POPULATE_WRITE: i32 = 23;
/// Like `MADV_DONTNEED`, but also drop locked pages.
pub const MADV_DONTNEED_LOCKED: i32 = 24;
/// Synchronously collapse into a huge page.
pub const MADV_COLLAPSE: i32 = 25;
/// Install a range that raises a fatal signal when accessed.
pub const MADV_GUARD_INSTALL: i32 = 102;
/// Remove a guarded range.
pub const MADV_GUARD_REMOVE: i32 = 103;

/// Compatibility mapping flag.
pub const MAP_FILE: i32 = 0;

pub const PKEY_UNRESTRICTED: i32 = 0x0;
pub const PKEY_DISABLE_ACCESS: i32 = 0x1;
pub const PKEY_DISABLE_WRITE: i32 = 0x2;
pub const PKEY_ACCESS_MASK: i32 = PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE;
