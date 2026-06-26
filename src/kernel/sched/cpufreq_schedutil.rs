//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cpufreq_schedutil.c
//! test-origin: linux:vendor/linux/kernel/sched/cpufreq_schedutil.c
//! schedutil cpufreq governor helpers.
//!
//! Mirrors `vendor/linux/kernel/sched/cpufreq_schedutil.c`. Linux applies a
//! 1.25 performance headroom before mapping utilization to frequency; Lupos
//! keeps that arithmetic separate from driver policy state.

use super::cpufreq::{SCHED_CAPACITY_SCALE, map_util_freq, map_util_perf};

pub fn sugov_next_freq(util: u64, max_util: u64, max_freq: u64) -> u64 {
    let boosted = map_util_perf(util).min(max_util);
    map_util_freq(boosted, max_freq, max_util)
}

pub fn sugov_cpu_is_busy(util: u64) -> bool {
    util >= SCHED_CAPACITY_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedutil_applies_headroom_and_caps_at_max() {
        assert_eq!(sugov_next_freq(512, 1024, 2000), 1250);
        assert_eq!(sugov_next_freq(1024, 1024, 2000), 2000);
    }
}
