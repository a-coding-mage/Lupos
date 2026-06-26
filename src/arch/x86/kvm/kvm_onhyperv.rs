//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kvm/kvm_onhyperv.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/kvm_onhyperv.c
//! KVM L1 Hyper-V remote TLB flush helpers.

use crate::arch::x86::hyperv::nested::{
    ENOTSUPP, HvGuestMappingFlushList, hyperv_fill_flush_guest_mapping_list,
    hyperv_flush_guest_mapping, hyperv_flush_guest_mapping_range,
};

pub const INVALID_PAGE: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmHvTlbRange {
    pub start_gfn: u64,
    pub pages: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervFlushEnv {
    pub hypercall_page_present: bool,
    pub pcpu_input_arg_present: bool,
    pub hypercall_status: u64,
}

impl Default for HypervFlushEnv {
    fn default() -> Self {
        Self {
            hypercall_page_present: true,
            pcpu_input_arg_present: true,
            hypercall_status: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmVcpuHyperv {
    pub hv_root_tdp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmOnHyperv {
    pub hv_root_tdp: u64,
    pub vcpus: alloc::vec::Vec<KvmVcpuHyperv>,
}

extern crate alloc;

impl KvmOnHyperv {
    pub fn new(roots: &[u64]) -> Self {
        Self {
            hv_root_tdp: INVALID_PAGE,
            vcpus: roots
                .iter()
                .copied()
                .map(|hv_root_tdp| KvmVcpuHyperv { hv_root_tdp })
                .collect(),
        }
    }
}

pub fn kvm_fill_hv_flush_list_func(
    flush: &mut HvGuestMappingFlushList,
    range: KvmHvTlbRange,
) -> i32 {
    hyperv_fill_flush_guest_mapping_list(flush, range.start_gfn, range.pages)
}

pub fn hv_remote_flush_root_tdp(
    root_tdp: u64,
    range: Option<KvmHvTlbRange>,
    env: HypervFlushEnv,
) -> i32 {
    if let Some(range) = range {
        hyperv_flush_guest_mapping_range(
            root_tdp,
            env.hypercall_page_present,
            env.pcpu_input_arg_present,
            env.hypercall_status,
            Some(|flush: &mut HvGuestMappingFlushList| kvm_fill_hv_flush_list_func(flush, range)),
        )
        .0
    } else {
        hyperv_flush_guest_mapping(
            root_tdp,
            env.hypercall_page_present,
            env.pcpu_input_arg_present,
            env.hypercall_status,
        )
        .0
    }
}

pub fn hv_flush_remote_tlbs_range(
    kvm: &mut KvmOnHyperv,
    start_gfn: u64,
    nr_pages: u64,
    env: HypervFlushEnv,
) -> i32 {
    let range = KvmHvTlbRange {
        start_gfn,
        pages: nr_pages,
    };
    __hv_flush_remote_tlbs_range(kvm, Some(range), env)
}

pub fn hv_flush_remote_tlbs(kvm: &mut KvmOnHyperv, env: HypervFlushEnv) -> i32 {
    __hv_flush_remote_tlbs_range(kvm, None, env)
}

pub fn __hv_flush_remote_tlbs_range(
    kvm: &mut KvmOnHyperv,
    range: Option<KvmHvTlbRange>,
    env: HypervFlushEnv,
) -> i32 {
    let mut ret = 0;

    if !valid_page(kvm.hv_root_tdp) {
        let mut nr_unique_valid_roots = 0;

        for vcpu in &kvm.vcpus {
            let root = vcpu.hv_root_tdp;
            if !valid_page(root) || root == kvm.hv_root_tdp {
                continue;
            }

            nr_unique_valid_roots += 1;
            if nr_unique_valid_roots == 1 {
                kvm.hv_root_tdp = root;
            }

            if ret == 0 {
                ret = hv_remote_flush_root_tdp(root, range, env);
            }

            if ret != 0 && nr_unique_valid_roots > 1 {
                break;
            }
        }

        if nr_unique_valid_roots > 1 {
            kvm.hv_root_tdp = INVALID_PAGE;
        }
    } else {
        ret = hv_remote_flush_root_tdp(kvm.hv_root_tdp, range, env);
    }

    ret
}

pub fn hv_track_root_tdp(
    kvm: &mut KvmOnHyperv,
    vcpu_index: usize,
    root_tdp: u64,
    hv_flush_enabled: bool,
) {
    if !hv_flush_enabled {
        return;
    }
    if let Some(vcpu) = kvm.vcpus.get_mut(vcpu_index) {
        vcpu.hv_root_tdp = root_tdp;
        if root_tdp != kvm.hv_root_tdp {
            kvm.hv_root_tdp = INVALID_PAGE;
        }
    }
}

pub const fn valid_page(page: u64) -> bool {
    page != INVALID_PAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kvm_onhyperv_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/kvm_onhyperv.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kvm/kvm_onhyperv.h"
        ));
        assert!(source.contains("struct kvm_hv_tlb_range"));
        assert!(source.contains("kvm_fill_hv_flush_list_func"));
        assert!(source.contains("return hyperv_fill_flush_guest_mapping_list"));
        assert!(source.contains("hv_remote_flush_root_tdp"));
        assert!(source.contains("return hyperv_flush_guest_mapping_range"));
        assert!(source.contains("return hyperv_flush_guest_mapping(root_tdp);"));
        assert!(source.contains("__hv_flush_remote_tlbs_range"));
        assert!(source.contains("spin_lock(&kvm_arch->hv_root_tdp_lock);"));
        assert!(source.contains("if (!VALID_PAGE(kvm_arch->hv_root_tdp))"));
        assert!(source.contains("if (++nr_unique_valid_roots == 1)"));
        assert!(source.contains("if (ret && nr_unique_valid_roots > 1)"));
        assert!(source.contains("kvm_arch->hv_root_tdp = INVALID_PAGE;"));
        assert!(source.contains("int hv_flush_remote_tlbs_range"));
        assert!(source.contains("int hv_flush_remote_tlbs(struct kvm *kvm)"));
        assert!(source.contains("void hv_track_root_tdp"));
        assert!(source.contains("kvm_x86_ops.flush_remote_tlbs == hv_flush_remote_tlbs"));
        assert!(header.contains("void hv_track_root_tdp"));

        assert!(valid_page(0));
        assert!(!valid_page(INVALID_PAGE));
    }

    #[test]
    fn remote_flush_uses_single_cached_root_when_valid() {
        let env = HypervFlushEnv::default();
        let mut kvm = KvmOnHyperv::new(&[0x1000, 0x1000]);
        kvm.hv_root_tdp = 0x1000;

        assert_eq!(hv_flush_remote_tlbs(&mut kvm, env), 0);
        assert_eq!(kvm.hv_root_tdp, 0x1000);
        assert_eq!(hv_flush_remote_tlbs_range(&mut kvm, 4, 2, env), 0);
    }

    #[test]
    fn remote_flush_discovers_common_or_multiple_roots_like_linux() {
        let env = HypervFlushEnv::default();
        let mut common = KvmOnHyperv::new(&[INVALID_PAGE, 0x2000, 0x2000]);
        assert_eq!(hv_flush_remote_tlbs(&mut common, env), 0);
        assert_eq!(common.hv_root_tdp, 0x2000);

        let mut mixed = KvmOnHyperv::new(&[0x2000, 0x3000]);
        assert_eq!(hv_flush_remote_tlbs(&mut mixed, env), 0);
        assert_eq!(mixed.hv_root_tdp, INVALID_PAGE);

        let failing_env = HypervFlushEnv {
            hypercall_status: 7,
            ..HypervFlushEnv::default()
        };
        let mut failing_mixed = KvmOnHyperv::new(&[0x2000, 0x3000]);
        assert_eq!(
            hv_flush_remote_tlbs(&mut failing_mixed, failing_env),
            -ENOTSUPP
        );
        assert_eq!(failing_mixed.hv_root_tdp, INVALID_PAGE);
    }

    #[test]
    fn track_root_tdp_invalidates_common_root_on_divergence() {
        let mut kvm = KvmOnHyperv::new(&[0x1000, 0x1000]);
        kvm.hv_root_tdp = 0x1000;

        hv_track_root_tdp(&mut kvm, 1, 0x1000, true);
        assert_eq!(kvm.hv_root_tdp, 0x1000);
        hv_track_root_tdp(&mut kvm, 1, 0x4000, true);
        assert_eq!(kvm.vcpus[1].hv_root_tdp, 0x4000);
        assert_eq!(kvm.hv_root_tdp, INVALID_PAGE);

        hv_track_root_tdp(&mut kvm, 0, 0x5000, false);
        assert_eq!(kvm.vcpus[0].hv_root_tdp, 0x1000);
    }
}
