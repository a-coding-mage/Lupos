//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/hyperv/hv_crash.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/hv_crash.c
//! Hyper-V crash MSR model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/hyperv/hv_crash.c

pub const HV_X64_MSR_CRASH_P0: u32 = 0x4000_0100;
pub const HV_X64_MSR_CRASH_P1: u32 = 0x4000_0101;
pub const HV_X64_MSR_CRASH_P2: u32 = 0x4000_0102;
pub const HV_X64_MSR_CRASH_P3: u32 = 0x4000_0103;
pub const HV_X64_MSR_CRASH_P4: u32 = 0x4000_0104;
pub const HV_X64_MSR_CRASH_CTL: u32 = 0x4000_0105;
pub const HV_CRASH_CTL_CRASH_NOTIFY: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypervCrashState {
    pub params: [u64; 5],
    pub control: u64,
}

impl HypervCrashState {
    pub const fn new(params: [u64; 5]) -> Self {
        Self {
            params,
            control: HV_CRASH_CTL_CRASH_NOTIFY,
        }
    }

    pub const fn should_notify(self) -> bool {
        self.control & HV_CRASH_CTL_CRASH_NOTIFY != 0
    }
}

pub const fn crash_param_msr(index: u8) -> Option<u32> {
    match index {
        0 => Some(HV_X64_MSR_CRASH_P0),
        1 => Some(HV_X64_MSR_CRASH_P1),
        2 => Some(HV_X64_MSR_CRASH_P2),
        3 => Some(HV_X64_MSR_CRASH_P3),
        4 => Some(HV_X64_MSR_CRASH_P4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_params_map_to_sequential_msrs() {
        assert_eq!(crash_param_msr(0), Some(HV_X64_MSR_CRASH_P0));
        assert_eq!(crash_param_msr(4), Some(HV_X64_MSR_CRASH_P4));
        assert_eq!(crash_param_msr(5), None);
        assert!(HypervCrashState::new([1, 2, 3, 4, 5]).should_notify());
    }
}
