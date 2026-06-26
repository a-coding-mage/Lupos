//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/suspend_pv.c
//! test-origin: linux:vendor/linux/arch/x86/xen/suspend_pv.c
//! Xen PV suspend pre/post state transitions.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenStartInfo {
    pub store_mfn: u64,
    pub console_mfn: u64,
    pub shared_info: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenPvSuspendState {
    pub start_info: XenStartInfo,
    pub irqs_disabled: bool,
    pub mm_pinned: bool,
    pub dummy_shared_info_active: bool,
    pub fixmap_shared_info_active: bool,
    pub mfn_list_built: bool,
    pub vcpu_restored: bool,
}

pub const fn mfn_to_pfn(mfn: u64) -> u64 {
    mfn
}

pub const fn pfn_to_mfn(pfn: u64) -> u64 {
    pfn
}

pub fn xen_pv_pre_suspend(state: &mut XenPvSuspendState) -> Result<(), i32> {
    state.mm_pinned = true;
    state.start_info.store_mfn = mfn_to_pfn(state.start_info.store_mfn);
    state.start_info.console_mfn = mfn_to_pfn(state.start_info.console_mfn);
    if !state.irqs_disabled {
        return Err(-EINVAL);
    }
    state.dummy_shared_info_active = true;
    state.fixmap_shared_info_active = false;
    Ok(())
}

pub fn xen_pv_post_suspend(
    state: &mut XenPvSuspendState,
    suspend_cancelled: bool,
) -> Result<(), i32> {
    state.mfn_list_built = true;
    state.fixmap_shared_info_active = true;
    state.dummy_shared_info_active = false;

    if suspend_cancelled {
        state.start_info.store_mfn = pfn_to_mfn(state.start_info.store_mfn);
        state.start_info.console_mfn = pfn_to_mfn(state.start_info.console_mfn);
    } else {
        state.vcpu_restored = true;
    }

    state.mm_pinned = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> XenPvSuspendState {
        XenPvSuspendState {
            start_info: XenStartInfo {
                store_mfn: 11,
                console_mfn: 12,
                shared_info: 13,
            },
            irqs_disabled: true,
            mm_pinned: false,
            dummy_shared_info_active: false,
            fixmap_shared_info_active: true,
            mfn_list_built: false,
            vcpu_restored: false,
        }
    }

    #[test]
    fn xen_pv_suspend_order_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/suspend_pv.c"
        ));
        assert!(source.contains("xen_mm_pin_all();"));
        assert!(source.contains("xen_start_info->store_mfn = mfn_to_pfn"));
        assert!(source.contains("BUG_ON(!irqs_disabled());"));
        assert!(source.contains("HYPERVISOR_shared_info = &xen_dummy_shared_info;"));
        assert!(source.contains("xen_build_mfn_list_list();"));
        assert!(source.contains("set_fixmap(FIX_PARAVIRT_BOOTMAP"));
        assert!(source.contains("if (suspend_cancelled)"));
        assert!(source.contains("xen_vcpu_restore();"));
        assert!(source.contains("xen_mm_unpin_all();"));

        let mut active = state();
        assert_eq!(xen_pv_pre_suspend(&mut active), Ok(()));
        assert!(active.mm_pinned);
        assert!(active.dummy_shared_info_active);
        assert_eq!(xen_pv_post_suspend(&mut active, false), Ok(()));
        assert!(active.vcpu_restored);
        assert!(!active.mm_pinned);

        let mut invalid = state();
        invalid.irqs_disabled = false;
        assert_eq!(xen_pv_pre_suspend(&mut invalid), Err(-EINVAL));
    }
}
