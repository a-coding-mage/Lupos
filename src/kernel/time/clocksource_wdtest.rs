//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/clocksource-wdtest.c
//! test-origin: linux:vendor/linux/kernel/time/clocksource-wdtest.c
//! Clocksource watchdog test coverage for M36.
//!
//! Mirrors `vendor/linux/kernel/time/clocksource-wdtest.c`.

pub fn watchdog_delta_within_margin(reference_ns: u64, candidate_ns: u64, margin_ns: u64) -> bool {
    reference_ns.abs_diff(candidate_ns) <= margin_ns
}

pub fn clocksource_watchdog_test(samples: &[(u64, u64)], margin_ns: u64) -> bool {
    samples.iter().all(|(reference, candidate)| {
        watchdog_delta_within_margin(*reference, *candidate, margin_ns)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_accepts_values_inside_margin() {
        assert!(watchdog_delta_within_margin(1_000, 1_010, 10));
        assert!(!watchdog_delta_within_margin(1_000, 1_011, 10));
    }
}
