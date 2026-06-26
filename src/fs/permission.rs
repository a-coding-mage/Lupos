//! linux-parity: partial
//! linux-source: vendor/linux/fs/namei.c
//! Minimal VFS permission helpers for regular file writes.
//!
//! This keeps filesystem write implementations from becoming their own access
//! control boundary: writable opens and write(2) both funnel through these
//! checks before an inode's `file_operations::write` can modify backing storage.

use core::sync::atomic::Ordering;

use crate::include::uapi::errno::{EACCES, EROFS};
use crate::kernel::capability::{CAP_DAC_OVERRIDE, CAP_FOWNER, capable};
use crate::kernel::cred::{KGid, current_cred};
use crate::kernel::groups::in_group_p;
use crate::security::security_inode_permission;

use super::types::{DentryRef, InodeKind, InodeRef};

pub const MAY_WRITE: u32 = 0x0000_0002;

#[inline]
fn inode_write_bit_allows(inode: &InodeRef) -> bool {
    if capable(CAP_DAC_OVERRIDE) {
        return true;
    }

    let cred = current_cred();
    if cred.is_null() {
        return false;
    }

    let mode = inode.mode.load(Ordering::Acquire);
    let uid = inode.uid.load(Ordering::Acquire);
    let gid = inode.gid.load(Ordering::Acquire);
    let write_bits = unsafe {
        if (*cred).fsuid.0 == uid {
            (mode >> 6) & 0o2
        } else if in_group_p(KGid(gid)) {
            (mode >> 3) & 0o2
        } else {
            mode & 0o2
        }
    };

    write_bits != 0
}

#[inline]
pub fn inode_owner_or_capable(inode: &InodeRef) -> bool {
    if capable(CAP_FOWNER) {
        return true;
    }

    let cred = current_cred();
    if cred.is_null() {
        return false;
    }

    unsafe { (*cred).fsuid.0 == inode.uid.load(Ordering::Acquire) }
}

pub fn check_inode_write_permission(inode: &InodeRef) -> Result<(), i32> {
    if !inode_write_bit_allows(inode) {
        return Err(EACCES);
    }

    let lsm = security_inode_permission(inode.ino, MAY_WRITE);
    if lsm != 0 {
        return Err(if lsm < 0 { -lsm } else { lsm });
    }

    Ok(())
}

pub fn check_file_write_permission(dentry: &DentryRef, inode: &InodeRef) -> Result<(), i32> {
    check_file_write_mount(dentry, inode.kind)?;
    check_inode_write_permission(inode)
}

pub fn check_file_write_mount(dentry: &DentryRef, kind: InodeKind) -> Result<(), i32> {
    if !mount_readonly_blocks_write(kind) {
        return Ok(());
    }
    if super::mount::containing_mount_for_dentry(dentry).is_some_and(|mount| mount.is_readonly()) {
        return Err(EROFS);
    }

    Ok(())
}

fn mount_readonly_blocks_write(kind: InodeKind) -> bool {
    matches!(
        kind,
        InodeKind::Regular | InodeKind::Directory | InodeKind::Symlink
    )
}
