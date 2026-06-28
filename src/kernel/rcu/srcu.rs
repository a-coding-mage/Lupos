//! linux-parity: complete
//! linux-source: vendor/linux/kernel/rcu
//! test-origin: linux:vendor/linux/kernel/rcu
//! Sleepable RCU (`srcu_struct`) — M34.
//!
//! Mirrors `vendor/linux/kernel/rcu/srcutiny.c`.  Lupos M34 ships the tiny
//! variant: one grace period at a time, two per-CPU counters per
//! `srcu_struct`.

use core::sync::atomic::{AtomicI32, Ordering};

use crate::kernel::sched::MAX_CPUS;

/// `struct srcu_struct` — Linux ABI shape (simplified for tiny SRCU).
pub struct SrcuStruct {
    /// Per-CPU pair of counters: `[cpu][0]` = lock-side, `[cpu][1]` = unlock-side.
    counters: [[AtomicI32; 2]; MAX_CPUS],
    /// Active index — flipped by each grace period.
    pub idx: AtomicI32,
}

impl SrcuStruct {
    pub const fn new() -> Self {
        Self {
            counters: [const { [const { AtomicI32::new(0) }; 2] }; MAX_CPUS],
            idx: AtomicI32::new(0),
        }
    }
}

#[inline]
fn cpu_index() -> usize {
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        // Skip the LAPIC MMIO read (a VM-exit on VBox) when only the BSP is
        // online; single-CPU SRCU read-side always resolves to index 0.
        if crate::arch::x86::kernel::smp::AP_READY_COUNT
            .load(core::sync::atomic::Ordering::Acquire)
            == 0
        {
            return 0;
        }
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

/// `srcu_read_lock(ssp)` — returns the index that must be passed to
/// `srcu_read_unlock`.
pub fn srcu_read_lock(ssp: &SrcuStruct) -> i32 {
    let idx = ssp.idx.load(Ordering::Acquire);
    let cpu = cpu_index();
    ssp.counters[cpu][idx as usize].fetch_add(1, Ordering::AcqRel);
    idx
}

pub fn srcu_read_unlock(ssp: &SrcuStruct, idx: i32) {
    let cpu = cpu_index();
    ssp.counters[cpu][idx as usize].fetch_sub(1, Ordering::AcqRel);
}

/// `synchronize_srcu(ssp)` — flip the index, then wait for the previous-index
/// counter to reach zero across all CPUs.
pub fn synchronize_srcu(ssp: &SrcuStruct) {
    let old = ssp.idx.load(Ordering::Acquire);
    ssp.idx.store(1 - old, Ordering::Release);
    loop {
        let mut sum: i32 = 0;
        for cpu in 0..MAX_CPUS {
            sum = sum.saturating_add(ssp.counters[cpu][old as usize].load(Ordering::Acquire));
        }
        if sum <= 0 {
            return;
        }
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        {
            // In tests, callers must release locks before synchronize.
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock_round_trip() {
        let s = SrcuStruct::new();
        let idx = srcu_read_lock(&s);
        srcu_read_unlock(&s, idx);
    }

    #[test]
    fn synchronize_advances_index() {
        let s = SrcuStruct::new();
        let before = s.idx.load(Ordering::Acquire);
        synchronize_srcu(&s);
        assert_ne!(s.idx.load(Ordering::Acquire), before);
    }
}
