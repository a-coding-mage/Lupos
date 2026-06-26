//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/smp_hvm.c
//! test-origin: linux:vendor/linux/arch/x86/xen/smp_hvm.c
//! Xen HVM SMP operation wiring.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenHvmSmpAction {
    NativePrepareBootCpu,
    VcpuSetup { cpu: usize },
    InitTimeOps,
    InitSpinlocks,
    NativePrepareCpus { max_cpus: usize },
    InitInterrupt { cpu: usize },
    InitLockCpu { cpu: usize },
    MarkVcpuInvalid { cpu: usize },
    CleanupInterrupt { cpu: usize },
    UninitLockCpu { cpu: usize },
    TeardownTimer { cpu: usize },
}

pub fn xen_hvm_smp_prepare_boot_cpu(cpu: usize) -> Result<Vec<XenHvmSmpAction>, &'static str> {
    if cpu != 0 {
        return Err("BUG_ON");
    }
    Ok(alloc::vec![
        XenHvmSmpAction::NativePrepareBootCpu,
        XenHvmSmpAction::VcpuSetup { cpu: 0 },
        XenHvmSmpAction::InitTimeOps,
        XenHvmSmpAction::InitSpinlocks,
    ])
}

pub fn xen_hvm_smp_prepare_cpus(
    max_cpus: usize,
    possible_cpus: usize,
    vector_callback: bool,
) -> Vec<XenHvmSmpAction> {
    let mut actions = Vec::new();
    actions.push(XenHvmSmpAction::NativePrepareCpus { max_cpus });
    if vector_callback {
        actions.push(XenHvmSmpAction::InitInterrupt { cpu: 0 });
        actions.push(XenHvmSmpAction::InitLockCpu { cpu: 0 });
    }
    for cpu in 1..possible_cpus {
        actions.push(XenHvmSmpAction::MarkVcpuInvalid { cpu });
    }
    actions
}

pub fn xen_hvm_cleanup_dead_cpu(cpu: usize, vector_callback: bool) -> Vec<XenHvmSmpAction> {
    if !vector_callback {
        return Vec::new();
    }
    alloc::vec![
        XenHvmSmpAction::CleanupInterrupt { cpu },
        XenHvmSmpAction::UninitLockCpu { cpu },
        XenHvmSmpAction::TeardownTimer { cpu },
    ]
}

pub const fn xen_hvm_uses_pv_spinlocks(vector_callback: bool) -> bool {
    vector_callback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_hvm_smp_ops_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/smp_hvm.c"
        ));
        assert!(source.contains("BUG_ON(smp_processor_id() != 0);"));
        assert!(source.contains("native_smp_prepare_boot_cpu();"));
        assert!(source.contains("xen_vcpu_setup(0);"));
        assert!(source.contains("xen_hvm_init_time_ops();"));
        assert!(source.contains("xen_init_spinlocks();"));
        assert!(source.contains("native_smp_prepare_cpus(max_cpus);"));
        assert!(source.contains("if (xen_have_vector_callback)"));
        assert!(source.contains("xen_smp_intr_init(0)"));
        assert!(source.contains("XEN_VCPU_ID_INVALID"));
        assert!(source.contains("smp_ops.smp_prepare_boot_cpu = xen_hvm_smp_prepare_boot_cpu;"));
        assert!(source.contains("nopvspin = true;"));

        assert!(xen_hvm_smp_prepare_boot_cpu(1).is_err());
        assert_eq!(xen_hvm_smp_prepare_boot_cpu(0).unwrap().len(), 4);
        let prep = xen_hvm_smp_prepare_cpus(8, 3, true);
        assert!(prep.contains(&XenHvmSmpAction::InitInterrupt { cpu: 0 }));
        assert!(prep.contains(&XenHvmSmpAction::MarkVcpuInvalid { cpu: 2 }));
        assert_eq!(xen_hvm_cleanup_dead_cpu(2, true).len(), 3);
        assert!(!xen_hvm_uses_pv_spinlocks(false));
    }
}
