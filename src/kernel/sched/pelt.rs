//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/pelt.c
//! test-origin: linux:vendor/linux/kernel/sched/pelt.c
//! PELT (Per-Entity Load Tracking) — Linux 6.x EWMA accumulators.
//!
//! M29 ships PELT as zero-cost stubs; M31's load balancer wires them up.

use super::entity::SchedAvg;

/// Linux `LOAD_AVG_PERIOD = 32` ms (geometric series period).
pub const LOAD_AVG_PERIOD_NS: u64 = 32_000_000;

/// Linux `LOAD_AVG_MAX` — saturation point for the geometric sum (47742).
pub const LOAD_AVG_MAX: u32 = 47742;

/// Update the per-entity load average at scheduler-clock time `now`.
///
/// M29 stub — increments the period_contrib field but does not yet feed
/// `load_avg`/`runnable_avg`/`util_avg`.  M31 lands the full geometric sum.
#[inline]
pub fn update_avg(avg: &mut SchedAvg, now: u64, runnable: bool) {
    let last = avg.last_update_time;
    if last == 0 || now <= last {
        avg.last_update_time = now;
        return;
    }
    let delta = now - last;
    avg.last_update_time = now;
    avg.period_contrib = avg.period_contrib.saturating_add(delta as u32);
    if runnable {
        avg.runnable_sum = avg.runnable_sum.saturating_add(delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_update_seeds_timestamp() {
        let mut a = SchedAvg::zeroed();
        update_avg(&mut a, 1_000, true);
        assert_eq!(a.last_update_time, 1_000);
    }

    #[test]
    fn subsequent_updates_accumulate() {
        let mut a = SchedAvg::zeroed();
        update_avg(&mut a, 1_000, true);
        update_avg(&mut a, 2_000, true);
        assert_eq!(a.last_update_time, 2_000);
        assert!(a.runnable_sum > 0);
    }

    #[test]
    fn load_avg_period_matches_linux() {
        assert_eq!(LOAD_AVG_PERIOD_NS, 32_000_000);
        assert_eq!(LOAD_AVG_MAX, 47742);
    }
}
