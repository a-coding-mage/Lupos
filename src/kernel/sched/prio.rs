//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! Nice → weight tables for CFS — verbatim from `vendor/linux/kernel/sched/core.c`.
//!
//! `SCHED_PRIO_TO_WEIGHT[nice + 20]` gives Linux's raw task weight.  On the
//! generic x86_64 target `scale_load()` shifts that value left by 10 before it
//! is stored in `sched_entity::load`; the fair-share ratio remains
//! 1024 / 15 ≈ 68×.
//!
//! `SCHED_PRIO_TO_WMULT[nice + 20]` is the precomputed `2^32 / weight` used by
//! `__calc_delta` to avoid a 64-bit divide on the fast path.

// ── SCHED_* policy constants (UAPI parity) ───────────────────────────────────
//
// Reference: `vendor/linux/include/uapi/linux/sched.h`.

pub const SCHED_NORMAL: u32 = 0;
pub const SCHED_FIFO: u32 = 1;
pub const SCHED_RR: u32 = 2;
pub const SCHED_BATCH: u32 = 3;
pub const SCHED_IDLE: u32 = 5;
pub const SCHED_DEADLINE: u32 = 6;
pub const SCHED_EXT: u32 = 7;

pub const SCHED_RESET_ON_FORK: u32 = 0x40000000;

// ── Priority constants (Linux include/linux/sched/prio.h) ────────────────────

pub const MAX_NICE: i32 = 19;
pub const MIN_NICE: i32 = -20;
pub const NICE_WIDTH: i32 = MAX_NICE - MIN_NICE + 1; // 40
pub const MAX_RT_PRIO: i32 = 100;
pub const MAX_PRIO: i32 = MAX_RT_PRIO + NICE_WIDTH; // 140
pub const DEFAULT_PRIO: i32 = MAX_RT_PRIO + NICE_WIDTH / 2; // 120

// ── CFS fixed-point ──────────────────────────────────────────────────────────

pub const SCHED_FIXEDPOINT_SHIFT: u32 = 10;
pub const SCHED_FIXEDPOINT_SCALE: u64 = 1u64 << SCHED_FIXEDPOINT_SHIFT;
/// Linux `CONFIG_64BIT` raises task-load resolution by another 10 bits.
pub const NICE_0_LOAD_SHIFT: u32 = SCHED_FIXEDPOINT_SHIFT + SCHED_FIXEDPOINT_SHIFT;
pub const NICE_0_LOAD: u64 = 1u64 << NICE_0_LOAD_SHIFT;

/// Idle class weight (Linux `WEIGHT_IDLEPRIO`).
pub const WEIGHT_IDLEPRIO: u64 = 3;
/// Precomputed inverse for `WEIGHT_IDLEPRIO` (Linux `WMULT_IDLEPRIO`).
pub const WMULT_IDLEPRIO: u32 = 1_431_655_765;

/// Linux x86_64 `scale_load()` (`CONFIG_64BIT`).
#[inline]
pub const fn scale_load(weight: u64) -> u64 {
    weight << SCHED_FIXEDPOINT_SHIFT
}

/// Linux x86_64 `scale_load_down()` (`CONFIG_64BIT`).
#[inline]
pub const fn scale_load_down(weight: u64) -> u64 {
    if weight == 0 {
        0
    } else {
        let down = weight >> SCHED_FIXEDPOINT_SHIFT;
        if down < 2 { 2 } else { down }
    }
}

// ── Nice-to-weight tables (verbatim from Linux core.c) ───────────────────────

/// `sched_prio_to_weight[nice + 20]`.
pub const SCHED_PRIO_TO_WEIGHT: [u64; 40] = [
    /* -20 */ 88761, 71755, 56483, 46273, 36291, /* -15 */ 29154, 23254, 18705, 14949,
    11916, /* -10 */ 9548, 7620, 6100, 4904, 3906, /*  -5 */ 3121, 2501, 1991, 1586,
    1277, /*   0 */ 1024, 820, 655, 526, 423, /*   5 */ 335, 272, 215, 172, 137,
    /*  10 */ 110, 87, 70, 56, 45, /*  15 */ 36, 29, 23, 18, 15,
];

/// `sched_prio_to_wmult[nice + 20]` = 2^32 / weight (precomputed).
pub const SCHED_PRIO_TO_WMULT: [u32; 40] = [
    /* -20 */ 48388, 59856, 76040, 92818, 118348, /* -15 */ 147320, 184698, 229616,
    287308, 360437, /* -10 */ 449829, 563644, 704093, 875809, 1099582, /*  -5 */ 1376151,
    1717300, 2157191, 2708050, 3363326, /*   0 */ 4194304, 5237765, 6557202, 8165337,
    10153587, /*   5 */ 12820798, 15790321, 19976592, 24970740, 31350126,
    /*  10 */ 39045157, 49367440, 61356676, 76695844, 95443717, /*  15 */ 119304647,
    148102320, 186737708, 238609294, 286331153,
];

/// Convert a nice value (-20..19) into a load weight.
#[inline]
pub fn nice_to_weight(nice: i32) -> u64 {
    let n = nice.clamp(MIN_NICE, MAX_NICE);
    scale_load(SCHED_PRIO_TO_WEIGHT[(n - MIN_NICE) as usize])
}

/// Convert a nice value (-20..19) into a precomputed inverse weight.
#[inline]
pub fn nice_to_wmult(nice: i32) -> u32 {
    let n = nice.clamp(MIN_NICE, MAX_NICE);
    SCHED_PRIO_TO_WMULT[(n - MIN_NICE) as usize]
}

/// Linux `NICE_TO_PRIO`: `prio = nice + DEFAULT_PRIO`.
#[inline]
pub const fn nice_to_prio(nice: i32) -> i32 {
    nice + DEFAULT_PRIO
}

/// Linux `PRIO_TO_NICE`: `nice = prio - DEFAULT_PRIO`.
#[inline]
pub const fn prio_to_nice(prio: i32) -> i32 {
    prio - DEFAULT_PRIO
}

/// Compute `delta_exec * NICE_0_LOAD / weight`, used by `update_curr` to
/// transform actual CPU time into virtual runtime.
///
/// Equivalent to Linux `__calc_delta(delta_exec, NICE_0_LOAD, &se->load)`.
/// As in Linux, the entity load weight must be nonzero.
#[inline]
pub fn calc_delta_fair(delta_exec: u64, weight: u64) -> u64 {
    if weight == NICE_0_LOAD {
        return delta_exec;
    }
    delta_exec.wrapping_mul(NICE_0_LOAD) / weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_nice_0_load_matches_linux_config_64bit() {
        // Linux sched.h raises load resolution by SCHED_FIXEDPOINT_SHIFT on
        // CONFIG_64BIT: scale_load(1024) == NICE_0_LOAD == 1 << 20.
        assert_eq!(SCHED_FIXEDPOINT_SHIFT, 10);
        assert_eq!(NICE_0_LOAD_SHIFT, 20);
        assert_eq!(NICE_0_LOAD, 1_048_576);
        assert_eq!(nice_to_weight(0), NICE_0_LOAD);
    }

    #[test]
    fn x86_64_nice_minus20_load_is_scaled() {
        assert_eq!(nice_to_weight(-20), 88761 << SCHED_FIXEDPOINT_SHIFT);
    }

    #[test]
    fn x86_64_nice_19_load_is_scaled() {
        assert_eq!(nice_to_weight(19), 15 << SCHED_FIXEDPOINT_SHIFT);
    }

    #[test]
    fn nice_19_to_nice_0_ratio_is_about_68() {
        // Linux documentation rounds the per-nice-level multiplier to "≈25%".
        // The 0/19 ratio is 1024/15 ≈ 68.27; selftests/sched expects this.
        let ratio = SCHED_PRIO_TO_WEIGHT[20] as f64 / SCHED_PRIO_TO_WEIGHT[39] as f64;
        assert!(ratio > 65.0 && ratio < 70.0);
    }

    #[test]
    fn calc_delta_fair_nice0_is_identity() {
        // Linux bypasses __calc_delta when weight == NICE_0_LOAD.
        assert_eq!(calc_delta_fair(1_000_000, NICE_0_LOAD), 1_000_000);
    }

    #[test]
    fn calc_delta_fair_lower_weight_grows_vruntime_faster() {
        let nice0 = calc_delta_fair(1_000_000, nice_to_weight(0));
        let nice19 = calc_delta_fair(1_000_000, nice_to_weight(19));
        // Lower weight → larger virtual runtime delta → falls behind in CFS.
        assert!(nice19 > nice0 * 50);
    }

    #[test]
    fn nice_prio_round_trip() {
        for n in MIN_NICE..=MAX_NICE {
            assert_eq!(prio_to_nice(nice_to_prio(n)), n);
        }
        assert_eq!(nice_to_prio(0), DEFAULT_PRIO);
    }

    #[test]
    fn rt_prio_band() {
        assert_eq!(MAX_RT_PRIO, 100);
        assert_eq!(MAX_PRIO, 140);
    }

    #[test]
    fn weight_table_has_40_entries() {
        assert_eq!(SCHED_PRIO_TO_WEIGHT.len(), 40);
        assert_eq!(SCHED_PRIO_TO_WMULT.len(), 40);
    }
}
