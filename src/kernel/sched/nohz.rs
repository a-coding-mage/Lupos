//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! NOHZ idle bookkeeping (M31).
//!
//! Linux `tick_nohz_idle_enter` / `tick_nohz_idle_exit` toggle a per-CPU bit
//! that the LAPIC tick handler consults to decide whether to mask itself when
//! all CPUs report idle.  M31 ships the bookkeeping only; physically masking
//! the LAPIC timer requires the per-CPU clockevents wiring landing in M36.

use core::sync::atomic::{AtomicU64, Ordering};

/// Bitmap of CPUs currently in NOHZ idle.  Updated by
/// `tick_nohz_idle_enter` / `tick_nohz_idle_exit`.
static NOHZ_IDLE_MASK: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn tick_nohz_idle_enter(cpu: u32) {
    NOHZ_IDLE_MASK.fetch_or(1u64 << (cpu & 63), Ordering::Release);
}

#[inline]
pub fn tick_nohz_idle_exit(cpu: u32) {
    NOHZ_IDLE_MASK.fetch_and(!(1u64 << (cpu & 63)), Ordering::Release);
}

#[inline]
pub fn is_nohz_idle(cpu: u32) -> bool {
    NOHZ_IDLE_MASK.load(Ordering::Acquire) & (1u64 << (cpu & 63)) != 0
}

#[inline]
pub fn all_cpus_idle(active_mask: u64) -> bool {
    let nohz = NOHZ_IDLE_MASK.load(Ordering::Acquire);
    (nohz & active_mask) == active_mask && active_mask != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    #[test]
    fn enter_then_exit_clears_mask() {
        // Reset
        NOHZ_IDLE_MASK.store(0, Ordering::SeqCst);
        tick_nohz_idle_enter(2);
        assert!(is_nohz_idle(2));
        tick_nohz_idle_exit(2);
        assert!(!is_nohz_idle(2));
    }

    #[test]
    fn all_cpus_idle_requires_all_bits_set() {
        NOHZ_IDLE_MASK.store(0, Ordering::SeqCst);
        let mask = 0b1111u64;
        assert!(!all_cpus_idle(mask));
        tick_nohz_idle_enter(0);
        tick_nohz_idle_enter(1);
        tick_nohz_idle_enter(2);
        tick_nohz_idle_enter(3);
        assert!(all_cpus_idle(mask));
        tick_nohz_idle_exit(0);
        tick_nohz_idle_exit(1);
        tick_nohz_idle_exit(2);
        tick_nohz_idle_exit(3);
    }
}
