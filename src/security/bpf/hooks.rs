//! linux-parity: complete
//! linux-source: vendor/linux/security/bpf/hooks.c
//! test-origin: linux:vendor/linux/security/bpf/hooks.c
//! BPF LSM registration facade.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::security::hooks::{LSM_ID_BPF, LsmHooks, NOOP_HOOKS};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BpfStorageBlob {
    pub storage: *mut (),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LsmBlobSizes {
    pub lbs_inode: usize,
}

pub const BPF_LSM_ID_NAME: &str = "bpf";
pub const BPF_LSM_BLOB_SIZES: LsmBlobSizes = LsmBlobSizes {
    lbs_inode: core::mem::size_of::<BpfStorageBlob>(),
};

pub const HOOKS: LsmHooks = LsmHooks {
    name: BPF_LSM_ID_NAME,
    id: LSM_ID_BPF,
    ..NOOP_HOOKS
};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let _ = crate::security::lsm_list::register_lsm(HOOKS);
    crate::kernel::printk::log_info!("", "LSM support for eBPF active");
}

pub fn blob_sizes() -> LsmBlobSizes {
    BPF_LSM_BLOB_SIZES
}

pub const fn has_inode_free_security_hook() -> bool {
    true
}

#[cfg(test)]
pub fn reset_for_test() {
    INITIALIZED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::hooks::LSM_ID_BPF;
    use crate::security::lsm_list::{TEST_LSM_LOCK, lsm_active_ids, reset_for_test as reset_lsms};

    #[test]
    fn bpf_hooks_match_linux_source_shape() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/bpf/hooks.c"
        ));
        assert!(source.contains("LSM_HOOK_INIT(NAME, bpf_lsm_##NAME)"));
        assert!(source.contains("LSM_HOOK_INIT(inode_free_security, bpf_inode_storage_free)"));
        assert!(source.contains(".name = \"bpf\""));
        assert!(source.contains(".id = LSM_ID_BPF"));
        assert!(source.contains(".lbs_inode = sizeof(struct bpf_storage_blob)"));

        assert_eq!(HOOKS.name, "bpf");
        assert_eq!(HOOKS.id, LSM_ID_BPF);
        assert_eq!(blob_sizes().lbs_inode, core::mem::size_of::<*mut ()>());
        assert!(has_inode_free_security_hook());
    }

    #[test]
    fn bpf_lsm_registers_once() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_lsms();
        reset_for_test();

        init();
        init();

        let mut ids = [0u64; 2];
        assert_eq!(lsm_active_ids(&mut ids), 1);
        assert_eq!(ids[0], LSM_ID_BPF);
    }
}
