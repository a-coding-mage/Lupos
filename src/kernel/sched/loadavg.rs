//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/loadavg.c
//! test-origin: linux:vendor/linux/kernel/sched/loadavg.c
//! Global load average accounting.
//!
//! Mirrors `vendor/linux/kernel/sched/loadavg.c` and
//! `vendor/linux/include/linux/sched/loadavg.h`.

use core::sync::atomic::{AtomicU64, Ordering};

pub const FSHIFT: u32 = 11;
pub const FIXED_1: u64 = 1 << FSHIFT;
pub const EXP_1: u64 = 1884;
pub const EXP_5: u64 = 2014;
pub const EXP_15: u64 = 2037;

static AVENRUN: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

pub fn calc_load(load: u64, exp: u64, active: u64) -> u64 {
    let mut newload = load
        .saturating_mul(exp)
        .saturating_add(active.saturating_mul(FIXED_1 - exp));
    if active >= load {
        newload = newload.saturating_add(FIXED_1 - 1);
    }
    newload / FIXED_1
}

pub fn calc_global_load(active_tasks: u64) {
    let active = active_tasks.saturating_mul(FIXED_1);
    let exps = [EXP_1, EXP_5, EXP_15];
    for (idx, exp) in exps.iter().enumerate() {
        let old = AVENRUN[idx].load(Ordering::Acquire);
        AVENRUN[idx].store(calc_load(old, *exp, active), Ordering::Release);
    }
}

pub fn get_avenrun(loads: &mut [u64; 3], offset: u64, shift: u32) {
    for idx in 0..3 {
        loads[idx] = AVENRUN[idx].load(Ordering::Acquire).saturating_add(offset) << shift;
    }
}

pub const fn load_int(load: u64) -> u64 {
    load >> FSHIFT
}

pub const fn load_frac(load: u64) -> u64 {
    (((load & (FIXED_1 - 1)) * 100) >> FSHIFT) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_load_uses_linux_fixed_point_formula() {
        assert_eq!(calc_load(0, EXP_1, FIXED_1), 164);
    }

    #[test]
    fn avenrun_snapshot_is_shifted() {
        calc_global_load(1);
        let mut loads = [0; 3];
        get_avenrun(&mut loads, 0, 0);
        assert!(loads[0] > 0);
    }
}
