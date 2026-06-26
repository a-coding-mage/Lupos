//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/suspend_hvm.c
//! test-origin: linux:vendor/linux/arch/x86/xen/suspend_hvm.c
//! Xen HVM post-suspend action ordering.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XenHvmPostSuspendAction {
    InitSharedInfo,
    RestoreVcpu,
    SetUpcallVector { cpu: usize },
    SetupCallbackVector,
    UnplugEmulatedDevices,
}

pub fn xen_hvm_post_suspend_plan(
    suspend_cancelled: bool,
    xen_percpu_upcall: bool,
    online_cpus: usize,
) -> Vec<XenHvmPostSuspendAction> {
    let mut actions = Vec::new();

    if !suspend_cancelled {
        actions.push(XenHvmPostSuspendAction::InitSharedInfo);
        actions.push(XenHvmPostSuspendAction::RestoreVcpu);
    }

    if xen_percpu_upcall {
        for cpu in 0..online_cpus {
            actions.push(XenHvmPostSuspendAction::SetUpcallVector { cpu });
        }
    } else {
        actions.push(XenHvmPostSuspendAction::SetupCallbackVector);
    }

    actions.push(XenHvmPostSuspendAction::UnplugEmulatedDevices);
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn post_suspend_reinitializes_then_restores_vectors_and_unplugs() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/suspend_hvm.c"
        ));
        assert!(source.contains("xen_hvm_init_shared_info();"));
        assert!(source.contains("xen_vcpu_restore();"));
        assert!(source.contains("xen_unplug_emulated_devices();"));

        assert_eq!(
            xen_hvm_post_suspend_plan(false, false, 4),
            vec![
                XenHvmPostSuspendAction::InitSharedInfo,
                XenHvmPostSuspendAction::RestoreVcpu,
                XenHvmPostSuspendAction::SetupCallbackVector,
                XenHvmPostSuspendAction::UnplugEmulatedDevices,
            ]
        );
    }

    #[test]
    fn cancelled_suspend_skips_shared_info_restore_but_keeps_upcall_setup() {
        assert_eq!(
            xen_hvm_post_suspend_plan(true, true, 2),
            vec![
                XenHvmPostSuspendAction::SetUpcallVector { cpu: 0 },
                XenHvmPostSuspendAction::SetUpcallVector { cpu: 1 },
                XenHvmPostSuspendAction::UnplugEmulatedDevices,
            ]
        );
    }
}
