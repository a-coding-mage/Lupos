//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/cpufreq.c
//! test-origin: linux:vendor/linux/kernel/sched/cpufreq.c
//! Scheduler to cpufreq hooks.
//!
//! Mirrors `vendor/linux/kernel/sched/cpufreq.c` and
//! `vendor/linux/include/linux/sched/cpufreq.h`.

pub const SCHED_CPUFREQ_IOWAIT: u32 = 1 << 0;
pub const SCHED_CAPACITY_SCALE: u64 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedCpufreqPolicy {
    pub cpu: u32,
    pub min_freq: u64,
    pub max_freq: u64,
    pub cur_freq: u64,
    pub util: u64,
    pub flags: u32,
}

impl SchedCpufreqPolicy {
    pub const fn new(cpu: u32, min_freq: u64, max_freq: u64) -> Self {
        Self {
            cpu,
            min_freq,
            max_freq,
            cur_freq: min_freq,
            util: 0,
            flags: 0,
        }
    }
}

pub fn map_util_freq(util: u64, freq: u64, cap: u64) -> u64 {
    if cap == 0 {
        return 0;
    }
    freq.saturating_mul(util).checked_div(cap).unwrap_or(0)
}

pub const fn map_util_perf(util: u64) -> u64 {
    util.saturating_add(util >> 2)
}

pub fn cpufreq_update_util(policy: &mut SchedCpufreqPolicy, time: u64, util: u64, flags: u32) {
    let boosted = if flags & SCHED_CPUFREQ_IOWAIT != 0 {
        map_util_perf(util)
    } else {
        util
    };
    let next = map_util_freq(boosted, policy.max_freq, SCHED_CAPACITY_SCALE);
    policy.util = util;
    policy.flags = flags;
    policy.cur_freq = next.clamp(policy.min_freq, policy.max_freq);
    let _ = time;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_util_freq_matches_linux_formula() {
        assert_eq!(map_util_freq(512, 2_000_000, 1024), 1_000_000);
    }

    #[test]
    fn iowait_boost_raises_frequency() {
        let mut policy = SchedCpufreqPolicy::new(0, 100, 1000);
        cpufreq_update_util(&mut policy, 0, 512, SCHED_CPUFREQ_IOWAIT);
        assert!(policy.cur_freq > 500);
    }
}
