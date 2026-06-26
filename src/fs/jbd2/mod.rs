//! linux-parity: partial
//! linux-source: vendor/linux/fs/jbd2
//! jbd2 (Journaling Block Device, v2).
//!
//! Mirrors `vendor/linux/fs/jbd2/`.  This module covers the transaction,
//! metadata dirtying, commit writeout, revoke, checkpoint, and payload replay
//! surface needed by the ext4 write/fsck boot gates.

pub mod journal;
pub mod recovery;
pub mod transaction;

pub const JBD2_MAGIC_NUMBER: u32 = 0xc03b3998;

// Journal block types (`jbd2_journal_header.h_blocktype`).
pub const JBD2_DESCRIPTOR_BLOCK: u32 = 1;
pub const JBD2_COMMIT_BLOCK: u32 = 2;
pub const JBD2_SUPERBLOCK_V1: u32 = 3;
pub const JBD2_SUPERBLOCK_V2: u32 = 4;
pub const JBD2_REVOKE_BLOCK: u32 = 5;

// Descriptor-block tag flags (`journal_block_tag.t_flags`).
pub const JBD2_FLAG_ESCAPE: u32 = 1; // on-disk block is escaped
pub const JBD2_FLAG_SAME_UUID: u32 = 2; // block has same uuid as previous
pub const JBD2_FLAG_DELETED: u32 = 4; // block deleted by this transaction
pub const JBD2_FLAG_LAST_TAG: u32 = 8; // last tag in this descriptor block

// On-disk feature flags (`journal_superblock.s_feature_{compat,incompat,ro_compat}`).
pub const JBD2_FEATURE_COMPAT_CHECKSUM: u32 = 0x00000001;
pub const JBD2_FEATURE_INCOMPAT_REVOKE: u32 = 0x00000001;
pub const JBD2_FEATURE_INCOMPAT_64BIT: u32 = 0x00000002;
pub const JBD2_FEATURE_INCOMPAT_ASYNC_COMMIT: u32 = 0x00000004;
pub const JBD2_FEATURE_INCOMPAT_CSUM_V2: u32 = 0x00000008;
pub const JBD2_FEATURE_INCOMPAT_CSUM_V3: u32 = 0x00000010;
pub const JBD2_FEATURE_INCOMPAT_FAST_COMMIT: u32 = 0x00000020;
