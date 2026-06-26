//! linux-parity: partial
//! linux-source: vendor/linux/security/integrity/ima/ima_fs.c
//! test-origin: linux:vendor/linux/security/integrity/ima/ima_fs.c
//! IMA securityfs reporting surface.
//!
//! Linux exposes the append-only measurement queue under
//! `<securityfs>/integrity/ima`. Lupos currently publishes the boot aggregate,
//! measured exec/images and counters, and accepts Linux-shaped measure/appraise
//! text policy through the `policy` file. Signature appraisal lives in `ima`.

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fs::kernfs::KernfsNode;
use crate::security::inode::{
    securityfs_create_dir, securityfs_create_file, securityfs_create_symlink,
};

use super::ima;

static IMA_FS_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_securityfs() {
    if IMA_FS_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let integrity = securityfs_create_dir("integrity", None);
    let ima_dir = securityfs_create_dir("ima", Some(&integrity));

    securityfs_create_symlink("ima", None, "integrity/ima");
    securityfs_create_file(
        "binary_runtime_measurements_sha1",
        0o440,
        Some(&ima_dir),
        Some(binary_runtime_measurements_show),
        None,
    );
    securityfs_create_file(
        "ascii_runtime_measurements_sha1",
        0o440,
        Some(&ima_dir),
        Some(ascii_runtime_measurements_show),
        None,
    );
    securityfs_create_symlink(
        "binary_runtime_measurements",
        Some(&ima_dir),
        "binary_runtime_measurements_sha1",
    );
    securityfs_create_symlink(
        "ascii_runtime_measurements",
        Some(&ima_dir),
        "ascii_runtime_measurements_sha1",
    );
    securityfs_create_file(
        "runtime_measurements_count",
        0o440,
        Some(&ima_dir),
        Some(runtime_measurements_count_show),
        None,
    );
    securityfs_create_file(
        "violations",
        0o440,
        Some(&ima_dir),
        Some(violations_show),
        None,
    );
    securityfs_create_file(
        "policy",
        0o600,
        Some(&ima_dir),
        Some(policy_show),
        Some(policy_store),
    );
}

fn copy_bytes(buf: &mut [u8], bytes: &[u8]) -> Result<usize, i32> {
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    Ok(n)
}

fn runtime_measurements_count_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = alloc::format!("{}\n", ima::runtime_measurements_count());
    copy_bytes(buf, text.as_bytes())
}

fn violations_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = alloc::format!("{}\n", ima::runtime_violations());
    copy_bytes(buf, text.as_bytes())
}

fn ascii_runtime_measurements_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = ima::ascii_runtime_measurements_sha1();
    copy_bytes(buf, text.as_bytes())
}

fn binary_runtime_measurements_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let bytes = ima::binary_runtime_measurements_sha1();
    copy_bytes(buf, &bytes)
}

fn policy_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = ima::policy_text();
    copy_bytes(buf, text.as_bytes())
}

fn policy_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    ima::load_policy(buf)
}

#[cfg(test)]
pub fn reset_for_test() {
    IMA_FS_INITIALIZED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};

    fn show(node: &Arc<KernfsNode>) -> alloc::string::String {
        let KernfsKind::File { show, .. } = &node.kind else {
            panic!("not a file");
        };
        let mut buf = [0u8; 512];
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
    fn ima_securityfs_tree_exposes_boot_aggregate_measurement() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        crate::security::inode::reset_for_test();
        ima::reset_for_test();
        reset_for_test();

        ima::init();

        let root = crate::security::inode::securityfs_root();
        let integrity = lookup(&root, "integrity").expect("integrity dir");
        let ima_dir = lookup(&integrity, "ima").expect("ima dir");
        assert!(lookup(&root, "ima").is_some());

        assert_eq!(
            show(&lookup(&ima_dir, "runtime_measurements_count").expect("count")),
            "1\n"
        );
        assert_eq!(
            show(&lookup(&ima_dir, "violations").expect("violations")),
            "0\n"
        );
        let ascii = show(
            &lookup(&ima_dir, "ascii_runtime_measurements_sha1")
                .expect("ascii_runtime_measurements_sha1"),
        );
        assert!(ascii.contains("ima-ng sha1:"));
        assert!(ascii.contains("boot_aggregate"));

        let ascii_link = lookup(&ima_dir, "ascii_runtime_measurements").expect("ascii symlink");
        match &ascii_link.kind {
            KernfsKind::Symlink { target } => {
                assert_eq!(target, "ascii_runtime_measurements_sha1")
            }
            _ => panic!("ascii_runtime_measurements must be a symlink"),
        }

        let policy = lookup(&ima_dir, "policy").expect("policy");
        assert_eq!(show(&policy), "");
        let policy_text = b"measure func=FILE_CHECK fsname=rootfs mask=MAY_READ\n";
        assert_eq!(store(&policy, policy_text), Ok(policy_text.len()));
        assert_eq!(
            show(&policy),
            "measure func=FILE_CHECK mask=MAY_READ fsname=rootfs\n"
        );
    }
}
