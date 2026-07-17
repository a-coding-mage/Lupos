//! linux-parity: partial
//! linux-source: vendor/linux/fs/isofs
//! ISO9660 — read-only mount + lookup (M46).
//!
//! Mirrors `vendor/linux/fs/isofs/`.  Lupos M46 ships the Primary Volume
//! Descriptor (PVD) parser and a simple directory-record walker.  Joliet,
//! Rock Ridge, and zisofs are deferred to userspace.

pub mod dir;
pub mod extensions;
pub mod inode;
pub mod joliet;
pub mod ops;
pub mod volume;

extern crate alloc;

use alloc::sync::Arc;

use crate::block::{block_device::BlockDeviceRef, lookup_block_device};
use crate::fs::dcache::d_alloc;
use crate::fs::super_block::{FileSystemType, register_filesystem};
use crate::fs::types::{SuperBlock, SuperBlockRef};
use crate::include::uapi::errno::ENODEV;

pub const ISOFS_SUPER_MAGIC: u64 = 0x9660;
pub const ISOFS_SECTOR_SIZE: usize = 2048;

pub struct IsoSbi {
    pub bdev: BlockDeviceRef,
    pub root_extent: u32, // first sector of root dir
    pub root_size: u32,
}

pub fn mount(source: &str, _flags: u64, _data: &str) -> Result<SuperBlockRef, i32> {
    let bdev = lookup_block_device(source).ok_or(ENODEV)?;
    let pvd = volume::read_pvd(&bdev)?;
    let sbi = Arc::new(IsoSbi {
        bdev,
        root_extent: pvd.root_extent,
        root_size: pvd.root_size,
    });
    let sb = SuperBlock::alloc("iso9660", ISOFS_SUPER_MAGIC, &ops::ISO_SUPER_OPS);
    stash_sbi(&sb, sbi.clone())?;
    let root_inode = inode::root_inode(&sbi, &sb);
    let root_dentry = d_alloc("/");
    root_dentry.instantiate(root_inode);
    *sb.root.lock() = Some(root_dentry);
    Ok(sb)
}

pub fn stash_sbi(sb: &SuperBlockRef, sbi: Arc<IsoSbi>) -> Result<(), i32> {
    sb.set_fs_private(sbi);
    Ok(())
}
pub fn get_sbi(sb: &SuperBlockRef) -> Option<Arc<IsoSbi>> {
    sb.fs_private::<IsoSbi>()
}

pub fn register() {
    let _ = register_filesystem(FileSystemType {
        name: "iso9660",
        mount,
        fs_flags: 0,
    });
}
