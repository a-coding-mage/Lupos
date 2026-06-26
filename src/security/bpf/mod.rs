//! linux-parity: complete
//! linux-source: vendor/linux/security/bpf
//! test-origin: linux:vendor/linux/security/bpf
//! BPF LSM source-backed helpers.

pub mod hooks;

pub const BPF_LSM_MODULES: [&str; 1] = ["hooks"];
pub const BPF_LSM_MAKEFILE_OBJECT: &str = "hooks.o";

#[cfg(test)]
mod tests {
    use super::*;

    const MAKEFILE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/security/bpf/Makefile"
    ));
    const HOOKS_C: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/security/bpf/hooks.c"
    ));

    #[test]
    fn bpf_lsm_wrapper_matches_linux_source_set() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(BPF_LSM_MODULES, ["hooks"]);
        assert_eq!(BPF_LSM_MAKEFILE_OBJECT, "hooks.o");
        assert!(MAKEFILE.contains("obj-$(CONFIG_BPF_LSM) := hooks.o"));
        assert!(HOOKS_C.contains("DEFINE_LSM(bpf)"));
    }

    #[test]
    fn bpf_lsm_wrapper_reexports_hooks_module_contract() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        assert_eq!(hooks::BPF_LSM_ID_NAME, "bpf");
        assert_eq!(
            hooks::blob_sizes().lbs_inode,
            core::mem::size_of::<hooks::BpfStorageBlob>()
        );
        assert!(hooks::has_inode_free_security_hook());
    }
}
