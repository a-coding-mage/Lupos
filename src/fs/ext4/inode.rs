//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext4/inode.c
//! test-origin: linux:vendor/linux/fs/ext4/inode.c
//! ext4 on-disk inode parsing + read_inode().
//!
//! Mirrors `vendor/linux/fs/ext4/ext4.h::struct ext4_inode`.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::fs::types::{Inode, InodeKind, InodePrivate, InodeRef, SuperBlockRef};
use crate::include::uapi::errno::EINVAL;
use crate::include::uapi::stat::*;

use super::Ext4Sbi;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct OnDiskInode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size_lo: u32,
    pub i_atime: u32,
    pub i_ctime: u32,
    pub i_mtime: u32,
    pub i_dtime: u32,
    pub i_gid: u16,
    pub i_links_count: u16,
    pub i_blocks_lo: u32,
    pub i_flags: u32,
    pub _osd1: u32,
    pub i_block: [u32; 15], // direct/indirect block indices OR extent header
    pub i_generation: u32,
    pub i_file_acl_lo: u32,
    pub i_size_hi: u32,
    pub i_obso_faddr: u32,
    pub _osd2: [u8; 12],
    pub i_extra_isize: u16,
    pub i_checksum_hi: u16,
    pub i_ctime_extra: u32,
    pub i_mtime_extra: u32,
    pub i_atime_extra: u32,
    pub i_crtime: u32,
    pub i_crtime_extra: u32,
    pub i_version_hi: u32,
    pub i_projid: u32,
}

const EXT4_EXTENTS_FL: u32 = 0x80000;
pub const EXT4_INLINE_DATA_FL: u32 = 0x10000000;

pub fn read_inode(sbi: &Ext4Sbi, ino: u32, sb: &SuperBlockRef) -> Result<InodeRef, i32> {
    let raw = super::ialloc::read_raw_inode(sbi, ino)?;
    if raw.len() < core::mem::size_of::<OnDiskInode>() {
        return Err(EINVAL);
    }
    let on_disk: OnDiskInode =
        unsafe { core::ptr::read_unaligned(raw.as_ptr() as *const OnDiskInode) };

    let i_mode = u16::from_le(on_disk.i_mode);
    let i_size =
        (u32::from_le(on_disk.i_size_lo) as u64) | ((u32::from_le(on_disk.i_size_hi) as u64) << 32);
    let i_blocks = u32::from_le(on_disk.i_blocks_lo) as u64;
    let i_flags = u32::from_le(on_disk.i_flags);
    let i_links = u16::from_le(on_disk.i_links_count) as u32;

    let kind = match (i_mode as u32) & S_IFMT {
        S_IFREG => InodeKind::Regular,
        S_IFDIR => InodeKind::Directory,
        S_IFLNK => InodeKind::Symlink,
        S_IFCHR => InodeKind::Chardev,
        S_IFBLK => InodeKind::Blockdev,
        S_IFIFO => InodeKind::Fifo,
        S_IFSOCK => InodeKind::Socket,
        _ => InodeKind::Regular,
    };

    let priv_payload = Arc::new(super::Ext4Inode {
        ino,
        i_mode,
        i_size: AtomicU64::new(i_size),
        i_blocks: AtomicU64::new(i_blocks),
        raw: Mutex::new(on_disk),
        dir_cache: Mutex::new(None),
        append_reservation: Mutex::new(None),
    });
    let opaque = Arc::into_raw(priv_payload) as usize;

    let i = Inode::new(
        ino as u64,
        kind,
        i_mode as u32,
        match kind {
            InodeKind::Directory => &super::ops::EXT4_DIR_INODE_OPS,
            InodeKind::Symlink => &super::ops::EXT4_SYMLINK_INODE_OPS,
            _ => &super::ops::EXT4_FILE_INODE_OPS,
        },
        match kind {
            InodeKind::Directory => &super::ops::EXT4_DIR_FILE_OPS,
            _ => &super::ops::EXT4_FILE_FILE_OPS,
        },
        InodePrivate::Opaque(opaque),
    );
    *i.sb.lock() = Some(sb.clone());
    i.size.store(i_size, Ordering::Release);
    i.nlink.store(i_links, Ordering::Release);
    i.uid
        .store(u16::from_le(on_disk.i_uid) as u32, Ordering::Release);
    i.gid
        .store(u16::from_le(on_disk.i_gid) as u32, Ordering::Release);
    let _ = (EXT4_EXTENTS_FL, i_flags);
    Ok(i)
}

/// Recover the `Ext4Inode` Arc from an Inode's private payload.  Bumps the
/// strong count so the caller can drop normally.
pub fn ext4_inode_of(inode: &InodeRef) -> Option<Arc<super::Ext4Inode>> {
    if let InodePrivate::Opaque(p) = &inode.private {
        let raw = *p as *const super::Ext4Inode;
        unsafe {
            Arc::increment_strong_count(raw);
            Some(Arc::from_raw(raw))
        }
    } else {
        None
    }
}

pub fn uses_extents(ext4_inode: &super::Ext4Inode) -> bool {
    u32::from_le(ext4_inode.raw.lock().i_flags) & EXT4_EXTENTS_FL != 0
}

pub fn uses_inline(ext4_inode: &super::Ext4Inode) -> bool {
    u32::from_le(ext4_inode.raw.lock().i_flags) & EXT4_INLINE_DATA_FL != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::block::block_device::BlockDevice;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
    use crate::fs::ext4::balloc::Ext4GroupDesc;
    use crate::fs::ext4::{EXT4_SUPER_MAGIC, Ext4Sbi};
    use crate::fs::types::SuperBlock;

    #[test]
    fn read_inode_assigns_ext4_symlink_ops() {
        let target = b"/usr/lib/systemd/system/multi-user.target";
        let mem = MemBlockDevice::new("ext4-symlink-inode", 8192);
        let bdev = BlockDevice::wrap(mem.clone(), mem_block_device_ops());
        let inode_size = core::mem::size_of::<OnDiskInode>() as u32;
        let table_block = 4u64;
        let raw = fast_symlink_inode(target);
        let raw_bytes = unsafe {
            core::slice::from_raw_parts(
                (&raw as *const OnDiskInode).cast::<u8>(),
                core::mem::size_of::<OnDiskInode>(),
            )
        };
        {
            let mut data = mem.data.lock();
            let off = table_block as usize * 1024;
            data[off..off + raw_bytes.len()].copy_from_slice(raw_bytes);
        }

        let sbi = Ext4Sbi {
            bdev,
            fs_uuid: [0; 16],
            block_size: 1024,
            blocks_per_group: 64,
            inodes_per_group: 16,
            first_ino: 11,
            inode_size,
            want_extra_isize: 0,
            feature_compat: 0,
            feature_incompat: 0,
            feature_ro_compat: 0,
            inodes_count: 16,
            blocks_count: 64,
            group_desc_size: 32,
            group_descs: alloc::vec![Ext4GroupDesc {
                bg_block_bitmap: 0,
                bg_inode_bitmap: 0,
                bg_inode_table: table_block,
                bg_free_blocks_count: 0,
                bg_free_inodes_count: 0,
                bg_used_dirs_count: 0,
            }],
        };
        let sb = SuperBlock::alloc(
            "ext4",
            EXT4_SUPER_MAGIC as u64,
            &super::super::ops::EXT4_SUPER_OPS,
        );
        let inode = read_inode(&sbi, 1, &sb).expect("read symlink inode");

        assert_eq!(inode.kind, InodeKind::Symlink);
        assert!(core::ptr::eq(
            inode.ops,
            &super::super::ops::EXT4_SYMLINK_INODE_OPS
        ));
        let mut buf = [0u8; 128];
        let readlink = inode.ops.readlink.expect("symlink readlink op");
        let n = readlink(&inode, &mut buf).expect("readlink");
        assert_eq!(&buf[..n], target);
    }

    fn fast_symlink_inode(target: &[u8]) -> OnDiskInode {
        let mut i_block = [0u32; 15];
        let bytes =
            unsafe { core::slice::from_raw_parts_mut(i_block.as_mut_ptr().cast::<u8>(), 60) };
        bytes[..target.len()].copy_from_slice(target);

        OnDiskInode {
            i_mode: 0o120777u16.to_le(),
            i_uid: 0,
            i_size_lo: (target.len() as u32).to_le(),
            i_atime: 0,
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: 0,
            i_links_count: 1u16.to_le(),
            i_blocks_lo: 0,
            i_flags: 0,
            _osd1: 0,
            i_block,
            i_generation: 0,
            i_file_acl_lo: 0,
            i_size_hi: 0,
            i_obso_faddr: 0,
            _osd2: [0; 12],
            i_extra_isize: 0,
            i_checksum_hi: 0,
            i_ctime_extra: 0,
            i_mtime_extra: 0,
            i_atime_extra: 0,
            i_crtime: 0,
            i_crtime_extra: 0,
            i_version_hi: 0,
            i_projid: 0,
        }
    }
}
