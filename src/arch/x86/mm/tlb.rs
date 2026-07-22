//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/tlb.c
//! test-origin: linux:vendor/linux/arch/x86/mm/tlb.c
//! Permanent TLB shootdown support for SMP.
//!
//! Remote invalidation is driven by one descriptor per CPU. Callers exclusively
//! own a target descriptor from payload publication through acknowledgement,
//! bump its generation, and send `TLB_SHOOTDOWN_VECTOR`. The remote CPU
//! acknowledges by copying `generation -> ack` after invalidation. Waiters also
//! service their own pending descriptor so reciprocal shootdowns make progress
//! while entered through an IF-clearing exception gate. If the target publishes
//! that it has left the requested mm before servicing the fixed descriptor, the
//! owner completes the stale request exactly as Linux `flush_tlb_func()` would
//! after observing a wrong `loaded_mm`.
//!
//! Linux uses per-mm TLB generations and its generic call-function queues.
//! Lupos retains fixed per-CPU descriptors, with exclusive ownership providing
//! equivalent no-overwrite and completion ordering for the supported topology.
//! The x86 lazy-TLB bit is still tracked explicitly so ordinary range flushes
//! can skip kernel threads borrowing a user CR3, matching `should_flush_tlb()`.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::arch::x86::kernel::apic;
use crate::kernel::sched::MAX_CPUS;
use crate::mm::mm_types::MmStruct;

use super::paging::{PAGE_MASK, PAGE_SIZE};

/// Total number of TLB shootdown IPIs received across all CPUs.
pub static TLB_SHOOTDOWN_COUNT: AtomicU64 = AtomicU64::new(0);

/// Number of acknowledgements observed by callers waiting on remote flushes.
pub static TLB_SHOOTDOWN_ACK_COUNT: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
struct ShootdownDesc {
    /// Serializes multiple source CPUs targeting this descriptor. Linux's
    /// call-function queue gives each request distinct storage; this ownership
    /// bit provides the equivalent no-overwrite guarantee for Lupos's fixed
    /// one-descriptor representation.
    owned: AtomicBool,
    generation: AtomicU64,
    ack: AtomicU64,
    mm: AtomicUsize,
    start: AtomicU64,
    end: AtomicU64,
    full: AtomicU32,
}

impl ShootdownDesc {
    const fn new() -> Self {
        Self {
            owned: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            ack: AtomicU64::new(0),
            mm: AtomicUsize::new(0),
            start: AtomicU64::new(0),
            end: AtomicU64::new(0),
            full: AtomicU32::new(0),
        }
    }
}

static DESCRIPTORS: [ShootdownDesc; MAX_CPUS] = [const { ShootdownDesc::new() }; MAX_CPUS];
static ACTIVE_MM: [AtomicUsize; MAX_CPUS] = [const { AtomicUsize::new(0) }; MAX_CPUS];
static ACTIVE_MM_LAZY: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];

/// Matches Linux's `LOADED_MM_SWITCHING` sentinel.
///
/// A target carrying this value may still have either the outgoing or incoming
/// CR3 loaded, so every address-space-specific request must target it
/// conservatively.
const ACTIVE_MM_SWITCHING: usize = usize::MAX;

/// A private temporary kernel address space is loaded on this CPU.
///
/// Linux publishes the temporary `mm_struct` after `use_temporary_mm()` has
/// completed its `switch_mm_irqs_off()` transition. Lupos's EFI page-table root
/// has no `MmStruct`, so this sentinel carries the equivalent information:
/// NMI user access must reject it, user-mm invalidations need not target it,
/// and kernel/global invalidations still do.
const ACTIVE_MM_TEMPORARY: usize = usize::MAX - 1;

#[cfg(test)]
static LOCAL_FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);

const FULL_FLUSH_PAGE_THRESHOLD: u64 = 32;

pub fn init() {}

pub unsafe fn set_active_mm(cpu: u32, mm: *mut MmStruct) {
    debug_assert_ne!(mm as usize, ACTIVE_MM_SWITCHING);
    debug_assert_ne!(mm as usize, ACTIVE_MM_TEMPORARY);
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM_LAZY[cpu].store(false, Ordering::Release);
    ACTIVE_MM[cpu].store(mm as usize, Ordering::Release);
}

/// Publish the CR3 transition window before changing address spaces.
///
/// Ref: Linux `switch_mm_irqs_off()` publishes `LOADED_MM_SWITCHING` before
/// loading a different CR3.
pub unsafe fn set_active_mm_switching(cpu: u32) {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM_LAZY[cpu].store(false, Ordering::Release);
    ACTIVE_MM[cpu].store(ACTIVE_MM_SWITCHING, Ordering::Release);
}

/// Publish that a private temporary kernel page-table root is loaded.
///
/// This is the post-CR3 state corresponding to Linux's temporary `mm_struct`.
/// Callers must publish [`ACTIVE_MM_SWITCHING`] and execute the ordered CR3
/// transition before entering this state.
pub unsafe fn set_active_mm_temporary(cpu: u32) {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM_LAZY[cpu].store(false, Ordering::Release);
    ACTIVE_MM[cpu].store(ACTIVE_MM_TEMPORARY, Ordering::Release);
}

/// Mark that this CPU is lazily borrowing a user address space.
///
/// Ref: x86 `enter_lazy_tlb()` stores `cpu_tlbstate_shared.is_lazy = true`
/// while keeping `loaded_mm` as the borrowed user mm.  Ordinary remote
/// invalidations skip this CPU; freed-table invalidations still target it.
pub unsafe fn enter_lazy_tlb(cpu: u32, mm: *mut MmStruct) {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    if mm.is_null() {
        ACTIVE_MM_LAZY[cpu].store(false, Ordering::Release);
        ACTIVE_MM[cpu].store(0, Ordering::Release);
        return;
    }
    ACTIVE_MM[cpu].store(mm as usize, Ordering::Release);
    ACTIVE_MM_LAZY[cpu].store(true, Ordering::Release);
}

/// Reactivate a lazily borrowed mm without reloading CR3.
///
/// Linux compares the CPU's local TLB generation with `mm->context.tlb_gen`
/// when `switch_mm_irqs_off()` returns from lazy mode to the same `loaded_mm`.
/// Lupos does not yet carry those per-mm generations, so a stale lazy CPU must
/// conservatively perform one full local flush before returning to userspace.
pub unsafe fn reactivate_lazy_tlb(cpu: u32, mm: *mut MmStruct) {
    debug_assert!(!mm.is_null(), "reactivate_lazy_tlb requires an mm");
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    if active_mm_state(cpu as u32) != mm as usize {
        unsafe {
            set_active_mm(cpu as u32, mm);
        }
        return;
    }
    if ACTIVE_MM_LAZY[cpu].swap(false, Ordering::AcqRel) {
        unsafe {
            flush_local_range(0, 0, true);
        }
    }
    ACTIVE_MM[cpu].store(mm as usize, Ordering::Release);
}

#[inline]
fn active_mm_state(cpu: u32) -> usize {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM[cpu].load(Ordering::Acquire)
}

#[inline]
fn active_mm_is_lazy(cpu: u32) -> bool {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM_LAZY[cpu].load(Ordering::Acquire)
}

pub fn active_mm(cpu: u32) -> *mut MmStruct {
    let state = active_mm_state(cpu);
    if state == ACTIVE_MM_SWITCHING || state == ACTIVE_MM_TEMPORARY {
        core::ptr::null_mut()
    } else {
        state as *mut MmStruct
    }
}

/// Return whether `cpu` has exactly `mm` published as its loaded user mm.
///
/// Unlike [`active_mm`], this predicate deliberately treats a null state and
/// both private sentinel states as mismatches. Linux's `nmi_uaccess_okay()`
/// relies on that conservative distinction to reject an NMI user copy during
/// either half of a CR3/current-task transition or while a temporary mm is
/// loaded.
#[inline]
pub fn loaded_mm_matches(cpu: u32, mm: *mut MmStruct) -> bool {
    !mm.is_null() && active_mm_state(cpu) == mm as usize
}

#[inline]
fn flush_applies_to_active_state(mm: *mut MmStruct, state: usize) -> bool {
    if mm.is_null() || state == ACTIVE_MM_SWITCHING {
        return true;
    }
    if state == ACTIVE_MM_TEMPORARY {
        return false;
    }
    state == mm as usize
}

#[inline]
fn remote_flush_applies_to_cpu(mm: *mut MmStruct, cpu: u32, include_lazy: bool) -> bool {
    let state = active_mm_state(cpu);
    // Match Linux `should_flush_tlb()`: observe `loaded_mm`, then order
    // against `is_lazy` so the lazy bit is at least as new as the mm state.
    core::sync::atomic::fence(Ordering::Acquire);
    if !include_lazy && active_mm_is_lazy(cpu) {
        return false;
    }
    flush_applies_to_active_state(mm, state)
}

#[cfg(not(test))]
#[inline]
unsafe fn invlpg(addr: u64) {
    unsafe {
        core::arch::asm!(
            "invlpg [{0}]",
            in(reg) addr,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
unsafe fn invlpg(_addr: u64) {}

#[cfg(not(test))]
#[inline]
unsafe fn local_full_flush() {
    let cr3 = crate::arch::x86::mm::paging::read_cr3();
    unsafe {
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) cr3,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
unsafe fn local_full_flush() {}

#[inline]
unsafe fn flush_local_range(start: u64, end: u64, full: bool) {
    #[cfg(test)]
    LOCAL_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);

    let start = start & PAGE_MASK;
    let end = end.max(start.saturating_add(PAGE_SIZE));
    let pages = end.saturating_sub(start).div_ceil(PAGE_SIZE);
    if full || pages >= FULL_FLUSH_PAGE_THRESHOLD {
        unsafe { local_full_flush() };
        return;
    }
    let mut addr = start;
    while addr < end {
        unsafe { invlpg(addr) };
        addr = addr.saturating_add(PAGE_SIZE);
    }
}

#[inline]
fn cpu_index() -> usize {
    crate::arch::x86::kernel::setup_percpu::current_cpu_number()
}

pub fn on_shootdown_ipi() {
    let cpu = cpu_index();
    // x86 interrupt gates enter with IF clear. Keep the non-reentrant service
    // routine in that state until its acknowledgement is published, matching
    // Linux flush_tlb_func().
    service_pending_for_cpu_irqs_off(cpu);
    TLB_SHOOTDOWN_COUNT.fetch_add(1, Ordering::Release);
    unsafe {
        apic::eoi();
    }
}

/// Apply and acknowledge the currently published request for `cpu`.
///
/// Linux's `flush_tlb_func()` cannot be re-entered and requires IRQs disabled.
/// The caller must preserve that invariant until the acknowledgement store:
/// otherwise a nested IPI can service a newer generation before the outer
/// invocation stores its older generation, moving `ack` backwards.
fn service_pending_for_cpu_irqs_off(cpu: usize) -> bool {
    debug_assert!(crate::kernel::locking::irqs_disabled());
    let cpu = cpu.min(MAX_CPUS - 1);
    let desc = &DESCRIPTORS[cpu];
    let generation = desc.generation.load(Ordering::Acquire);
    let ack = desc.ack.load(Ordering::Acquire);
    if generation > ack {
        let mm = desc.mm.load(Ordering::Relaxed) as *mut MmStruct;
        let start = desc.start.load(Ordering::Acquire);
        let end = desc.end.load(Ordering::Acquire);
        let full = desc.full.load(Ordering::Acquire) != 0;
        // Linux's flush_tlb_func() drops a stale request when the target CPU
        // has switched to a different mm.  Flushing the newly loaded address
        // space is unnecessary and turns teardown-heavy workloads into a
        // stream of unrelated CR3 reloads. A null mm remains the flush-all
        // request used by the compatibility path.
        if flush_applies_to_active_state(mm, active_mm_state(cpu as u32)) {
            unsafe {
                flush_local_range(start, end, full);
            }
        }
        desc.ack.store(generation, Ordering::Release);
        TLB_SHOOTDOWN_ACK_COUNT.fetch_add(1, Ordering::Release);
        true
    } else {
        false
    }
}

/// Service a local descriptor from a polling path that may have IF enabled.
///
/// The unlocked precheck keeps the common no-request spin path free of
/// `pushfq; cli; popfq`. A request arriving just after the precheck is handled
/// by the next poll or its pending IPI.
fn service_pending_for_cpu(cpu: usize) -> bool {
    let cpu = cpu.min(MAX_CPUS - 1);
    let desc = &DESCRIPTORS[cpu];
    if desc.generation.load(Ordering::Acquire) <= desc.ack.load(Ordering::Acquire) {
        return false;
    }

    let irq_flags = crate::kernel::locking::local_irq_save();
    let serviced = service_pending_for_cpu_irqs_off(cpu);
    crate::kernel::locking::local_irq_restore(irq_flags);
    serviced
}

fn try_acquire_descriptor(cpu: usize) -> bool {
    DESCRIPTORS[cpu.min(MAX_CPUS - 1)]
        .owned
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
}

fn acquire_descriptor(cpu: usize) {
    while !try_acquire_descriptor(cpu) {
        // A target can simultaneously be waiting on this CPU with IF clear.
        // Service its request directly before retrying descriptor ownership.
        service_pending_for_cpu(cpu_index());
        core::hint::spin_loop();
    }
}

fn release_descriptor(cpu: usize) {
    let desc = &DESCRIPTORS[cpu.min(MAX_CPUS - 1)];
    debug_assert_eq!(
        desc.generation.load(Ordering::Acquire),
        desc.ack.load(Ordering::Acquire),
        "shootdown descriptor released before acknowledgement"
    );
    desc.owned.store(false, Ordering::Release);
}

/// Publish one request while the caller exclusively owns the target
/// descriptor. Ownership must remain held until `ack >= generation`.
fn publish_remote_flush_owned(
    cpu: u32,
    mm: *mut MmStruct,
    start: u64,
    end: u64,
    full: bool,
) -> u64 {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    let desc = &DESCRIPTORS[cpu];
    debug_assert!(desc.owned.load(Ordering::Relaxed));
    debug_assert_eq!(
        desc.generation.load(Ordering::Acquire),
        desc.ack.load(Ordering::Acquire)
    );
    let generation = desc
        .generation
        .load(Ordering::Relaxed)
        .checked_add(1)
        .expect("TLB shootdown generation overflow");
    desc.mm.store(mm as usize, Ordering::Relaxed);
    desc.start.store(start, Ordering::Relaxed);
    desc.end.store(end, Ordering::Relaxed);
    desc.full.store(full as u32, Ordering::Relaxed);
    desc.generation.store(generation, Ordering::Release);
    generation
}

/// Send a TLB shootdown IPI to a dense logical CPU.
///
/// Linux's TLB code operates on logical CPU masks; the x86 APIC backend then
/// translates each CPU through `x86_cpu_to_apicid`. Return `false` if the
/// logical CPU has no physical destination.
unsafe fn send_shootdown_ipi(cpu: u32) -> bool {
    #[cfg(test)]
    {
        let _ = cpu;
        true
    }
    #[cfg(not(test))]
    {
        let Some(apic_id) = crate::arch::x86::kernel::smp::logical_cpu_to_apic_id(cpu) else {
            return false;
        };
        unsafe {
            apic::send_ipi(apic_id, crate::arch::x86::kernel::idt::TLB_SHOOTDOWN_VECTOR);
        }
        true
    }
}

fn complete_if_target_no_longer_needs_flush(
    cpu: usize,
    generation: u64,
    mm: *mut MmStruct,
    include_lazy: bool,
) -> bool {
    if mm.is_null() {
        return false;
    }
    if remote_flush_applies_to_cpu(mm, cpu as u32, include_lazy) {
        return false;
    }

    let desc = &DESCRIPTORS[cpu.min(MAX_CPUS - 1)];
    let ack = desc.ack.load(Ordering::Acquire);
    if ack >= generation {
        return true;
    }
    desc.ack.store(generation, Ordering::Release);
    TLB_SHOOTDOWN_ACK_COUNT.fetch_add(1, Ordering::Release);
    true
}

fn wait_for_remote_flushes(
    waits: &[(usize, u64); MAX_CPUS],
    wait_len: usize,
    mm: *mut MmStruct,
    include_lazy: bool,
) {
    #[cfg(test)]
    {
        for &(cpu, generation) in waits.iter().take(wait_len) {
            if complete_if_target_no_longer_needs_flush(cpu, generation, mm, include_lazy) {
                continue;
            }
            assert!(
                service_pending_for_cpu(cpu),
                "test shootdown target had no published request"
            );
        }
    }

    #[cfg(not(test))]
    {
        loop {
            // The caller may have entered through an interrupt gate with IF
            // clear while another CPU waits on us. Make that request progress
            // before checking our own remote acknowledgements.
            service_pending_for_cpu(cpu_index());

            let mut complete = true;
            for &(cpu, generation) in waits.iter().take(wait_len) {
                if DESCRIPTORS[cpu].ack.load(Ordering::Acquire) >= generation {
                    continue;
                }
                if complete_if_target_no_longer_needs_flush(cpu, generation, mm, include_lazy) {
                    continue;
                }
                complete = false;
                break;
            }
            if complete {
                break;
            }
            core::hint::spin_loop();
        }
    }

    for &(cpu, generation) in waits.iter().take(wait_len) {
        assert!(
            DESCRIPTORS[cpu].ack.load(Ordering::Acquire) >= generation,
            "TLB shootdown descriptor lost its acknowledgement"
        );
        release_descriptor(cpu);
    }
}

unsafe fn flush_tlb_mm_range_inner(
    mm: *mut MmStruct,
    start: u64,
    end: u64,
    include_lazy: bool,
) -> bool {
    // Linux's inc_mm_tlb_gen() is also the full barrier that pairs a page-table
    // update with switch_mm_irqs_off(). Lupos has no per-mm generation yet, so
    // retain the ordering half explicitly: either the scan observes a CPU in
    // this mm (or SWITCHING) and shoots it down, or the PTE update is globally
    // visible before that CPU publishes the mm and serializes with MOV-to-CR3.
    core::sync::atomic::fence(Ordering::SeqCst);

    let full = end <= start;
    let this_cpu = crate::kernel::sched::current_cpu();
    // Linux performs the local invalidation only when this CPU currently has
    // the requested mm loaded.  The caller may be tearing down a different
    // process while running in its own address space.
    if flush_applies_to_active_state(mm, active_mm_state(this_cpu)) {
        unsafe {
            flush_local_range(start, end, full);
        }
    }
    let mut targeted = false;
    let mut wait: [(usize, u64); MAX_CPUS] = [(0, 0); MAX_CPUS];

    for cpu in 0..MAX_CPUS as u32 {
        if cpu == this_cpu {
            continue;
        }
        if mm.is_null() && !crate::kernel::sched::cpu_active_mask().test(cpu) {
            continue;
        }
        if !remote_flush_applies_to_cpu(mm, cpu, include_lazy) {
            continue;
        }
        targeted = true;
        acquire_descriptor(cpu as usize);
        let generation = publish_remote_flush_owned(cpu, mm, start, end, full);
        wait[0] = (cpu as usize, generation);
        assert!(
            unsafe { send_shootdown_ipi(cpu) },
            "active CPU has no APIC destination for TLB shootdown"
        );
        // Linux's call-function queue gives each request independent storage.
        // Lupos has one descriptor per target CPU, so release each target
        // before acquiring another to avoid descriptor-ownership cycles among
        // concurrent flushers.
        wait_for_remote_flushes(&wait, 1, mm, include_lazy);
    }

    if !targeted {
        return true;
    }
    true
}

/// Invalidate translations in one mm.
///
/// Ordinary range flushes skip CPUs in Linux lazy-TLB state; the CPU catches up
/// on the next real-mm context switch.
pub unsafe fn flush_tlb_mm_range(mm: *mut MmStruct, start: u64, end: u64) -> bool {
    // Linux `flush_tlb_mm_range()` uses `get_cpu()` before it snapshots the
    // initiating CPU and publishes per-CPU flush state. Pin the caller for the
    // same reason: wait loops service this CPU's descriptor while remote CPUs
    // may be waiting on it.
    crate::kernel::locking::preempt::preempt_disable();
    let result = unsafe { flush_tlb_mm_range_inner(mm, start, end, false) };
    crate::kernel::locking::preempt::preempt_enable();
    result
}

/// Invalidate translations before freeing page-table hierarchy.
///
/// Linux must still target CPUs in lazy-TLB mode when page-table hierarchy can
/// be freed, because speculative accesses may otherwise touch reused memory.
pub unsafe fn flush_tlb_mm_range_freed_tables(mm: *mut MmStruct, start: u64, end: u64) -> bool {
    crate::kernel::locking::preempt::preempt_disable();
    let result = unsafe { flush_tlb_mm_range_inner(mm, start, end, true) };
    crate::kernel::locking::preempt::preempt_enable();
    result
}

/// Compatibility helper retained for the Milestone 6 shootdown test.
///
/// `cpus` contains dense logical CPU numbers, matching Linux cpumask
/// semantics. The APIC destination translation happens in
/// [`send_shootdown_ipi`].
#[cfg(feature = "test-tlb-shootdown")]
pub unsafe fn flush_tlb_others(cpus: &[u8], addr: u64) -> bool {
    let mm = active_mm(crate::kernel::sched::current_cpu());
    let mut waits: [(usize, u64); MAX_CPUS] = [(0, 0); MAX_CPUS];
    let mut wait_len = 0usize;
    let mut targets = 0u64;

    for &cpu in cpus {
        if cpu as usize >= MAX_CPUS {
            return false;
        }
        targets |= 1u64 << cpu;
    }

    // A cpumask has unique, ascending CPU members. Canonicalize the
    // compatibility slice to that same lock order so two callers cannot form
    // a descriptor-ownership cycle with differently ordered input.
    for cpu in 0..MAX_CPUS {
        if targets & (1u64 << cpu) == 0 {
            continue;
        }
        acquire_descriptor(cpu);
        let generation = publish_remote_flush_owned(cpu as u32, mm, addr, addr + PAGE_SIZE, false);
        waits[wait_len] = (cpu, generation);
        wait_len += 1;
        assert!(
            unsafe { send_shootdown_ipi(cpu as u32) },
            "requested CPU has no APIC destination for TLB shootdown"
        );
    }

    wait_for_remote_flushes(&waits, wait_len, mm, false);
    true
}

#[cfg(feature = "test-tlb-shootdown")]
pub fn run_shootdown_test(cpus: &[crate::arch::x86::kernel::acpi::CpuInfo]) {
    let bsp_id = unsafe { apic::id() };
    let target = cpus.iter().find(|c| c.enabled && c.apic_id != bsp_id);
    let Some(ap) = target else {
        panic!("tlb: no non-BSP CPU available for shootdown test");
    };
    let logical_cpu = (1..MAX_CPUS as u32)
        .find(|&cpu| crate::arch::x86::kernel::smp::logical_cpu_to_apic_id(cpu) == Some(ap.apic_id))
        .unwrap_or_else(|| {
            panic!(
                "tlb: APIC ID {} has no dense logical CPU mapping",
                ap.apic_id
            )
        });

    let cpus_to_flush = [logical_cpu as u8];
    let ok = unsafe { flush_tlb_others(&cpus_to_flush, 0) };
    if !ok {
        panic!(
            "tlb: shootdown IPI to AP {} could not be queued",
            ap.apic_id
        );
    }

    crate::kernel::printk::log_info!(
        "tlb",
        "tlb: shootdown IPI delivered (ap={}, count={})",
        ap.apic_id,
        TLB_SHOOTDOWN_COUNT.load(Ordering::Acquire)
    );

    #[cfg(feature = "qemu-test")]
    unsafe {
        crate::linux_driver_abi::platform::qemu::exit_success();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn test_guard() -> spin::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock();
        TLB_SHOOTDOWN_COUNT.store(0, Ordering::SeqCst);
        TLB_SHOOTDOWN_ACK_COUNT.store(0, Ordering::SeqCst);
        LOCAL_FLUSH_COUNT.store(0, Ordering::SeqCst);
        for desc in &DESCRIPTORS {
            desc.owned.store(false, Ordering::SeqCst);
            desc.generation.store(0, Ordering::SeqCst);
            desc.ack.store(0, Ordering::SeqCst);
            desc.mm.store(0, Ordering::SeqCst);
            desc.start.store(0, Ordering::SeqCst);
            desc.end.store(0, Ordering::SeqCst);
            desc.full.store(0, Ordering::SeqCst);
        }
        for mm in &ACTIVE_MM {
            mm.store(0, Ordering::SeqCst);
        }
        for lazy in &ACTIVE_MM_LAZY {
            lazy.store(false, Ordering::SeqCst);
        }
        guard
    }

    #[test]
    fn shootdown_counter_starts_at_zero() {
        let _guard = test_guard();
        assert_eq!(TLB_SHOOTDOWN_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ack_counter_starts_at_zero() {
        let _guard = test_guard();
        assert_eq!(TLB_SHOOTDOWN_ACK_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn active_mm_round_trip() {
        let _guard = test_guard();
        let ptr = 0x1234usize as *mut MmStruct;
        unsafe { set_active_mm(2, ptr) };
        assert_eq!(active_mm(2), ptr);
    }

    #[test]
    fn switching_state_is_targeted_conservatively() {
        let _guard = test_guard();
        let requested = 0x1234usize as *mut MmStruct;
        let unrelated = 0x5678usize as *mut MmStruct;

        unsafe {
            set_active_mm_switching(2);
        }

        assert!(flush_applies_to_active_state(requested, active_mm_state(2)));
        assert!(flush_applies_to_active_state(unrelated, active_mm_state(2)));
        assert!(
            active_mm(2).is_null(),
            "the switching sentinel must not escape as an MmStruct pointer"
        );
        assert!(
            !loaded_mm_matches(2, requested),
            "NMI user access must reject the CR3 transition window"
        );
    }

    #[test]
    fn temporary_mm_rejects_nmi_uaccess_and_skips_user_mm_flushes() {
        // Origin: vendor/linux/arch/x86/mm/tlb.c::use_temporary_mm and
        // should_flush_tlb. Linux publishes the private temporary mm after the
        // CR3 transition, so it cannot match current->mm and an unrelated
        // user-mm invalidation does not target this CPU.
        let _guard = test_guard();
        let requested = 0x1234usize as *mut MmStruct;

        unsafe {
            set_active_mm_temporary(2);
        }

        assert!(
            active_mm(2).is_null(),
            "a private temporary root must not escape as an MmStruct pointer"
        );
        assert!(!loaded_mm_matches(2, requested));
        assert!(!flush_applies_to_active_state(
            requested,
            active_mm_state(2)
        ));
        assert!(unsafe { flush_tlb_mm_range(requested, 0x3000, 0x5000) });
        assert_eq!(
            DESCRIPTORS[2].generation.load(Ordering::Acquire),
            0,
            "an unrelated user-mm flush must not publish work to a CPU in a private temporary mm"
        );
        assert!(flush_applies_to_active_state(
            core::ptr::null_mut(),
            active_mm_state(2)
        ));
    }

    #[test]
    fn loaded_mm_match_requires_the_exact_non_null_mm() {
        let _guard = test_guard();
        let loaded = 0x1234usize as *mut MmStruct;
        let unrelated = 0x5678usize as *mut MmStruct;
        unsafe {
            set_active_mm(1, loaded);
        }

        assert!(loaded_mm_matches(1, loaded));
        assert!(!loaded_mm_matches(1, unrelated));
        assert!(!loaded_mm_matches(1, core::ptr::null_mut()));
    }

    #[test]
    fn null_request_is_the_flush_all_state() {
        let _guard = test_guard();
        let loaded = 0x1234usize as *mut MmStruct;
        unsafe {
            set_active_mm(1, loaded);
        }

        assert!(flush_applies_to_active_state(
            core::ptr::null_mut(),
            active_mm_state(1)
        ));
    }

    #[test]
    fn descriptor_publish_advances_generation() {
        let _guard = test_guard();
        acquire_descriptor(1);
        let generation =
            publish_remote_flush_owned(1, core::ptr::null_mut(), 0x1000, 0x2000, false);
        assert!(generation >= 1);
        assert_eq!(
            DESCRIPTORS[1].generation.load(Ordering::Acquire),
            generation
        );
        assert!(service_pending_for_cpu(1));
        release_descriptor(1);
    }

    #[test]
    fn flush_targets_only_cpus_running_the_same_mm() {
        let _guard = test_guard();
        let mm = 0x1234usize as *mut MmStruct;
        let other = 0x5678usize as *mut MmStruct;

        unsafe {
            set_active_mm(1, mm);
            set_active_mm(2, other);
        }
        DESCRIPTORS[1].generation.store(0, Ordering::Release);
        DESCRIPTORS[1].ack.store(0, Ordering::Release);
        DESCRIPTORS[2].generation.store(0, Ordering::Release);
        DESCRIPTORS[2].ack.store(0, Ordering::Release);

        assert!(unsafe { flush_tlb_mm_range(mm, 0x3000, 0x5000) });
        assert_eq!(
            DESCRIPTORS[1].ack.load(Ordering::Acquire),
            DESCRIPTORS[1].generation.load(Ordering::Acquire)
        );
        assert_eq!(DESCRIPTORS[2].generation.load(Ordering::Acquire), 0);
        assert_eq!(DESCRIPTORS[2].ack.load(Ordering::Acquire), 0);
    }

    #[test]
    fn local_flush_skips_an_unrelated_mm() {
        let _guard = test_guard();
        let requested = 0x1234usize as *mut MmStruct;
        let loaded = 0x5678usize as *mut MmStruct;
        unsafe {
            set_active_mm(0, loaded);
            assert!(flush_tlb_mm_range(requested, 0x3000, 0x5000));
        }
        assert_eq!(LOCAL_FLUSH_COUNT.load(Ordering::Acquire), 0);
    }

    #[test]
    fn enter_lazy_tlb_skips_ordinary_remote_user_mm_flush() {
        let _guard = test_guard();
        let mm = 0x1234usize as *mut MmStruct;
        unsafe {
            set_active_mm(1, mm);
            enter_lazy_tlb(1, mm);
            assert!(flush_tlb_mm_range(mm, 0x3000, 0x5000));
        }

        assert_eq!(
            DESCRIPTORS[1].generation.load(Ordering::Acquire),
            0,
            "x86 should_flush_tlb() skips lazy remote CPUs for ordinary flushes"
        );
        assert_eq!(LOCAL_FLUSH_COUNT.load(Ordering::Acquire), 0);
    }

    #[test]
    fn reactivating_same_lazy_mm_flushes_without_tlb_generations() {
        // test-origin: linux:vendor/linux/arch/x86/mm/tlb.c:switch_mm_irqs_off
        let _guard = test_guard();
        let mm = 0x1234usize as *mut MmStruct;
        unsafe {
            set_active_mm(0, mm);
            enter_lazy_tlb(0, mm);
            reactivate_lazy_tlb(0, mm);
        }

        assert!(!active_mm_is_lazy(0));
        assert_eq!(
            LOCAL_FLUSH_COUNT.load(Ordering::Acquire),
            1,
            "without Linux's per-mm tlb_gen, lazy reactivation must catch up with a full flush"
        );
    }

    #[test]
    fn hierarchy_free_api_targets_a_borrowed_mm() {
        let _guard = test_guard();
        let mm = 0x1234usize as *mut MmStruct;
        unsafe {
            set_active_mm(1, mm);
            enter_lazy_tlb(1, mm);
            assert!(flush_tlb_mm_range_freed_tables(mm, 0x3000, 0x5000));
        }

        assert_ne!(DESCRIPTORS[1].generation.load(Ordering::Acquire), 0);
        assert_eq!(
            DESCRIPTORS[1].ack.load(Ordering::Acquire),
            DESCRIPTORS[1].generation.load(Ordering::Acquire)
        );
        assert_eq!(LOCAL_FLUSH_COUNT.load(Ordering::Acquire), 1);
    }

    #[test]
    fn stale_remote_request_is_acked_without_flushing_new_mm() {
        let _guard = test_guard();
        let requested = 0x1234usize as *mut MmStruct;
        let loaded = 0x5678usize as *mut MmStruct;
        let cpu = 2usize;
        unsafe {
            set_active_mm(cpu as u32, loaded);
        }
        acquire_descriptor(cpu);
        let generation = publish_remote_flush_owned(cpu as u32, requested, 0x1000, 0x2000, false);

        assert!(service_pending_for_cpu(cpu));
        assert_eq!(DESCRIPTORS[cpu].ack.load(Ordering::Acquire), generation);
        assert_eq!(LOCAL_FLUSH_COUNT.load(Ordering::Acquire), 0);
        release_descriptor(cpu);
    }

    #[test]
    fn waiter_completes_stale_descriptor_after_target_leaves_mm() {
        // test-origin: linux:vendor/linux/arch/x86/mm/tlb.c:flush_tlb_func
        // Lupos-specific harness: Linux's call-function queue owns the remote
        // request storage, but Lupos has fixed per-target descriptors.  The
        // owner must mirror Linux's wrong-loaded-mm return path when the target
        // publishes a new mm before servicing the descriptor.
        let _guard = test_guard();
        let requested = 0x1234usize as *mut MmStruct;
        let loaded = 0x5678usize as *mut MmStruct;
        let cpu = 2usize;
        let mut waits = [(0usize, 0u64); MAX_CPUS];

        unsafe {
            set_active_mm(cpu as u32, requested);
        }
        acquire_descriptor(cpu);
        let generation = publish_remote_flush_owned(cpu as u32, requested, 0x1000, 0x2000, false);
        waits[0] = (cpu, generation);

        unsafe {
            set_active_mm(cpu as u32, loaded);
        }

        wait_for_remote_flushes(&waits, 1, requested, false);

        assert_eq!(DESCRIPTORS[cpu].ack.load(Ordering::Acquire), generation);
        assert!(!DESCRIPTORS[cpu].owned.load(Ordering::Acquire));
        assert_eq!(
            LOCAL_FLUSH_COUNT.load(Ordering::Acquire),
            0,
            "a CPU that has left the mm must not flush its new address space"
        );
    }

    #[test]
    fn target_descriptor_cannot_be_overwritten_before_ack() {
        let _guard = test_guard();
        let cpu = 3usize;
        assert!(try_acquire_descriptor(cpu));
        let first =
            publish_remote_flush_owned(cpu as u32, core::ptr::null_mut(), 0x1000, 0x2000, false);

        assert!(
            !try_acquire_descriptor(cpu),
            "a second producer must not overwrite an in-flight payload"
        );
        assert_eq!(DESCRIPTORS[cpu].start.load(Ordering::Acquire), 0x1000);
        assert_eq!(DESCRIPTORS[cpu].generation.load(Ordering::Acquire), first);

        assert!(service_pending_for_cpu(cpu));
        release_descriptor(cpu);
        assert!(try_acquire_descriptor(cpu));
        release_descriptor(cpu);
    }

    #[test]
    fn reciprocal_if_disabled_waits_can_service_local_requests() {
        let _guard = test_guard();
        acquire_descriptor(0);
        acquire_descriptor(1);
        let to_cpu0 = publish_remote_flush_owned(0, core::ptr::null_mut(), 0x3000, 0x4000, false);
        let to_cpu1 = publish_remote_flush_owned(1, core::ptr::null_mut(), 0x5000, 0x6000, false);

        // These direct service calls are what each IF-disabled waiter performs
        // while the reciprocal IPI remains pending in the LAPIC.
        assert!(service_pending_for_cpu(0));
        assert!(service_pending_for_cpu(1));
        assert_eq!(DESCRIPTORS[0].ack.load(Ordering::Acquire), to_cpu0);
        assert_eq!(DESCRIPTORS[1].ack.load(Ordering::Acquire), to_cpu1);
        release_descriptor(0);
        release_descriptor(1);
    }

    #[test]
    fn range_flush_releases_each_fixed_descriptor_before_acquiring_next() {
        // test-origin: linux:vendor/linux/arch/x86/mm/tlb.c:native_flush_tlb_multi
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/mm/tlb.c"
        ));
        assert!(linux.contains("on_each_cpu_cond_mask(should_flush_tlb, flush_tlb_func"));

        let source = include_str!("tlb.rs");
        let body = source
            .split("unsafe fn flush_tlb_mm_range_inner(")
            .nth(1)
            .and_then(|s| s.split("/// Invalidate translations in one mm.").next())
            .expect("flush_tlb_mm_range_inner body present");
        let publish = body
            .find("publish_remote_flush_owned(cpu, mm, start, end, full)")
            .expect("range flush must publish a remote request");
        let wait = body
            .find("wait_for_remote_flushes(&wait, 1, mm, include_lazy);")
            .expect("range flush must wait per target descriptor");
        let loop_end = body[wait..]
            .find("}\n\n    if !targeted")
            .expect("per-target wait should remain inside the CPU loop")
            + wait;
        assert!(publish < wait);
        assert!(wait < loop_end);
        assert!(body.contains("one descriptor per target CPU"));
    }

    #[test]
    fn range_flush_entry_pins_initiating_cpu_like_linux_get_cpu() {
        // test-origin: linux:vendor/linux/arch/x86/mm/tlb.c:flush_tlb_mm_range
        let linux = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/mm/tlb.c"
        ));
        assert!(linux.contains("int cpu = get_cpu();"));
        assert!(linux.contains("put_cpu();"));

        let source = include_str!("tlb.rs");
        for entry in [
            "pub unsafe fn flush_tlb_mm_range(mm: *mut MmStruct, start: u64, end: u64) -> bool",
            "pub unsafe fn flush_tlb_mm_range_freed_tables(mm: *mut MmStruct, start: u64, end: u64) -> bool",
        ] {
            let body = source
                .split(entry)
                .nth(1)
                .and_then(|s| s.split("\n}\n").next())
                .expect("TLB flush wrapper body present");
            let disable = body
                .find("preempt::preempt_disable()")
                .expect("TLB flush wrapper must pin the initiating CPU");
            let inner = body
                .find("flush_tlb_mm_range_inner")
                .expect("TLB flush wrapper must call the shared implementation");
            let enable = body
                .find("preempt::preempt_enable()")
                .expect("TLB flush wrapper must unpin the initiating CPU");
            assert!(disable < inner);
            assert!(inner < enable);
        }
    }
}
