//! linux-parity: partial
//! linux-source: vendor/linux/fs/xattr.c
//! test-origin: linux:vendor/linux/fs/xattr.c
//! Generic extended attribute storage and integrity hook glue.
//!
//! Mirrors the syscall-visible validation from `fs/xattr.c` and wires the EVM
//! hook sequence from `security/integrity/evm/evm_main.c`:
//! `inode_setxattr`/`post_setxattr`, `inode_removexattr`/`post_removexattr`,
//! `inode_setattr`/`post_setattr`, and POSIX ACL post-update wrappers.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::include::uapi::errno::{E2BIG, EEXIST, EINVAL, ENODATA, ERANGE};
use crate::include::uapi::stat::S_IFMT;
use crate::kernel::capability::{CAP_SYS_ADMIN, capable};
use crate::security::integrity::evm::{
    self, EvmIntegrityStatus, EvmMetadata, EvmProtectedXattr, EvmXattrUpdate,
};

use super::attr::{ATTR_GID, ATTR_MODE, ATTR_UID, IAttr};
use super::types::{Inode, InodeKind, InodeRef};

pub const XATTR_CREATE: i32 = 0x1;
pub const XATTR_REPLACE: i32 = 0x2;
pub const XATTR_NAME_MAX: usize = 255;
pub const XATTR_SIZE_MAX: usize = 65_536;
pub const XATTR_LIST_MAX: usize = 65_536;

pub fn validate_name(name: &str) -> Result<(), i32> {
    if name.is_empty() || name.len() > XATTR_NAME_MAX {
        return Err(ERANGE);
    }
    Ok(())
}

fn validate_set_flags(flags: i32) -> Result<(), i32> {
    if flags & !(XATTR_CREATE | XATTR_REPLACE) != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

fn evm_errno<T>(result: Result<T, i32>) -> Result<T, i32> {
    result.map_err(i32::abs)
}

fn protected_xattrs_from_map<'a>(
    xattrs: &'a BTreeMap<String, Vec<u8>>,
) -> Vec<EvmProtectedXattr<'a>> {
    let mut protected = Vec::new();
    for configured in evm::DEFAULT_XATTRS {
        if let Some(value) = xattrs.get(configured.name) {
            protected.push(EvmProtectedXattr {
                name: configured.name,
                value,
            });
        }
    }
    protected
}

fn current_evm_status(inode: &Inode, xattrs: &BTreeMap<String, Vec<u8>>) -> EvmIntegrityStatus {
    if !evm::evm_key_loaded() || inode.kind != InodeKind::Regular {
        return EvmIntegrityStatus::Pass;
    }
    let protected = protected_xattrs_from_map(xattrs);
    evm::verify_hmac_xattr(
        EvmMetadata::from_inode(inode),
        xattrs.get(evm::EVM_XATTR_NAME).map(Vec::as_slice),
        &protected,
    )
}

fn apply_evm_update(xattrs: &mut BTreeMap<String, Vec<u8>>, update: Option<EvmXattrUpdate>) {
    match update {
        Some(EvmXattrUpdate::Set(value)) => {
            xattrs.insert(String::from(evm::EVM_XATTR_NAME), value.to_vec());
        }
        Some(EvmXattrUpdate::Remove) => {
            xattrs.remove(evm::EVM_XATTR_NAME);
        }
        None => {}
    }
}

pub fn set_inode_xattr(inode: &InodeRef, name: &str, value: &[u8], flags: i32) -> Result<(), i32> {
    validate_name(name)?;
    validate_set_flags(flags)?;
    if value.len() > XATTR_SIZE_MAX {
        return Err(E2BIG);
    }

    let metadata = EvmMetadata::from_inode(inode);
    let mut xattrs = inode.xattrs.lock();
    let existing = xattrs.get(name);
    if existing.is_some() && flags & XATTR_CREATE != 0 {
        return Err(EEXIST);
    }
    if existing.is_none() && flags & XATTR_REPLACE != 0 {
        return Err(ENODATA);
    }

    let value_changed = existing.map_or(true, |old| old.as_slice() != value);
    let current_status = current_evm_status(inode, &xattrs);
    evm_errno(evm::protect_setxattr(
        name,
        value,
        current_status,
        capable(CAP_SYS_ADMIN),
        false,
        value_changed,
    ))?;

    xattrs.insert(String::from(name), value.to_vec());
    let protected = protected_xattrs_from_map(&xattrs);
    if let Ok(update) = evm_errno(evm::post_setxattr_update(metadata, name, &protected, false)) {
        apply_evm_update(&mut xattrs, update);
    }
    Ok(())
}

pub fn get_inode_xattr(inode: &InodeRef, name: &str) -> Result<Vec<u8>, i32> {
    validate_name(name)?;
    inode.xattrs.lock().get(name).cloned().ok_or(ENODATA)
}

pub fn list_inode_xattrs(inode: &InodeRef) -> Result<Vec<u8>, i32> {
    let xattrs = inode.xattrs.lock();
    let mut out = Vec::new();
    for name in xattrs.keys() {
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        if out.len() > XATTR_LIST_MAX {
            return Err(E2BIG);
        }
    }
    Ok(out)
}

pub fn remove_inode_xattr(inode: &InodeRef, name: &str) -> Result<(), i32> {
    validate_name(name)?;
    let metadata = EvmMetadata::from_inode(inode);
    let mut xattrs = inode.xattrs.lock();
    if !xattrs.contains_key(name) {
        return Err(ENODATA);
    }

    let current_status = current_evm_status(inode, &xattrs);
    evm_errno(evm::protect_removexattr(
        name,
        current_status,
        capable(CAP_SYS_ADMIN),
        false,
    ))?;

    xattrs.remove(name);
    let protected = protected_xattrs_from_map(&xattrs);
    if let Ok(update) = evm_errno(evm::post_removexattr_update(metadata, name, &protected)) {
        apply_evm_update(&mut xattrs, update);
    }
    Ok(())
}

pub fn evm_set_acl_prepare(
    inode: &InodeRef,
    acl_name: &str,
    acl_changes_mode: bool,
) -> Result<(), i32> {
    if !evm::posix_xattr_acl(acl_name) {
        return Ok(());
    }
    let xattrs = inode.xattrs.lock();
    let current_status = current_evm_status(inode, &xattrs);
    evm_errno(evm::protect_set_acl(
        current_status,
        false,
        acl_changes_mode,
    ))
}

pub fn evm_remove_acl_prepare(inode: &InodeRef, acl_name: &str) -> Result<(), i32> {
    if !evm::posix_xattr_acl(acl_name) {
        return Ok(());
    }
    let xattrs = inode.xattrs.lock();
    let current_status = current_evm_status(inode, &xattrs);
    evm_errno(evm::protect_remove_acl(current_status, false))
}

pub fn evm_post_set_acl(
    inode: &InodeRef,
    acl_name: &str,
    unsupported_hmac_fs: bool,
) -> Result<(), i32> {
    let metadata = EvmMetadata::from_inode(inode);
    let mut xattrs = inode.xattrs.lock();
    let protected = protected_xattrs_from_map(&xattrs);
    if let Ok(update) = evm_errno(evm::post_set_acl_update(
        metadata,
        acl_name,
        &protected,
        unsupported_hmac_fs,
    )) {
        apply_evm_update(&mut xattrs, update);
    }
    Ok(())
}

pub fn evm_post_remove_acl(inode: &InodeRef, acl_name: &str) -> Result<(), i32> {
    let metadata = EvmMetadata::from_inode(inode);
    let mut xattrs = inode.xattrs.lock();
    let protected = protected_xattrs_from_map(&xattrs);
    if let Ok(update) = evm_errno(evm::post_remove_acl_update(metadata, acl_name, &protected)) {
        apply_evm_update(&mut xattrs, update);
    }
    Ok(())
}

fn setattr_metadata_changed(inode: &Inode, attr: &IAttr) -> bool {
    if attr.valid & ATTR_MODE != 0 {
        let new_mode = inode.kind.s_ifmt() | (attr.mode & !S_IFMT);
        if inode.mode.load(Ordering::Acquire) != new_mode {
            return true;
        }
    }
    if attr.valid & ATTR_UID != 0 && inode.uid.load(Ordering::Acquire) != attr.uid {
        return true;
    }
    if attr.valid & ATTR_GID != 0 && inode.gid.load(Ordering::Acquire) != attr.gid {
        return true;
    }
    false
}

pub fn evm_setattr_prepare(inode: &InodeRef, attr: &IAttr) -> Result<bool, i32> {
    if attr.valid & (ATTR_MODE | ATTR_UID | ATTR_GID) == 0 {
        return Ok(false);
    }
    let metadata_changed = setattr_metadata_changed(inode, attr);
    let xattrs = inode.xattrs.lock();
    let current_status = current_evm_status(inode, &xattrs);
    evm_errno(evm::protect_setattr(
        current_status,
        false,
        metadata_changed,
    ))?;
    Ok(metadata_changed)
}

pub fn evm_post_setattr(inode: &InodeRef, metadata_changed: bool) -> Result<(), i32> {
    let metadata = EvmMetadata::from_inode(inode);
    let mut xattrs = inode.xattrs.lock();
    let protected = protected_xattrs_from_map(&xattrs);
    if let Ok(update) = evm_errno(evm::post_setattr_update(
        metadata,
        &protected,
        false,
        metadata_changed,
    )) {
        apply_evm_update(&mut xattrs, update);
    }
    Ok(())
}

#[cfg(test)]
pub fn set_inode_xattr_raw_for_test(inode: &InodeRef, name: &str, value: &[u8]) {
    inode
        .xattrs
        .lock()
        .insert(String::from(name), value.to_vec());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::attr;
    use crate::fs::ops::{NOOP_FILE_OPS, NOOP_INODE_OPS};
    use crate::fs::types::{Inode, InodeKind, InodePrivate};
    use crate::security::integrity::evm::{EVM_INIT_HMAC, EVM_KEY_DESCRIPTION};
    use crate::security::lsm_list::TEST_LSM_LOCK;

    fn regular_inode() -> InodeRef {
        Inode::new(
            42,
            InodeKind::Regular,
            0o644,
            &NOOP_INODE_OPS,
            &NOOP_FILE_OPS,
            InodePrivate::RamBytes(spin::Mutex::new(Vec::new())),
        )
    }

    fn load_test_evm_key() {
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        evm::reset_for_test();
        assert!(
            crate::security::keys::add_key("encrypted", EVM_KEY_DESCRIPTION, &[0x5au8; 32],) > 0
        );
        evm::write_key_flags(EVM_INIT_HMAC).expect("load evm hmac key");
    }

    #[test]
    fn xattr_create_replace_flags_follow_linux_simple_xattr_rules() {
        let inode = regular_inode();
        set_inode_xattr(&inode, "user.demo", b"one", 0).unwrap();
        assert_eq!(
            set_inode_xattr(&inode, "user.demo", b"two", XATTR_CREATE),
            Err(EEXIST)
        );
        assert_eq!(
            set_inode_xattr(&inode, "user.missing", b"two", XATTR_REPLACE),
            Err(ENODATA)
        );
        assert_eq!(set_inode_xattr(&inode, "user.bad", b"x", 0x10), Err(EINVAL));

        set_inode_xattr(&inode, "user.demo", b"two", XATTR_REPLACE).unwrap();
        assert_eq!(get_inode_xattr(&inode, "user.demo").unwrap(), b"two");
    }

    #[test]
    fn evm_live_xattr_hooks_update_security_evm_after_protected_change() {
        let _guard = TEST_LSM_LOCK.lock();
        let inode = regular_inode();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        evm::reset_for_test();

        set_inode_xattr(&inode, "security.capability", b"cap-v1", 0).unwrap();
        assert!(
            crate::security::keys::add_key("encrypted", EVM_KEY_DESCRIPTION, &[0x5au8; 32],) > 0
        );
        evm::write_key_flags(EVM_INIT_HMAC).expect("load evm hmac key");
        let metadata = EvmMetadata::from_inode(&inode);
        let initial_hmac = evm::build_hmac_xattr(
            metadata,
            &[EvmProtectedXattr {
                name: "security.capability",
                value: b"cap-v1",
            }],
        )
        .unwrap();
        set_inode_xattr_raw_for_test(&inode, evm::EVM_XATTR_NAME, &initial_hmac);

        set_inode_xattr(&inode, "security.capability", b"cap-v2", 0).unwrap();
        let updated_hmac = get_inode_xattr(&inode, evm::EVM_XATTR_NAME).unwrap();
        assert_ne!(updated_hmac, initial_hmac);
        assert_eq!(
            evm::verify_hmac_xattr(
                EvmMetadata::from_inode(&inode),
                Some(&updated_hmac),
                &[EvmProtectedXattr {
                    name: "security.capability",
                    value: b"cap-v2",
                }],
            ),
            EvmIntegrityStatus::Pass
        );

        remove_inode_xattr(&inode, "security.capability").unwrap();
        assert_eq!(get_inode_xattr(&inode, evm::EVM_XATTR_NAME), Err(ENODATA));
    }

    #[test]
    fn evm_live_setattr_hook_updates_security_evm_after_mode_change() {
        let _guard = TEST_LSM_LOCK.lock();
        let inode = regular_inode();
        set_inode_xattr(&inode, "security.capability", b"cap-v1", 0).unwrap();
        load_test_evm_key();
        let initial_hmac = evm::build_hmac_xattr(
            EvmMetadata::from_inode(&inode),
            &[EvmProtectedXattr {
                name: "security.capability",
                value: b"cap-v1",
            }],
        )
        .unwrap();
        set_inode_xattr_raw_for_test(&inode, evm::EVM_XATTR_NAME, &initial_hmac);

        attr::notify_change(&inode, &attr::IAttr::mode(0o600), false).unwrap();
        let updated_hmac = get_inode_xattr(&inode, evm::EVM_XATTR_NAME).unwrap();
        assert_ne!(updated_hmac, initial_hmac);
        assert_eq!(
            evm::verify_hmac_xattr(
                EvmMetadata::from_inode(&inode),
                Some(&updated_hmac),
                &[EvmProtectedXattr {
                    name: "security.capability",
                    value: b"cap-v1",
                }],
            ),
            EvmIntegrityStatus::Pass
        );
    }
}
