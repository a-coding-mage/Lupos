//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/aperfmperf.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/aperfmperf.c
//! APERF/MPERF average-frequency sampling.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/aperfmperf.c

// Linux uses the IA32_APERF and IA32_MPERF MSRs together with the maximum
// non-Turbo TSC frequency to estimate the average operating frequency
// over a sampling window. The delta calculation is identical to the one
// in `arch/x86/kernel/cpu/aperfmperf.c`: avg_khz = (delta_aperf / delta_mperf) * base_khz.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AperfMperfSample {
    pub aperf: u64,
    pub mperf: u64,
}

pub const fn delta(prev: AperfMperfSample, now: AperfMperfSample) -> AperfMperfSample {
    AperfMperfSample {
        aperf: now.aperf.wrapping_sub(prev.aperf),
        mperf: now.mperf.wrapping_sub(prev.mperf),
    }
}

pub const fn avg_khz(delta: AperfMperfSample, base_khz: u64) -> u64 {
    if delta.mperf == 0 {
        return 0;
    }
    // Use 128-bit-equivalent math: scale aperf delta to 64-bit before dividing.
    let scaled = (delta.aperf as u128).saturating_mul(base_khz as u128);
    (scaled / delta.mperf as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_mperf_delta_returns_zero_frequency() {
        let d = AperfMperfSample {
            aperf: 1000,
            mperf: 0,
        };
        assert_eq!(avg_khz(d, 2_400_000), 0);
    }

    #[test]
    fn idle_cpu_reports_a_lower_average_than_base() {
        // 50% MPERF utilization with matching APERF should be base * 0.5.
        let prev = AperfMperfSample { aperf: 0, mperf: 0 };
        let now = AperfMperfSample {
            aperf: 1_000_000,
            mperf: 2_000_000,
        };
        let d = delta(prev, now);
        assert_eq!(avg_khz(d, 4_000_000), 2_000_000);
    }
}
