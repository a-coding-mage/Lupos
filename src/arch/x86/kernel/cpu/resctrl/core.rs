//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/core.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/core.c
//! Resource control resource init and per-CPU PQR state.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/resctrl/core.c

// `core.c` enumerates RDT resources (L3 CAT/CDP, L2 CAT, MBA, etc.) at
// boot, builds the per-resource domain list, and tracks the per-CPU
// IA32_PQR_ASSOC MSR shadow. We model the shadow state.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct PqrAssoc {
    pub rmid: u32,
    pub closid: u32,
}

pub const fn encode(pqr: PqrAssoc) -> u64 {
    (pqr.rmid as u64) | ((pqr.closid as u64) << 32)
}

pub const fn decode(value: u64) -> PqrAssoc {
    PqrAssoc {
        rmid: (value & 0xffff_ffff) as u32,
        closid: ((value >> 32) & 0xffff_ffff) as u32,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RdtResource {
    L3Cat,
    L3Cdp,
    L2Cat,
    MemoryBandwidth,
    L3Monitor,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pqr_round_trips_through_encode_decode() {
        let pqr = PqrAssoc { rmid: 7, closid: 3 };
        let value = encode(pqr);
        let back = decode(value);
        assert_eq!(back, pqr);
    }
}
