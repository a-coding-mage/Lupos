//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/resctrl/monitor.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/resctrl/monitor.c
//! RDT monitoring counter aggregation.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/resctrl/monitor.c

// Monitoring counters are 24..62-bit wrapping registers per RMID. The
// driver scales raw counter ticks to bytes/sec using a per-event scale.
// We model the wrap-aware delta and the scale application.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorSample {
    pub raw: u64,
}

pub const fn wrap_aware_delta(prev: MonitorSample, now: MonitorSample, width: u8) -> u64 {
    let mask = if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    now.raw.wrapping_sub(prev.raw) & mask
}

pub const fn scale_bytes(raw_delta: u64, scale_kb: u32) -> u64 {
    raw_delta
        .saturating_mul(scale_kb as u64)
        .saturating_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_at_width_works_across_overflow() {
        let prev = MonitorSample {
            raw: (1u64 << 40) - 1,
        };
        let now = MonitorSample { raw: 9 };
        assert_eq!(wrap_aware_delta(prev, now, 40), 10);
    }

    #[test]
    fn scale_converts_ticks_to_bytes() {
        let bytes = scale_bytes(100, 64);
        assert_eq!(bytes, 100 * 64 * 1024);
    }
}
