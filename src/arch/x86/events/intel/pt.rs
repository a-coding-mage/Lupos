//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/events/intel/pt.c
//! test-origin: linux:vendor/linux/arch/x86/events/intel/pt.c
//! Intel Processor Trace PMU model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/events/intel/pt.c

use crate::include::uapi::errno::EOPNOTSUPP;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PtCapabilities {
    pub cr3_filter: bool,
    pub mtc: bool,
    pub ptwrite: bool,
    pub power_event_trace: bool,
}

pub const fn pt_available(cpuid_pt: bool, msr_rtit_ctl: bool) -> bool {
    cpuid_pt && msr_rtit_ctl
}

pub const fn pt_capabilities(leaf_ebx: u32, leaf_ecx: u32) -> PtCapabilities {
    PtCapabilities {
        cr3_filter: leaf_ebx & 1 != 0,
        mtc: leaf_ebx & (1 << 3) != 0,
        ptwrite: leaf_ebx & (1 << 4) != 0,
        power_event_trace: leaf_ecx & (1 << 4) != 0,
    }
}

pub const fn pt_programming_errno() -> i32 {
    EOPNOTSUPP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pt_capabilities_decode_cpuid_bits() {
        let caps = pt_capabilities(0b1_1001, 1 << 4);
        assert!(caps.cr3_filter);
        assert!(caps.mtc);
        assert!(caps.ptwrite);
        assert!(caps.power_event_trace);
    }
}
