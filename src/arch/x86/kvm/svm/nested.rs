//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/svm/nested.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/svm/nested.c
//! Nested SVM control validation and exit routing.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/svm/nested.c

use crate::include::uapi::errno::EINVAL;

pub const SVM_EXIT_NPF: u64 = 0x400;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NestedSvmControls {
    pub nested_cr3: u64,
    pub npt_enabled: bool,
    pub virtual_vmload_vmsave: bool,
    pub pause_filter_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NestedSvmEnter {
    RunGuest,
    EmulateVmloadVmsave,
    InjectNestedPageFault { gpa: u64, error_code: u32 },
}

pub const fn nested_cr3_is_aligned(nested_cr3: u64) -> bool {
    nested_cr3 & 0x1f == 0
}

pub const fn validate_nested_controls(ctl: NestedSvmControls) -> Result<(), i32> {
    if ctl.npt_enabled && !nested_cr3_is_aligned(ctl.nested_cr3) {
        return Err(EINVAL);
    }
    Ok(())
}

pub const fn nested_svm_enter_action(
    ctl: NestedSvmControls,
    fault_gpa: Option<u64>,
    error_code: u32,
) -> Result<NestedSvmEnter, i32> {
    match validate_nested_controls(ctl) {
        Ok(()) => {}
        Err(err) => return Err(err),
    }
    if let Some(gpa) = fault_gpa {
        return Ok(NestedSvmEnter::InjectNestedPageFault { gpa, error_code });
    }
    if ctl.virtual_vmload_vmsave {
        Ok(NestedSvmEnter::EmulateVmloadVmsave)
    } else {
        Ok(NestedSvmEnter::RunGuest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_cr3_alignment_matches_tdp_pdptr_rule() {
        assert!(nested_cr3_is_aligned(0x2000));
        assert!(!nested_cr3_is_aligned(0x2010));
        let bad = NestedSvmControls {
            nested_cr3: 0x2010,
            npt_enabled: true,
            virtual_vmload_vmsave: false,
            pause_filter_count: 0,
        };
        assert_eq!(validate_nested_controls(bad), Err(EINVAL));
    }

    #[test]
    fn nested_page_fault_takes_exit_priority() {
        let ctl = NestedSvmControls {
            nested_cr3: 0x2000,
            npt_enabled: true,
            virtual_vmload_vmsave: true,
            pause_filter_count: 0,
        };
        assert_eq!(
            nested_svm_enter_action(ctl, Some(0xdead), 0x5).unwrap(),
            NestedSvmEnter::InjectNestedPageFault {
                gpa: 0xdead,
                error_code: 0x5
            }
        );
    }
}
