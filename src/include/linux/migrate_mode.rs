// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/migrate_mode.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014373

/// `MIGRATE_ASYNC` means never block.
///
/// In the current implementation, `MIGRATE_SYNC_LIGHT` allows blocking on
/// most operations but not `->writepage`, because the potential stall time is
/// too significant.
///
/// `MIGRATE_SYNC` blocks while migrating pages.
///
/// The declaration order and implicit zero-based discriminants match `enum
/// migrate_mode` in the Linux source.
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum migrate_mode {
    MIGRATE_ASYNC,
    MIGRATE_SYNC_LIGHT,
    MIGRATE_SYNC,
}

/// Identifies the caller's reason for migrating pages.
///
/// The declaration order and implicit zero-based discriminants match
/// `enum migrate_reason` in the Linux source.
#[repr(C)]
#[allow(non_camel_case_types)]
pub enum migrate_reason {
    MR_COMPACTION,
    MR_MEMORY_FAILURE,
    MR_MEMORY_HOTPLUG,
    MR_SYSCALL,
    MR_MEMPOLICY_MBIND,
    MR_NUMA_MISPLACED,
    MR_CONTIG_RANGE,
    MR_LONGTERM_PIN,
    MR_DEMOTION,
    MR_DAMON,
    MR_TYPES,
}
