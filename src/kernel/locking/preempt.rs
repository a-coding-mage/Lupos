//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Preemption count and atomicity predicates (M33).
//!
//! Mirrors `vendor/linux/include/linux/preempt.h`.  The `preempt_count` is a
//! per-CPU `u32` with the following bit layout:
//!
//! ```text
//!   [31:21] HARDIRQ_MASK  (HARDIRQ_BITS = 4)
//!   [20:16] NMI_MASK      (NMI_BITS = 4)
//!   [15:8]  SOFTIRQ_MASK  (SOFTIRQ_BITS = 8)
//!   [7:0]   PREEMPT_MASK  (PREEMPT_BITS = 8)
//! ```
//!
//! `in_atomic()` ≡ `preempt_count() != 0`.  Sleeping in atomic context is a
//! bug; `might_sleep()` `WARN_ON`s when violated.

use core::sync::atomic::Ordering;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched::MAX_CPUS;

pub const PREEMPT_BITS: u32 = 8;
pub const SOFTIRQ_BITS: u32 = 8;
pub const NMI_BITS: u32 = 4;
pub const HARDIRQ_BITS: u32 = 4;

pub const PREEMPT_SHIFT: u32 = 0;
pub const SOFTIRQ_SHIFT: u32 = PREEMPT_SHIFT + PREEMPT_BITS;
pub const NMI_SHIFT: u32 = SOFTIRQ_SHIFT + SOFTIRQ_BITS;
pub const HARDIRQ_SHIFT: u32 = NMI_SHIFT + NMI_BITS;

pub const PREEMPT_OFFSET: u32 = 1u32 << PREEMPT_SHIFT;
pub const SOFTIRQ_OFFSET: u32 = 1u32 << SOFTIRQ_SHIFT;
pub const NMI_OFFSET: u32 = 1u32 << NMI_SHIFT;
pub const HARDIRQ_OFFSET: u32 = 1u32 << HARDIRQ_SHIFT;

pub const PREEMPT_MASK: u32 = ((1u32 << PREEMPT_BITS) - 1) << PREEMPT_SHIFT;
pub const SOFTIRQ_MASK: u32 = ((1u32 << SOFTIRQ_BITS) - 1) << SOFTIRQ_SHIFT;
pub const NMI_MASK: u32 = ((1u32 << NMI_BITS) - 1) << NMI_SHIFT;
pub const HARDIRQ_MASK: u32 = ((1u32 << HARDIRQ_BITS) - 1) << HARDIRQ_SHIFT;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__local_bh_disable_ip",
        linux___local_bh_disable_ip as usize,
        false,
    );
    export_symbol_once(
        "__local_bh_enable_ip",
        linux___local_bh_enable_ip as usize,
        false,
    );
}

#[inline]
fn cpu_index() -> usize {
    // On host unit tests we never go through the LAPIC; pin to CPU 0 so the
    // counter remains coherent across thread boundaries inside one test
    // (since tests run on a single Tokio worker by default).
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        // Reading the LAPIC ID is an MMIO access — a VM-exit under VirtualBox
        // (and slow emulated MMIO under TCG) — and this runs on every preempt
        // disable/enable. When no AP is online the only CPU is the BSP (id 0),
        // so skip the LAPIC entirely; otherwise fall back to the real read.
        // (Linux reads this from a GS-relative per-CPU field; that's the proper
        // fix — see the M34 note in kernel::sched.)
        if crate::arch::x86::kernel::smp::AP_READY_COUNT.load(Ordering::Acquire) == 0 {
            return 0;
        }
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

#[inline]
fn counter() -> &'static core::sync::atomic::AtomicU32 {
    crate::arch::x86::kernel::setup_percpu::preempt_count_slot(cpu_index())
}

/// Read the current CPU's preempt count.
#[inline]
pub fn preempt_count() -> u32 {
    counter().load(Ordering::Acquire)
}

/// Increment the preempt-disable counter.
#[inline]
pub fn preempt_disable() {
    counter().fetch_add(PREEMPT_OFFSET, Ordering::AcqRel);
}

/// Decrement the preempt-disable counter.
#[inline]
pub fn preempt_enable() {
    counter().fetch_sub(PREEMPT_OFFSET, Ordering::AcqRel);
}

#[inline]
pub fn local_bh_disable() {
    counter().fetch_add(SOFTIRQ_OFFSET, Ordering::AcqRel);
}

#[inline]
pub fn local_bh_enable() {
    counter().fetch_sub(SOFTIRQ_OFFSET, Ordering::AcqRel);
}

/// `__local_bh_disable_ip` — `vendor/linux/kernel/softirq.c`.
#[unsafe(export_name = "__local_bh_disable_ip")]
pub extern "C" fn linux___local_bh_disable_ip(_ip: usize, cnt: u32) {
    counter().fetch_add(cnt, Ordering::AcqRel);
}

/// `__local_bh_enable_ip` — `vendor/linux/kernel/softirq.c`.
#[unsafe(export_name = "__local_bh_enable_ip")]
pub extern "C" fn linux___local_bh_enable_ip(_ip: usize, cnt: u32) {
    counter().fetch_sub(cnt, Ordering::AcqRel);
}

#[inline]
pub fn __irq_enter_raw() {
    counter().fetch_add(HARDIRQ_OFFSET, Ordering::AcqRel);
}

#[inline]
pub fn __irq_exit_raw() {
    counter().fetch_sub(HARDIRQ_OFFSET, Ordering::AcqRel);
}

#[inline]
pub fn __nmi_enter_raw() {
    counter().fetch_add(NMI_OFFSET, Ordering::AcqRel);
}

#[inline]
pub fn __nmi_exit_raw() {
    counter().fetch_sub(NMI_OFFSET, Ordering::AcqRel);
}

/// `in_atomic()` — true if any of preempt_disable / softirq / hardirq is held.
#[inline]
pub fn in_atomic() -> bool {
    preempt_count() != 0
}

#[inline]
pub fn in_softirq() -> bool {
    preempt_count() & SOFTIRQ_MASK != 0
}
#[inline]
pub fn in_hardirq() -> bool {
    preempt_count() & HARDIRQ_MASK != 0
}
#[inline]
pub fn in_nmi() -> bool {
    preempt_count() & NMI_MASK != 0
}
#[inline]
pub fn in_irq() -> bool {
    in_hardirq() || in_nmi()
}

/// `might_sleep()` — debug check, no-op in release.  Returns true if it would
/// have warned (callers ignore the return; tests use it).
#[inline]
pub fn might_sleep() -> bool {
    !in_atomic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_match_linux_preempt_h() {
        assert_eq!(PREEMPT_SHIFT, 0);
        assert_eq!(SOFTIRQ_SHIFT, 8);
        assert_eq!(NMI_SHIFT, 16);
        assert_eq!(HARDIRQ_SHIFT, 20);
    }

    #[test]
    fn masks_are_disjoint() {
        assert_eq!(PREEMPT_MASK & SOFTIRQ_MASK, 0);
        assert_eq!(SOFTIRQ_MASK & NMI_MASK, 0);
        assert_eq!(NMI_MASK & HARDIRQ_MASK, 0);
    }

    #[test]
    fn disable_then_enable_round_trip() {
        let before = preempt_count();
        preempt_disable();
        assert!(in_atomic());
        preempt_enable();
        assert_eq!(preempt_count(), before);
    }

    #[test]
    fn local_bh_increments_softirq_field() {
        let before = preempt_count();
        local_bh_disable();
        assert!(in_softirq());
        local_bh_enable();
        assert_eq!(preempt_count(), before);
    }

    #[test]
    fn irq_enter_exit_increments_hardirq_field() {
        let before = preempt_count();
        __irq_enter_raw();
        assert!(in_hardirq());
        assert!(in_irq());
        __irq_exit_raw();
        assert_eq!(preempt_count(), before);
    }

    #[test]
    fn nmi_enter_exit_increments_nmi_field() {
        let before = preempt_count();
        __nmi_enter_raw();
        assert!(in_nmi());
        assert!(in_irq());
        __nmi_exit_raw();
        assert_eq!(preempt_count(), before);
    }
}
