//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/suspend.c
//! test-origin: linux:vendor/linux/arch/x86/xen/suspend.c
//! Xen architecture suspend/resume ordering.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenSuspendAction {
    SaveTimeMemoryArea,
    PvPreSuspend,
    PvPostSuspend { cancelled: bool },
    HvmPostSuspend { cancelled: bool },
    RestoreTimeMemoryArea,
    PmuFinish { cpu: usize },
    NotifySuspend { cpu: usize },
    NotifyRestore { cpu: usize },
    PmuInit { cpu: usize },
}

pub fn xen_arch_pre_suspend(pv_domain: bool) -> Vec<XenSuspendAction> {
    let mut actions = Vec::new();
    actions.push(XenSuspendAction::SaveTimeMemoryArea);
    if pv_domain {
        actions.push(XenSuspendAction::PvPreSuspend);
    }
    actions
}

pub fn xen_arch_post_suspend(pv_domain: bool, cancelled: bool) -> Vec<XenSuspendAction> {
    let mut actions = Vec::new();
    if pv_domain {
        actions.push(XenSuspendAction::PvPostSuspend { cancelled });
    } else {
        actions.push(XenSuspendAction::HvmPostSuspend { cancelled });
    }
    actions.push(XenSuspendAction::RestoreTimeMemoryArea);
    actions
}

pub fn xen_arch_suspend(online_cpus: usize) -> Vec<XenSuspendAction> {
    let mut actions = Vec::new();
    for cpu in 0..online_cpus {
        actions.push(XenSuspendAction::PmuFinish { cpu });
    }
    for cpu in 0..online_cpus {
        actions.push(XenSuspendAction::NotifySuspend { cpu });
    }
    actions
}

pub fn xen_arch_resume(online_cpus: usize) -> Vec<XenSuspendAction> {
    let mut actions = Vec::new();
    for cpu in 0..online_cpus {
        actions.push(XenSuspendAction::NotifyRestore { cpu });
    }
    for cpu in 0..online_cpus {
        actions.push(XenSuspendAction::PmuInit { cpu });
    }
    actions
}

pub const fn xen_spec_ctrl_saved(pv_domain: bool, spec_ctrl_feature: bool) -> bool {
    pv_domain && spec_ctrl_feature
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn xen_suspend_resume_order_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/suspend.c"
        ));
        assert!(source.contains("xen_save_time_memory_area();"));
        assert!(source.contains("if (xen_pv_domain())"));
        assert!(source.contains("xen_pv_pre_suspend();"));
        assert!(source.contains("xen_hvm_post_suspend(cancelled);"));
        assert!(source.contains("xen_restore_time_memory_area();"));
        assert!(source.contains("tick_resume_local();"));
        assert!(source.contains("tick_suspend_local();"));
        assert!(source.contains("MSR_IA32_SPEC_CTRL"));
        assert!(source.contains("for_each_online_cpu(cpu)"));
        assert!(source.contains("xen_pmu_finish(cpu);"));
        assert!(source.contains("xen_pmu_init(cpu);"));

        assert_eq!(
            xen_arch_pre_suspend(true),
            vec![
                XenSuspendAction::SaveTimeMemoryArea,
                XenSuspendAction::PvPreSuspend
            ]
        );
        assert_eq!(
            xen_arch_post_suspend(false, true),
            vec![
                XenSuspendAction::HvmPostSuspend { cancelled: true },
                XenSuspendAction::RestoreTimeMemoryArea,
            ]
        );
        assert_eq!(
            xen_arch_suspend(2),
            vec![
                XenSuspendAction::PmuFinish { cpu: 0 },
                XenSuspendAction::PmuFinish { cpu: 1 },
                XenSuspendAction::NotifySuspend { cpu: 0 },
                XenSuspendAction::NotifySuspend { cpu: 1 },
            ]
        );
        assert!(xen_spec_ctrl_saved(true, true));
        assert!(!xen_spec_ctrl_saved(false, true));
    }
}
