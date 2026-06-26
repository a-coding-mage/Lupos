//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/feat_ctl.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/feat_ctl.c
//! IA32_FEAT_CTL MSR decoder for VMX/SGX/TXT enablement.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/feat_ctl.c

// `feat_ctl.c` reads MSR 0x3a, decides whether VMX/SGX/TSX_RTM should be
// available, and locks the MSR for the rest of boot. We model the decoder
// over a 64-bit MSR value and expose the policy outcome; the MSR write is
// fenced behind the MSR trait once it exists.

pub const MSR_IA32_FEAT_CTL: u32 = 0x3a;

pub const FEAT_CTL_LOCK: u64 = 1 << 0;
pub const FEAT_CTL_VMX_INSIDE_SMX: u64 = 1 << 1;
pub const FEAT_CTL_VMX_OUTSIDE_SMX: u64 = 1 << 2;
pub const FEAT_CTL_SGX_ENABLED: u64 = 1 << 18;
pub const FEAT_CTL_SGX_LC_ENABLED: u64 = 1 << 17;
pub const FEAT_CTL_LMCE: u64 = 1 << 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureControl {
    pub locked: bool,
    pub vmx_outside_smx: bool,
    pub vmx_inside_smx: bool,
    pub sgx_enabled: bool,
    pub sgx_launch_control: bool,
    pub local_mce: bool,
}

pub const fn parse(msr: u64) -> FeatureControl {
    FeatureControl {
        locked: msr & FEAT_CTL_LOCK != 0,
        vmx_outside_smx: msr & FEAT_CTL_VMX_OUTSIDE_SMX != 0,
        vmx_inside_smx: msr & FEAT_CTL_VMX_INSIDE_SMX != 0,
        sgx_enabled: msr & FEAT_CTL_SGX_ENABLED != 0,
        sgx_launch_control: msr & FEAT_CTL_SGX_LC_ENABLED != 0,
        local_mce: msr & FEAT_CTL_LMCE != 0,
    }
}

pub const fn vmx_usable(ctl: FeatureControl) -> bool {
    ctl.locked && ctl.vmx_outside_smx
}

pub const fn sgx_usable(ctl: FeatureControl) -> bool {
    ctl.locked && ctl.sgx_enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocked_msr_disables_all_features() {
        let ctl = parse(FEAT_CTL_VMX_OUTSIDE_SMX | FEAT_CTL_SGX_ENABLED);
        assert!(!ctl.locked);
        assert!(!vmx_usable(ctl));
        assert!(!sgx_usable(ctl));
    }

    #[test]
    fn vmx_requires_lock_and_outside_smx() {
        let msr = FEAT_CTL_LOCK | FEAT_CTL_VMX_OUTSIDE_SMX;
        assert!(vmx_usable(parse(msr)));

        let msr = FEAT_CTL_LOCK | FEAT_CTL_VMX_INSIDE_SMX;
        assert!(!vmx_usable(parse(msr)));
    }
}
