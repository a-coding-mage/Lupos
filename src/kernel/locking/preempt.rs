//! linux-parity: partial
//! linux-source: vendor/linux/include/linux/preempt.h
//! linux-source: vendor/linux/arch/x86/include/asm/preempt.h
//! test-origin: linux:vendor/linux/include/linux/preempt.h
//! test-origin: linux:vendor/linux/arch/x86/include/asm/preempt.h
//! Preemption count and atomicity predicates (M33).
//!
//! Mirrors `vendor/linux/include/linux/preempt.h`.  The `preempt_count` is a
//! per-CPU `u32` with the following bit layout:
//!
//! ```text
//!   [31]    PREEMPT_NEED_RESCHED (inverted; x86 fast-path state)
//!   [23:20] NMI_MASK      (NMI_BITS = 4)
//!   [19:16] HARDIRQ_MASK  (HARDIRQ_BITS = 4)
//!   [15:8]  SOFTIRQ_MASK  (SOFTIRQ_BITS = 8)
//!   [7:0]   PREEMPT_MASK  (PREEMPT_BITS = 8)
//! ```
//!
//! `in_atomic()` ≡ `preempt_count() != 0`.  Sleeping in atomic context is a
//! bug; `might_sleep()` `WARN_ON`s when violated.
//!
//! Lupos does not yet implement Linux's `CONFIG_PREEMPT_DYNAMIC` static-call
//! selection or fold `TIF_NEED_RESCHED` into bit 31 of the raw per-CPU word.
//! The counter layout and the target's `PREEMPT_VOLUNTARY` enable behavior
//! match Linux, but those missing x86 fast-path pieces keep this file partial.

use core::sync::atomic::Ordering;

use crate::kernel::module::{export_symbol, find_symbol};

pub const PREEMPT_BITS: u32 = 8;
pub const SOFTIRQ_BITS: u32 = 8;
pub const HARDIRQ_BITS: u32 = 4;
pub const NMI_BITS: u32 = 4;

pub const PREEMPT_SHIFT: u32 = 0;
pub const SOFTIRQ_SHIFT: u32 = PREEMPT_SHIFT + PREEMPT_BITS;
pub const HARDIRQ_SHIFT: u32 = SOFTIRQ_SHIFT + SOFTIRQ_BITS;
pub const NMI_SHIFT: u32 = HARDIRQ_SHIFT + HARDIRQ_BITS;

pub const PREEMPT_OFFSET: u32 = 1u32 << PREEMPT_SHIFT;
pub const SOFTIRQ_OFFSET: u32 = 1u32 << SOFTIRQ_SHIFT;
pub const HARDIRQ_OFFSET: u32 = 1u32 << HARDIRQ_SHIFT;
pub const NMI_OFFSET: u32 = 1u32 << NMI_SHIFT;
pub const SOFTIRQ_DISABLE_OFFSET: u32 = 2 * SOFTIRQ_OFFSET;
pub const SOFTIRQ_LOCK_OFFSET: u32 = SOFTIRQ_DISABLE_OFFSET + PREEMPT_OFFSET;

pub const PREEMPT_MASK: u32 = ((1u32 << PREEMPT_BITS) - 1) << PREEMPT_SHIFT;
pub const SOFTIRQ_MASK: u32 = ((1u32 << SOFTIRQ_BITS) - 1) << SOFTIRQ_SHIFT;
pub const HARDIRQ_MASK: u32 = ((1u32 << HARDIRQ_BITS) - 1) << HARDIRQ_SHIFT;
pub const NMI_MASK: u32 = ((1u32 << NMI_BITS) - 1) << NMI_SHIFT;

/// x86 stores inverted `need_resched` in the raw counter's most-significant
/// bit. Lupos does not fold the task flag into this bit yet, so public
/// [`preempt_count`] must continue to expose only the context-count fields.
pub const PREEMPT_NEED_RESCHED: u32 = 0x8000_0000;

#[inline(always)]
const fn visible_preempt_count(raw: u32) -> u32 {
    raw & !PREEMPT_NEED_RESCHED
}

#[inline(always)]
const fn preempt_count_decision(count_after_decrement: u32, need_resched: bool) -> bool {
    count_after_decrement == 0 && need_resched
}

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

#[cfg(any(not(target_arch = "x86_64"), test))]
#[inline]
fn cpu_index() -> usize {
    crate::arch::x86::kernel::setup_percpu::current_cpu_number()
}

#[cfg(any(not(target_arch = "x86_64"), test))]
#[inline]
fn counter() -> &'static core::sync::atomic::AtomicU32 {
    crate::arch::x86::kernel::setup_percpu::preempt_count_slot(cpu_index())
}

/// Linux x86 `raw_cpu_add_4`: one local, non-LOCKed memory instruction.
///
/// Interrupt entry cannot split an instruction on the same CPU, and no other
/// CPU writes this per-CPU slot. A globally serializing LOCK prefix on every
/// spinlock/BH/IRQ transition only adds cross-core cache traffic.
#[inline(always)]
fn counter_add(value: u32) {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        core::arch::asm!(
            "add dword ptr gs:[rip + {percpu_base} + {preempt_offset}], {value:e}",
            percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
            preempt_offset = const crate::arch::x86::kernel::setup_percpu::PREEMPT_COUNT_OFFSET,
            value = in(reg) value,
            options(nostack),
        );
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    {
        counter().fetch_add(value, Ordering::AcqRel);
    }
}

#[inline(always)]
fn counter_sub(value: u32) {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        core::arch::asm!(
            "sub dword ptr gs:[rip + {percpu_base} + {preempt_offset}], {value:e}",
            percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
            preempt_offset = const crate::arch::x86::kernel::setup_percpu::PREEMPT_COUNT_OFFSET,
            value = in(reg) value,
            options(nostack),
        );
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    {
        counter().fetch_sub(value, Ordering::AcqRel);
    }
}

/// Read the current CPU's preempt count.
#[inline]
pub fn preempt_count() -> u32 {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    {
        let count: u32;
        unsafe {
            core::arch::asm!(
                "mov {count:e}, dword ptr gs:[rip + {percpu_base} + {preempt_offset}]",
                count = lateout(reg) count,
                percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
                preempt_offset = const crate::arch::x86::kernel::setup_percpu::PREEMPT_COUNT_OFFSET,
                options(nostack, readonly, preserves_flags),
            );
        }
        visible_preempt_count(count)
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    {
        visible_preempt_count(counter().load(Ordering::Acquire))
    }
}

/// Increment the preempt-disable counter.
#[inline]
pub fn preempt_disable() {
    counter_add(PREEMPT_OFFSET);
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
}

/// `sched_preempt_enable_no_resched()` / `preempt_enable_no_resched()`.
///
/// Linux places the compiler barrier before the decrement so accesses in the
/// protected region cannot escape past the point where migration becomes
/// possible. This variant deliberately never invokes the scheduler.
#[inline]
pub fn preempt_enable_no_resched() {
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
    counter_sub(PREEMPT_OFFSET);
}

/// Linux `preempt_count_dec_and_test()`.
///
/// Linux x86 folds inverted `TIF_NEED_RESCHED` into the raw count so `decl`
/// plus a zero test implements this decision in one instruction. Lupos keeps
/// the task flag separate for now, but preserves the same observable decision:
/// report true only when this decrement reaches the outermost preemptible
/// level and the current task needs rescheduling.
///
/// As in Linux, the caller supplies the compiler barrier which precedes this
/// primitive. The `PREEMPT_VOLUNTARY` target does not call it from
/// [`preempt_enable`].
#[inline]
pub fn preempt_count_dec_and_test() -> bool {
    counter_sub(PREEMPT_OFFSET);
    let need_resched = {
        let current = unsafe { crate::kernel::sched::get_current() };
        crate::kernel::sched::task_needs_resched(current)
    };
    preempt_count_decision(preempt_count(), need_resched)
}

/// Decrement the preempt-disable counter.
///
/// The generic x86_64 target follows Linux's `PREEMPT_VOLUNTARY` runtime
/// policy, where the `CONFIG_PREEMPT_DYNAMIC` static call for
/// `preempt_schedule` is disabled. Keep the API distinct from
/// [`preempt_enable_no_resched`] so a future dynamic-mode implementation can
/// add Linux's outermost-decrement reschedule test without changing callers.
#[inline]
pub fn preempt_enable() {
    preempt_enable_no_resched();
}

#[inline]
pub fn local_bh_disable() {
    linux___local_bh_disable_ip(0, SOFTIRQ_DISABLE_OFFSET);
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
}

#[inline]
pub fn local_bh_enable() {
    linux___local_bh_enable_ip(0, SOFTIRQ_DISABLE_OFFSET);
}

/// `__local_bh_disable_ip` — `vendor/linux/kernel/softirq.c`.
#[unsafe(export_name = "__local_bh_disable_ip")]
pub extern "C" fn linux___local_bh_disable_ip(_ip: usize, cnt: u32) {
    counter_add(cnt);
}

/// Linux's private `__local_bh_enable()` decrement.
///
/// Unlike [`linux___local_bh_enable_ip`], this helper deliberately does not
/// process pending softirqs. Linux uses it when leaving active softirq
/// handling, where recursively entering `do_softirq()` would be invalid.
#[inline]
pub(crate) fn local_bh_enable_no_softirq(cnt: u32) {
    counter_sub(cnt);
}

/// `__local_bh_enable_ip` — `vendor/linux/kernel/softirq.c`.
#[unsafe(export_name = "__local_bh_enable_ip")]
pub extern "C" fn linux___local_bh_enable_ip(_ip: usize, cnt: u32) {
    // Linux keeps one PREEMPT_OFFSET held until pending work has drained, so
    // neither migration nor voluntary scheduling can move execution to a
    // different CPU between the local pending-word check and do_softirq().
    counter_sub(cnt.wrapping_sub(PREEMPT_OFFSET));

    if !in_irq() && !in_softirq() && crate::kernel::softirq::local_softirq_pending() != 0 {
        crate::kernel::softirq::do_softirq();
    }

    counter_sub(PREEMPT_OFFSET);
    // Linux ends with preempt_check_resched(). The generic x86_64 target uses
    // PREEMPT_VOLUNTARY, whose selected preempt-schedule static call is a
    // no-op, matching preempt_enable() above.
}

#[inline]
pub fn __irq_enter_raw() {
    counter_add(HARDIRQ_OFFSET);
}

#[inline]
pub fn __irq_exit_raw() {
    counter_sub(HARDIRQ_OFFSET);
}

#[inline]
pub fn __nmi_enter_raw() {
    counter_add(NMI_OFFSET);
}

#[inline]
pub fn __nmi_exit_raw() {
    counter_sub(NMI_OFFSET);
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
        assert_eq!(HARDIRQ_SHIFT, 16);
        assert_eq!(NMI_SHIFT, 20);
        assert_eq!(PREEMPT_NEED_RESCHED, 0x8000_0000);
    }

    #[test]
    fn masks_match_linux_preempt_h_and_are_disjoint() {
        assert_eq!(PREEMPT_MASK, 0x0000_00ff);
        assert_eq!(SOFTIRQ_MASK, 0x0000_ff00);
        assert_eq!(HARDIRQ_MASK, 0x000f_0000);
        assert_eq!(NMI_MASK, 0x00f0_0000);
        assert_eq!(PREEMPT_MASK & SOFTIRQ_MASK, 0);
        assert_eq!(SOFTIRQ_MASK & HARDIRQ_MASK, 0);
        assert_eq!(HARDIRQ_MASK & NMI_MASK, 0);
        assert_eq!(
            (PREEMPT_MASK | SOFTIRQ_MASK | HARDIRQ_MASK | NMI_MASK) & PREEMPT_NEED_RESCHED,
            0
        );
    }

    #[test]
    fn offsets_match_linux_preempt_h() {
        assert_eq!(PREEMPT_OFFSET, 0x0000_0001);
        assert_eq!(SOFTIRQ_OFFSET, 0x0000_0100);
        assert_eq!(SOFTIRQ_DISABLE_OFFSET, 0x0000_0200);
        assert_eq!(SOFTIRQ_LOCK_OFFSET, 0x0000_0201);
        assert_eq!(HARDIRQ_OFFSET, 0x0001_0000);
        assert_eq!(NMI_OFFSET, 0x0010_0000);
    }

    #[test]
    fn public_count_masks_x86_inverted_need_resched_bit() {
        assert_eq!(visible_preempt_count(PREEMPT_NEED_RESCHED), 0);
        assert_eq!(
            visible_preempt_count(PREEMPT_NEED_RESCHED | HARDIRQ_OFFSET | PREEMPT_OFFSET),
            HARDIRQ_OFFSET | PREEMPT_OFFSET
        );
    }

    #[test]
    fn dec_and_test_matches_linux_outermost_need_resched_truth_table() {
        assert!(preempt_count_decision(0, true));
        assert!(!preempt_count_decision(PREEMPT_OFFSET, true));
        assert!(!preempt_count_decision(0, false));
        assert!(!preempt_count_decision(PREEMPT_OFFSET, false));
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
    fn disable_then_enable_no_resched_round_trip() {
        let before = preempt_count();
        preempt_disable();
        assert_eq!(preempt_count(), before + PREEMPT_OFFSET);
        preempt_enable_no_resched();
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
