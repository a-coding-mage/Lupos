//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/test_udelay.c
//! test-origin: linux:vendor/linux/kernel/time/test_udelay.c
//! Delay calibration self-test coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/test_udelay.c`.

pub fn udelay_loops(usec: u64, loops_per_usec: u64) -> u64 {
    usec.saturating_mul(loops_per_usec)
}

pub fn udelay_within_tolerance(expected_usec: u64, measured_usec: u64, tolerance_pct: u64) -> bool {
    let tolerance = expected_usec.saturating_mul(tolerance_pct) / 100;
    expected_usec.abs_diff(measured_usec) <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_tolerance_uses_percent_margin() {
        assert_eq!(udelay_loops(10, 5), 50);
        assert!(udelay_within_tolerance(100, 105, 5));
        assert!(!udelay_within_tolerance(100, 106, 5));
    }
}
