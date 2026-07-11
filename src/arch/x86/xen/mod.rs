//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/xen
//! test-origin: linux:vendor/linux/arch/x86/xen
//! Xen x86 paravirtualization policy.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/xen/apic.c
//! - vendor/linux/arch/x86/xen/debugfs.c
//! - vendor/linux/arch/x86/xen/efi.c
//! - vendor/linux/arch/x86/xen/enlighten.c
//! - vendor/linux/arch/x86/xen/enlighten_hvm.c
//! - vendor/linux/arch/x86/xen/enlighten_pv.c
//! - vendor/linux/arch/x86/xen/enlighten_pvh.c
//! - vendor/linux/arch/x86/xen/grant-table.c
//! - vendor/linux/arch/x86/xen/irq.c
//! - vendor/linux/arch/x86/xen/mmu.c
//! - vendor/linux/arch/x86/xen/mmu_hvm.c
//! - vendor/linux/arch/x86/xen/mmu_pv.c
//! - vendor/linux/arch/x86/xen/multicalls.c
//! - vendor/linux/arch/x86/xen/p2m.c
//! - vendor/linux/arch/x86/xen/platform-pci-unplug.c
//! - vendor/linux/arch/x86/xen/pmu.c
//! - vendor/linux/arch/x86/xen/setup.c
//! - vendor/linux/arch/x86/xen/smp.c
//! - vendor/linux/arch/x86/xen/smp_hvm.c
//! - vendor/linux/arch/x86/xen/smp_pv.c
//! - vendor/linux/arch/x86/xen/spinlock.c
//! - vendor/linux/arch/x86/xen/suspend.c
//! - vendor/linux/arch/x86/xen/suspend_hvm.c
//! - vendor/linux/arch/x86/xen/suspend_pv.c
//! - vendor/linux/arch/x86/xen/time.c
//! - vendor/linux/arch/x86/xen/trace.c
//! - vendor/linux/arch/x86/xen/vga.c

use crate::include::uapi::errno::{EINVAL, ENODEV};

pub mod debugfs;
pub mod irq;
pub mod mmu;
pub mod mmu_hvm;
pub mod smp_hvm;
pub mod suspend;
pub mod suspend_hvm;
pub mod suspend_pv;
pub mod trace;
pub mod vga;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenDomainType {
    Native,
    Pv,
    Hvm,
    Pvh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenFeatures {
    pub domain: XenDomainType,
    pub vector_callback: bool,
    pub grant_table: bool,
    pub p2m: bool,
    pub efi: bool,
    pub pmu: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenMmuMode {
    Native,
    Pv,
    Hvm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenSuspendState {
    pub domain: XenDomainType,
    pub shared_info_mapped: bool,
    pub vcpu_info_ready: bool,
}

pub const fn xen_enlightened(features: XenFeatures) -> bool {
    !matches!(features.domain, XenDomainType::Native)
}

pub const fn xen_apic_vector_callback_required(features: XenFeatures) -> bool {
    matches!(features.domain, XenDomainType::Hvm | XenDomainType::Pvh) && features.vector_callback
}

pub const fn xen_mmu_mode(features: XenFeatures) -> XenMmuMode {
    match features.domain {
        XenDomainType::Pv => XenMmuMode::Pv,
        XenDomainType::Hvm | XenDomainType::Pvh => XenMmuMode::Hvm,
        XenDomainType::Native => XenMmuMode::Native,
    }
}

pub const fn xen_grant_table_ready(features: XenFeatures) -> Result<(), i32> {
    if xen_enlightened(features) && features.grant_table {
        Ok(())
    } else {
        Err(ENODEV)
    }
}

pub const fn xen_p2m_ready(features: XenFeatures) -> bool {
    xen_enlightened(features) && features.p2m
}

pub const fn xen_multicall_slots(requested: usize) -> Result<usize, i32> {
    if requested == 0 || requested > 32 {
        Err(EINVAL)
    } else {
        Ok(requested)
    }
}

pub const fn xen_should_unplug_platform_pci(features: XenFeatures) -> bool {
    matches!(features.domain, XenDomainType::Hvm | XenDomainType::Pvh)
}

pub const fn xen_pmu_available(features: XenFeatures) -> bool {
    xen_enlightened(features) && features.pmu
}

pub const fn xen_suspend_valid(state: XenSuspendState) -> Result<(), i32> {
    if matches!(state.domain, XenDomainType::Native) {
        return Err(ENODEV);
    }
    if !state.shared_info_mapped || !state.vcpu_info_ready {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn xen_spinlock_should_kick(waiters: u32, preempted: bool) -> bool {
    waiters != 0 && preempted
}

pub const fn xen_clock_uses_stolen_time(features: XenFeatures) -> bool {
    matches!(features.domain, XenDomainType::Pv | XenDomainType::Pvh)
}

pub const fn xen_vga_console_allowed(features: XenFeatures) -> bool {
    !matches!(features.domain, XenDomainType::Pv)
}

pub const fn xen_trace_enabled(features: XenFeatures, debugfs_mounted: bool) -> bool {
    xen_enlightened(features) && debugfs_mounted
}

#[cfg(test)]
mod tests {
    use super::*;

    const HVM: XenFeatures = XenFeatures {
        domain: XenDomainType::Hvm,
        vector_callback: true,
        grant_table: true,
        p2m: true,
        efi: true,
        pmu: false,
    };

    #[test]
    fn domain_type_selects_mmu_and_apic_policy() {
        assert!(xen_enlightened(HVM));
        assert!(xen_apic_vector_callback_required(HVM));
        assert_eq!(xen_mmu_mode(HVM), XenMmuMode::Hvm);
        assert!(xen_should_unplug_platform_pci(HVM));
    }

    #[test]
    fn grant_table_p2m_multicalls_and_suspend_are_gated() {
        assert_eq!(xen_grant_table_ready(HVM), Ok(()));
        assert!(xen_p2m_ready(HVM));
        assert_eq!(xen_multicall_slots(32), Ok(32));
        assert_eq!(xen_multicall_slots(33), Err(EINVAL));
        assert_eq!(
            xen_suspend_valid(XenSuspendState {
                domain: XenDomainType::Hvm,
                shared_info_mapped: true,
                vcpu_info_ready: false,
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn xen_runtime_helpers_match_domain_expectations() {
        let pv = XenFeatures {
            domain: XenDomainType::Pv,
            vector_callback: false,
            grant_table: true,
            p2m: true,
            efi: false,
            pmu: true,
        };
        assert!(xen_pmu_available(pv));
        assert!(xen_clock_uses_stolen_time(pv));
        assert!(!xen_vga_console_allowed(pv));
        assert!(xen_spinlock_should_kick(1, true));
        assert!(xen_trace_enabled(pv, true));
    }
}
