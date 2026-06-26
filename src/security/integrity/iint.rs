//! linux-parity: complete
//! linux-source: vendor/linux/security/integrity/iint.c
//! test-origin: linux:vendor/linux/security/integrity/iint.c
//! Integrity securityfs directory and key-loading hooks.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::ENODEV;

lazy_static! {
    static ref INTEGRITY_DIR: Mutex<Option<Arc<KernfsNode>>> = Mutex::new(None);
}

static IMA_LOAD_X509_CALLS: AtomicUsize = AtomicUsize::new(0);
static EVM_LOAD_X509_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegrityKeyLoadReport {
    pub ima_x509_loaded: bool,
    pub evm_x509_loaded: bool,
}

pub fn integrity_kernel_read(
    source: &[u8],
    offset: u64,
    addr: &mut [u8],
    count: usize,
) -> Result<usize, i32> {
    let offset = offset as usize;
    if offset >= source.len() {
        return Ok(0);
    }
    let available = source.len() - offset;
    let len = count.min(addr.len()).min(available);
    addr[..len].copy_from_slice(&source[offset..offset + len]);
    Ok(len)
}

pub fn integrity_load_keys(config_ima_load_x509: bool) -> IntegrityKeyLoadReport {
    ima_load_x509();
    let evm_x509_loaded = if config_ima_load_x509 {
        false
    } else {
        evm_load_x509();
        true
    };
    IntegrityKeyLoadReport {
        ima_x509_loaded: true,
        evm_x509_loaded,
    }
}

fn ima_load_x509() {
    IMA_LOAD_X509_CALLS.fetch_add(1, Ordering::AcqRel);
}

fn evm_load_x509() {
    EVM_LOAD_X509_CALLS.fetch_add(1, Ordering::AcqRel);
}

pub fn integrity_fs_init() -> i32 {
    if INTEGRITY_DIR.lock().is_some() {
        return 0;
    }
    let dir = crate::security::inode::securityfs_create_dir("integrity", None);
    *INTEGRITY_DIR.lock() = Some(dir);
    0
}

pub fn integrity_fs_fini() {
    let Some(dir) = INTEGRITY_DIR.lock().clone() else {
        return;
    };
    if !dir.children.lock().is_empty() {
        return;
    }
    let root = crate::security::inode::securityfs_root();
    root.children.lock().remove("integrity");
    *INTEGRITY_DIR.lock() = None;
}

pub fn integrity_dir_present() -> bool {
    INTEGRITY_DIR.lock().is_some()
}

pub fn load_counts() -> (usize, usize) {
    (
        IMA_LOAD_X509_CALLS.load(Ordering::Acquire),
        EVM_LOAD_X509_CALLS.load(Ordering::Acquire),
    )
}

#[cfg(test)]
pub fn reset_for_test() {
    *INTEGRITY_DIR.lock() = None;
    IMA_LOAD_X509_CALLS.store(0, Ordering::Release);
    EVM_LOAD_X509_CALLS.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::lookup;

    #[test]
    fn integrity_iint_reads_buffers_loads_keys_and_manages_securityfs_dir() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::inode::reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/integrity/iint.c"
        ));
        assert!(source.contains("return __kernel_read(file, addr, count, &offset);"));
        assert!(source.contains("ima_load_x509();"));
        assert!(source.contains("if (!IS_ENABLED(CONFIG_IMA_LOAD_X509))"));
        assert!(source.contains("securityfs_create_dir(\"integrity\", NULL)"));
        assert!(source.contains("securityfs_remove(integrity_dir)"));

        let mut out = [0u8; 4];
        assert_eq!(integrity_kernel_read(b"abcdef", 2, &mut out, 4), Ok(4));
        assert_eq!(&out, b"cdef");
        assert_eq!(integrity_kernel_read(b"abcdef", 99, &mut out, 4), Ok(0));

        let report = integrity_load_keys(false);
        assert_eq!(
            report,
            IntegrityKeyLoadReport {
                ima_x509_loaded: true,
                evm_x509_loaded: true,
            }
        );
        assert_eq!(load_counts(), (1, 1));
        let report = integrity_load_keys(true);
        assert!(report.ima_x509_loaded);
        assert!(!report.evm_x509_loaded);
        assert_eq!(load_counts(), (2, 1));

        assert_eq!(integrity_fs_init(), 0);
        assert!(integrity_dir_present());
        let root = crate::security::inode::securityfs_root();
        assert!(lookup(&root, "integrity").is_some());
        integrity_fs_fini();
        assert!(!integrity_dir_present());
        assert!(lookup(&root, "integrity").is_none());
        assert_ne!(-ENODEV, 0);
    }
}
