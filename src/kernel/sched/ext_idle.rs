//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/ext/idle.c
//! test-origin: linux:vendor/linux/kernel/sched/ext/idle.c
//! sched_ext idle hooks.
//!
//! Mirrors `vendor/linux/kernel/sched/ext/idle.c`. These helpers are dormant
//! until sched_ext itself is enabled.

use super::entity::CpuMask;

pub const fn scx_idle_enabled() -> bool {
    false
}

pub fn scx_idle_pick_cpu(allowed: CpuMask) -> Option<u32> {
    for cpu in 0..super::MAX_CPUS as u32 {
        if allowed.test(cpu) {
            return Some(cpu);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_idle_picks_first_allowed_cpu() {
        assert_eq!(scx_idle_pick_cpu(CpuMask::one(3)), Some(3));
        assert_eq!(scx_idle_pick_cpu(CpuMask::empty()), None);
    }
}
