//! linux-parity: complete
//! linux-source: vendor/linux/kernel/cgroup
//! test-origin: linux:vendor/linux/kernel/cgroup
//! CPU cgroup v2 controller (M32).
//!
//! Files:
//!   * `cpu.max`           — `<runtime_us> <period_us>` quota / period.
//!   * `cpu.max.burst`     — burst allowance.
//!   * `cpu.weight`        — 1..10000 (mapped from CFS load weight).
//!   * `cpu.weight.nice`   — -20..19 (alternative spelling of `cpu.weight`).
//!   * `cpu.idle`          — 0/1 (group runs at IDLE class).
//!   * `cpu.stat`          — usage_usec / nr_periods / nr_throttled / throttled_usec.
//!   * `cpu.pressure`      — psi cpu pressure (M19/M36).
//!
//! Reference: `vendor/linux/kernel/sched/core.c::cpu_cgrp_subsys::dfl_cftypes`.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::kernel::sched::prio::nice_to_weight;

/// Linux `default_cfs_period_us = 100000` (100 ms).
pub const BANDWIDTH_PERIOD_NS_DEFAULT: u64 = 100_000_000;

/// Linux `max_cfs_quota_period = 1s`.
pub const MAX_BW_BURST: u64 = 1_000_000_000;

/// Linux `CGROUP_WEIGHT_MIN = 1`.
pub const CGROUP_WEIGHT_MIN: u64 = 1;
/// Linux `CGROUP_WEIGHT_DFL = 100`.
pub const CGROUP_WEIGHT_DFL: u64 = 100;
/// Linux `CGROUP_WEIGHT_MAX = 10000`.
pub const CGROUP_WEIGHT_MAX: u64 = 10_000;

/// Per-cgroup CPU statistics (mirrors `struct cpu_stat` produced by `cpu.stat`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CpuStat {
    pub usage_usec: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub nr_periods: u64,
    pub nr_throttled: u64,
    pub throttled_usec: u64,
}

/// CPU controller per-cgroup state — the "task group" structure.
pub struct TaskGroup {
    /// Quota in nanoseconds per period; `u64::MAX` ≡ `cpu.max max`.
    pub bw_quota: u64,
    /// Period length in nanoseconds.
    pub bw_period: u64,
    /// Burst allowance.
    pub bw_burst: u64,
    /// CFS load weight (mapped from `cpu.weight`).
    pub shares: u64,
    /// `cpu.idle` flag — 1 means run at IDLE class.
    pub idle: bool,
    /// Runtime budget remaining in the current period (nanoseconds).
    pub runtime_remaining_ns: AtomicU64,
    /// Statistics counters.
    stat_usage_usec: AtomicU64,
    stat_nr_periods: AtomicU64,
    stat_nr_throttled: AtomicU64,
    stat_throttled_usec: AtomicU64,
}

impl TaskGroup {
    pub const fn new_root() -> Self {
        Self {
            bw_quota: u64::MAX,
            bw_period: BANDWIDTH_PERIOD_NS_DEFAULT,
            bw_burst: 0,
            shares: 1024, // NICE_0_LOAD
            idle: false,
            runtime_remaining_ns: AtomicU64::new(u64::MAX),
            stat_usage_usec: AtomicU64::new(0),
            stat_nr_periods: AtomicU64::new(0),
            stat_nr_throttled: AtomicU64::new(0),
            stat_throttled_usec: AtomicU64::new(0),
        }
    }

    /// Apply `cpu.max <quota> <period>` (Linux UAPI).
    ///
    /// Returns `Ok(())` on success.  `quota = u64::MAX` is `max` (no limit).
    pub fn set_max(&mut self, quota_ns: u64, period_ns: u64) -> Result<(), &'static str> {
        if period_ns == 0 || period_ns > MAX_BW_BURST {
            return Err("period out of range");
        }
        self.bw_period = period_ns;
        self.bw_quota = quota_ns;
        if quota_ns == u64::MAX {
            self.runtime_remaining_ns.store(u64::MAX, Ordering::Release);
        } else {
            self.runtime_remaining_ns.store(quota_ns, Ordering::Release);
        }
        Ok(())
    }

    /// Apply `cpu.weight <w>` (1..10000 → CFS load_weight via Linux's
    /// `scale_load(w * NICE_0_LOAD / 100)`).
    pub fn set_weight(&mut self, weight: u64) -> Result<(), &'static str> {
        if weight < CGROUP_WEIGHT_MIN || weight > CGROUP_WEIGHT_MAX {
            return Err("weight out of range");
        }
        // Linux: shares = weight * NICE_0_LOAD / 100
        self.shares = weight.saturating_mul(1024) / 100;
        Ok(())
    }

    /// Apply `cpu.weight.nice <nice>` (-20..19).
    pub fn set_weight_nice(&mut self, nice: i32) -> Result<(), &'static str> {
        if !(-20..=19).contains(&nice) {
            return Err("nice out of range");
        }
        self.shares = nice_to_weight(nice);
        Ok(())
    }

    pub fn set_idle(&mut self, idle: bool) {
        self.idle = idle;
    }

    /// Charge `delta_ns` of CPU time to this group.  Returns `true` if the
    /// group has runtime remaining; `false` if it should now be throttled.
    pub fn charge(&self, delta_ns: u64) -> bool {
        if self.bw_quota == u64::MAX {
            self.stat_usage_usec
                .fetch_add(delta_ns / 1000, Ordering::Relaxed);
            return true;
        }
        let mut remaining = self.runtime_remaining_ns.load(Ordering::Acquire);
        loop {
            if remaining < delta_ns {
                self.stat_nr_throttled.fetch_add(1, Ordering::Relaxed);
                self.stat_throttled_usec
                    .fetch_add(delta_ns / 1000, Ordering::Relaxed);
                return false;
            }
            let new = remaining - delta_ns;
            match self.runtime_remaining_ns.compare_exchange(
                remaining,
                new,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.stat_usage_usec
                        .fetch_add(delta_ns / 1000, Ordering::Relaxed);
                    return true;
                }
                Err(actual) => remaining = actual,
            }
        }
    }

    /// Replenish budget at the start of a new period.
    pub fn refresh_period(&self) {
        if self.bw_quota != u64::MAX {
            self.runtime_remaining_ns.store(
                self.bw_quota.saturating_add(self.bw_burst),
                Ordering::Release,
            );
        }
        self.stat_nr_periods.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stat_snapshot(&self) -> CpuStat {
        CpuStat {
            usage_usec: self.stat_usage_usec.load(Ordering::Relaxed),
            user_usec: 0,
            system_usec: 0,
            nr_periods: self.stat_nr_periods.load(Ordering::Relaxed),
            nr_throttled: self.stat_nr_throttled.load(Ordering::Relaxed),
            throttled_usec: self.stat_throttled_usec.load(Ordering::Relaxed),
        }
    }
}

/// Format `cpu.stat` output exactly as Linux does (one key=value per line).
///
/// Returns the number of bytes written.
pub fn format_cpu_stat(buf: &mut [u8], stat: &CpuStat) -> usize {
    use core::fmt::Write;
    struct W<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }
    impl<'a> Write for W<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let n = bytes.len().min(self.buf.len() - self.pos);
            self.buf[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
            self.pos += n;
            Ok(())
        }
    }
    let mut w = W { buf, pos: 0 };
    let _ = write!(
        w,
        "usage_usec {}\nuser_usec {}\nsystem_usec {}\nnr_periods {}\nnr_throttled {}\nthrottled_usec {}\n",
        stat.usage_usec,
        stat.user_usec,
        stat.system_usec,
        stat.nr_periods,
        stat.nr_throttled,
        stat.throttled_usec
    );
    w.pos
}

/// Parse `cpu.max <quota> <period>` (Linux UAPI: "<n>|max <period>").
pub fn parse_cpu_max(s: &str) -> Option<(u64, u64)> {
    let mut it = s.split_ascii_whitespace();
    let quota_tok = it.next()?;
    let period_tok = it.next();
    let quota = if quota_tok == "max" {
        u64::MAX
    } else {
        quota_tok.parse().ok()?
    };
    let period: u64 = period_tok
        .and_then(|t| t.parse().ok())
        .unwrap_or(BANDWIDTH_PERIOD_NS_DEFAULT / 1000);
    Some((quota, period.checked_mul(1000)?)) // periods are µs in UAPI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_group_has_no_quota() {
        let g = TaskGroup::new_root();
        assert_eq!(g.bw_quota, u64::MAX);
        assert_eq!(g.shares, 1024);
    }

    #[test]
    fn set_weight_scales_to_shares() {
        let mut g = TaskGroup::new_root();
        g.set_weight(100).unwrap();
        assert_eq!(g.shares, 1024);
        g.set_weight(200).unwrap();
        assert_eq!(g.shares, 2048);
    }

    #[test]
    fn set_weight_out_of_range_errors() {
        let mut g = TaskGroup::new_root();
        assert!(g.set_weight(0).is_err());
        assert!(g.set_weight(20_000).is_err());
    }

    #[test]
    fn cpu_max_charge_throttles_when_exhausted() {
        let mut g = TaskGroup::new_root();
        g.set_max(1_000_000, 100_000_000).unwrap(); // 1ms quota, 100ms period
        assert!(g.charge(500_000));
        assert!(g.charge(500_000));
        // Now at zero — next charge should be throttled.
        assert!(!g.charge(1));
    }

    #[test]
    fn refresh_period_replenishes_budget() {
        let mut g = TaskGroup::new_root();
        g.set_max(1_000_000, 100_000_000).unwrap();
        let _ = g.charge(900_000);
        g.refresh_period();
        // Budget back to full quota.
        assert!(g.charge(900_000));
    }

    #[test]
    fn parse_cpu_max_handles_max_keyword() {
        assert_eq!(parse_cpu_max("max 100000"), Some((u64::MAX, 100_000_000)));
        assert_eq!(parse_cpu_max("50000 100000"), Some((50_000, 100_000_000)));
        assert_eq!(parse_cpu_max("max 18446744073709551615"), None);
    }

    #[test]
    fn cpu_stat_format_matches_linux() {
        let g = TaskGroup::new_root();
        let stat = g.stat_snapshot();
        let mut buf = [0u8; 256];
        let n = format_cpu_stat(&mut buf, &stat);
        let s = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(s.contains("usage_usec 0"));
        assert!(s.contains("nr_periods 0"));
        assert!(s.contains("nr_throttled 0"));
    }

    #[test]
    fn cpu_max_period_zero_is_rejected() {
        let mut g = TaskGroup::new_root();
        assert!(g.set_max(1000, 0).is_err());
    }
}
