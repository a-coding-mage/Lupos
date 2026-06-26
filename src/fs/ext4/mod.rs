//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4
//! ext4 filesystem support.
//!
//! Mirrors `vendor/linux/fs/ext4/`.  Lupos M45 lands the format parsers and
//! a working mount: superblock + block-group descriptors + inode tables +
//! extent tree + linear directories, plus the release boot write path needed
//! by remount/fsck gates. Metadata writes route through the local JBD2
//! transaction shim before hitting the block device.

pub mod balloc;
pub mod dir;
pub mod extents;
pub mod ialloc;
pub mod indirect;
pub mod inline;
pub mod inode;
pub mod metadata;
pub mod ops;
pub mod super_block;
pub mod xattr_hurd;
pub mod xattr_security;
pub mod xattr_trusted;
pub mod xattr_user;

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::AtomicU64;
use spin::Mutex;

use crate::block::{block_device::BlockDeviceRef, lookup_block_device};
use crate::fs::dcache::d_alloc;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::{EINVAL, ENODEV};

pub const EXT4_SUPER_MAGIC: u16 = 0xEF53;
pub const EXT4_BLOCK_SIZE_DEFAULT: u32 = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ext4XattrListGate {
    MountOptionXattrUser,
    CapSysAdmin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4XattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub index: u8,
    pub list_function: &'static str,
    pub get_function: &'static str,
    pub set_function: &'static str,
    pub list_gate: Ext4XattrListGate,
}

/// In-memory ext4 super_block payload, stashed in `SuperBlock` via Arc.
pub struct Ext4Sbi {
    pub bdev: BlockDeviceRef,
    pub fs_uuid: [u8; 16],
    pub block_size: u32,
    pub blocks_per_group: u32,
    pub inodes_per_group: u32,
    pub first_ino: u32,
    pub inode_size: u32,
    pub want_extra_isize: u16,
    pub feature_compat: u32,
    pub feature_incompat: u32,
    pub feature_ro_compat: u32,
    pub inodes_count: u64,
    pub blocks_count: u64,
    pub group_desc_size: u32,
    pub group_descs: Vec<balloc::Ext4GroupDesc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext4BlockReservation {
    pub start: u64,
    pub count: u16,
}

/// Filesystem-private inode payload — the on-disk-plus-cached fields.
pub struct Ext4Inode {
    pub ino: u32,
    pub i_mode: u16,
    pub i_size: AtomicU64,
    pub i_blocks: AtomicU64,
    pub raw: Mutex<inode::OnDiskInode>,
    pub dir_cache: Mutex<Option<Vec<dir::DirEntry>>>,
    pub append_reservation: Mutex<Option<Ext4BlockReservation>>,
}

pub fn mount(source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let bdev = lookup_block_device(source).ok_or(ENODEV)?;
    let sbi = match super_block::read_super(&bdev) {
        Ok(sbi) => sbi,
        Err(err) => {
            crate::log_warn!(
                "ext4",
                "EXT4-fs ({}): unable to read superblock: {}",
                source,
                err
            );
            return Err(err);
        }
    };
    let fs_uuid = sbi.fs_uuid;
    let sbi_arc = Arc::new(sbi);
    let sb = SuperBlock::alloc("ext4", EXT4_SUPER_MAGIC as u64, &ops::EXT4_SUPER_OPS);
    sb.set_uuid(fs_uuid);
    // Stash sbi as Opaque pointer in next_ino-adjacent state via super_block private.
    // The simplest place to put it for our scale is a side-table keyed by sb id.
    super_block::stash_sbi(&sb, sbi_arc.clone())?;

    // Build root inode (ino = EXT4_ROOT_INO = 2).
    let root_inode = match inode::read_inode(&sbi_arc, 2, &sb) {
        Ok(inode) => inode,
        Err(err) => {
            crate::log_warn!(
                "ext4",
                "EXT4-fs ({}): unable to read root inode: {}",
                source,
                err
            );
            return Err(err);
        }
    };
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    let _ = EINVAL;
    Ok(sb)
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "ext4",
        mount,
        fs_flags: 0,
    });
}
