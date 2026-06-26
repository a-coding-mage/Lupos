//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/evm/evm_secfs.c
//! test-origin: linux:vendor/linux/security/integrity/evm/evm_secfs.c
//! EVM securityfs control surface.
//!
//! Linux exposes `<securityfs>/integrity/evm/evm` as the userspace signal for
//! enabling EVM key modes, with a root-level `evm` symlink to that file. Lupos
//! publishes the same shape and loads the HMAC key from the Linux keyring's
//! encrypted `evm-key` entry before enabling HMAC mode. The HMAC calculator and
//! type-2 `security.evm` verifier and signature-xattr status handling live in
//! `evm`; asymmetric signature verification uses the `.evm` integrity keyring.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::EINVAL;
use crate::security::inode::{
    securityfs_create_dir, securityfs_create_file, securityfs_create_symlink,
};

use super::evm;

static EVM_FS_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_securityfs() {
    if EVM_FS_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let integrity = securityfs_create_dir("integrity", None);
    let evm_dir = securityfs_create_dir("evm", Some(&integrity));
    securityfs_create_file(
        "evm",
        0o660,
        Some(&evm_dir),
        Some(evm_key_show),
        Some(evm_key_store),
    );
    securityfs_create_symlink("evm", None, "integrity/evm/evm");
    securityfs_create_file(
        "evm_xattrs",
        0o440,
        Some(&evm_dir),
        Some(evm_xattrs_show),
        None,
    );
}

fn copy_text(buf: &mut [u8], text: &str) -> Result<usize, i32> {
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

fn evm_key_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = alloc::format!("{}", evm::readable_key_flags());
    copy_text(buf, &text)
}

fn evm_key_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let text = core::str::from_utf8(buf).map_err(|_| -EINVAL)?.trim();
    let value = text.parse::<u32>().map_err(|_| -EINVAL)?;
    evm::write_key_flags(value)?;
    Ok(buf.len())
}

fn evm_xattrs_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, &evm::enabled_xattrs_text())
}

#[cfg(test)]
pub fn reset_for_test() {
    EVM_FS_INITIALIZED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};

    fn show(node: &Arc<KernfsNode>) -> alloc::string::String {
        let KernfsKind::File { show, .. } = &node.kind else {
            panic!("not a file");
        };
        let mut buf = [0u8; 256];
        let n = (show.expect("show fn"))(node, &mut buf).expect("show ok");
        core::str::from_utf8(&buf[..n]).unwrap().into()
    }

    fn store(node: &Arc<KernfsNode>, bytes: &[u8]) -> Result<usize, i32> {
        let KernfsKind::File { store, .. } = &node.kind else {
            panic!("not a file");
        };
        (store.expect("store fn"))(node, bytes)
    }

    #[test]
    fn evm_securityfs_tree_exposes_key_control_and_xattrs() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        crate::security::inode::reset_for_test();
        evm::reset_for_test();
        reset_for_test();

        evm::init();

        let root = crate::security::inode::securityfs_root();
        let integrity = lookup(&root, "integrity").expect("integrity dir");
        let evm_dir = lookup(&integrity, "evm").expect("evm dir");
        let evm_file = lookup(&evm_dir, "evm").expect("evm file");
        assert_eq!(show(&evm_file), "0");
        assert_eq!(store(&evm_file, b"2\n"), Ok(2));
        assert_eq!(show(&evm_file), "2");

        let xattrs = show(&lookup(&evm_dir, "evm_xattrs").expect("evm_xattrs"));
        assert_eq!(xattrs, "security.capability\n");

        let link = lookup(&root, "evm").expect("root evm symlink");
        match &link.kind {
            KernfsKind::Symlink { target } => assert_eq!(target, "integrity/evm/evm"),
            _ => panic!("evm must be a securityfs symlink"),
        }
    }

    #[test]
    fn evm_securityfs_write_loads_hmac_key_from_encrypted_keyring() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        crate::security::keys::reset_for_test();
        crate::security::keys::init();
        crate::security::inode::reset_for_test();
        evm::reset_for_test();
        reset_for_test();

        let payload = [0x5au8; 32];
        let key = crate::security::keys::add_key("encrypted", evm::EVM_KEY_DESCRIPTION, &payload);
        assert!(key > 0);
        evm::init();

        let root = crate::security::inode::securityfs_root();
        let integrity = lookup(&root, "integrity").expect("integrity dir");
        let evm_dir = lookup(&integrity, "evm").expect("evm dir");
        let evm_file = lookup(&evm_dir, "evm").expect("evm file");

        assert_eq!(store(&evm_file, b"1\n"), Ok(2));
        assert_eq!(show(&evm_file), "1");
        assert!(evm::hmac_key_loaded());
        assert_eq!(evm::hmac_key_len(), payload.len());
        assert_eq!(
            crate::security::keys::read(key).expect("burned key"),
            alloc::vec![0u8; payload.len()]
        );
        assert_eq!(
            store(&evm_file, b"2\n"),
            Err(-crate::include::uapi::errno::EPERM)
        );
    }

    #[test]
    fn evm_and_ima_share_integrity_securityfs_dir() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        crate::security::inode::reset_for_test();
        crate::security::integrity::ima::reset_for_test();
        crate::security::integrity::ima_fs::reset_for_test();
        evm::reset_for_test();
        reset_for_test();

        crate::security::integrity::ima::init();
        evm::init();

        let root = crate::security::inode::securityfs_root();
        let integrity = lookup(&root, "integrity").expect("integrity dir");
        assert!(lookup(&integrity, "ima").is_some());
        assert!(lookup(&integrity, "evm").is_some());
    }
}
