//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/pmu.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/pmu.c
//! Virtual PMU for KVM guests.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/pmu.c

// `pmu.c` virtualizes IA32_PERFEVTSEL{0..N} and IA32_PMC{0..N}. The
// vPMU programs the host PMU through perf_event_open() under the hood.
// We model the per-vCPU PMU descriptor.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmVPmu {
    pub gp_counters: u8,
    pub fixed_counters: u8,
    pub counter_bits: u8,
    pub global_ctrl: u64,
}

pub const fn global_counter_count(pmu: KvmVPmu) -> u32 {
    (pmu.gp_counters as u32) + (pmu.fixed_counters as u32)
}

pub const fn fixed_bit(index: u8) -> u64 {
    1u64 << (32 + index as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_count_sums_gp_and_fixed() {
        let pmu = KvmVPmu {
            gp_counters: 4,
            fixed_counters: 3,
            counter_bits: 48,
            global_ctrl: 0,
        };
        assert_eq!(global_counter_count(pmu), 7);
    }

    #[test]
    fn fixed_bit_is_in_upper_half_of_global_ctrl() {
        assert_eq!(fixed_bit(0), 1u64 << 32);
        assert_eq!(fixed_bit(2), 1u64 << 34);
    }
}
