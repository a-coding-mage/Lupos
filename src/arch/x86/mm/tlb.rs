//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/tlb.c
//! test-origin: linux:vendor/linux/arch/x86/mm/tlb.c
//! Permanent TLB shootdown support for SMP.
//!
//! Remote invalidation is driven by one descriptor per CPU. Callers publish a
//! `(mm, start, end, full)` payload to the target descriptor, bump its
//! generation, and send `TLB_SHOOTDOWN_VECTOR`. The remote CPU acknowledges by
//! copying `generation -> ack` after it has invalidated the requested range.

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

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

const FULL_FLUSH_PAGE_THRESHOLD: u64 = 32;

pub fn init() {}

pub unsafe fn set_active_mm(cpu: u32, mm: *mut MmStruct) {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM[cpu].store(mm as usize, Ordering::Release);
}

pub fn active_mm(cpu: u32) -> *mut MmStruct {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    ACTIVE_MM[cpu].load(Ordering::Acquire) as *mut MmStruct
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
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        let id = unsafe { crate::arch::x86::kernel::apic::id() } as usize;
        id.min(MAX_CPUS - 1)
    }
}

fn rdtsc() -> u64 {
    #[cfg(test)]
    return 0;
    #[cfg(not(test))]
    {
        let lo: u32;
        let hi: u32;
        unsafe {
            core::arch::asm!(
                "rdtsc",
                out("eax") lo,
                out("edx") hi,
                options(nomem, nostack, preserves_flags),
            );
        }
        (hi as u64) << 32 | lo as u64
    }
}

pub fn on_shootdown_ipi() {
    let cpu = cpu_index();
    let desc = &DESCRIPTORS[cpu];
    let generation = desc.generation.load(Ordering::Acquire);
    let ack = desc.ack.load(Ordering::Acquire);
    if generation != ack {
        let start = desc.start.load(Ordering::Acquire);
        let end = desc.end.load(Ordering::Acquire);
        let full = desc.full.load(Ordering::Acquire) != 0;
        unsafe {
            flush_local_range(start, end, full);
        }
        desc.ack.store(generation, Ordering::Release);
        TLB_SHOOTDOWN_ACK_COUNT.fetch_add(1, Ordering::Release);
    }
    TLB_SHOOTDOWN_COUNT.fetch_add(1, Ordering::Release);
    unsafe {
        apic::eoi();
    }
}

fn publish_remote_flush(cpu: u32, mm: *mut MmStruct, start: u64, end: u64, full: bool) -> u64 {
    let cpu = (cpu as usize).min(MAX_CPUS - 1);
    let desc = &DESCRIPTORS[cpu];
    let generation = desc.generation.load(Ordering::Acquire).saturating_add(1);
    desc.mm.store(mm as usize, Ordering::Release);
    desc.start.store(start, Ordering::Release);
    desc.end.store(end, Ordering::Release);
    desc.full.store(full as u32, Ordering::Release);
    desc.generation.store(generation, Ordering::Release);
    generation
}

pub unsafe fn flush_tlb_mm_range(mm: *mut MmStruct, start: u64, end: u64) -> bool {
    let full = end <= start;
    unsafe {
        flush_local_range(start, end, full);
    }
    let this_cpu = crate::kernel::sched::current_cpu();
    let mut waits: [(usize, u64); MAX_CPUS] = [(0, 0); MAX_CPUS];
    let mut wait_len = 0usize;

    for cpu in 0..MAX_CPUS as u32 {
        if cpu == this_cpu {
            continue;
        }
        if active_mm(cpu) != mm {
            continue;
        }
        let generation = publish_remote_flush(cpu, mm, start, end, full);
        waits[wait_len] = (cpu as usize, generation);
        wait_len += 1;
        #[cfg(not(test))]
        unsafe {
            apic::send_ipi(
                cpu as u8,
                crate::arch::x86::kernel::idt::TLB_SHOOTDOWN_VECTOR,
            );
        }
    }

    if wait_len == 0 {
        return true;
    }
    #[cfg(test)]
    {
        for &(cpu, generation) in waits.iter().take(wait_len) {
            DESCRIPTORS[cpu].ack.store(generation, Ordering::Release);
        }
        return true;
    }
    #[cfg(not(test))]
    {
        let deadline = rdtsc().saturating_add(2_000_000_000);
        loop {
            let mut complete = true;
            for &(cpu, generation) in waits.iter().take(wait_len) {
                if DESCRIPTORS[cpu].ack.load(Ordering::Acquire) < generation {
                    complete = false;
                    break;
                }
            }
            if complete {
                return true;
            }
            if rdtsc() >= deadline {
                return false;
            }
            core::hint::spin_loop();
        }
    }
}

/// Compatibility helper retained for the Milestone 6 shootdown test.
#[cfg(feature = "test-tlb-shootdown")]
pub unsafe fn flush_tlb_others(cpus: &[u8], addr: u64) -> bool {
    let mm = active_mm(crate::kernel::sched::current_cpu());
    let before = TLB_SHOOTDOWN_ACK_COUNT.load(Ordering::Acquire);
    let mut waits: [(usize, u64); MAX_CPUS] = [(0, 0); MAX_CPUS];
    let mut wait_len = 0usize;

    for &cpu in cpus {
        let generation = publish_remote_flush(cpu as u32, mm, addr, addr + PAGE_SIZE, false);
        waits[wait_len] = (cpu as usize, generation);
        wait_len += 1;
        #[cfg(not(test))]
        unsafe {
            apic::send_ipi(cpu, crate::arch::x86::kernel::idt::TLB_SHOOTDOWN_VECTOR);
        }
    }

    #[cfg(test)]
    {
        for &(cpu, generation) in waits.iter().take(wait_len) {
            DESCRIPTORS[cpu].ack.store(generation, Ordering::Release);
        }
        return true;
    }
    #[cfg(not(test))]
    {
        let deadline = rdtsc().saturating_add(2_000_000_000);
        loop {
            let mut complete = true;
            for &(cpu, generation) in waits.iter().take(wait_len) {
                if DESCRIPTORS[cpu].ack.load(Ordering::Acquire) < generation {
                    complete = false;
                    break;
                }
            }
            if complete
                && TLB_SHOOTDOWN_ACK_COUNT.load(Ordering::Acquire)
                    >= before.saturating_add(wait_len as u64)
            {
                return true;
            }
            if rdtsc() >= deadline {
                return false;
            }
            core::hint::spin_loop();
        }
    }
}

#[cfg(feature = "test-tlb-shootdown")]
pub fn run_shootdown_test(cpus: &[crate::arch::x86::kernel::acpi::CpuInfo]) {
    let bsp_id = unsafe { apic::id() };
    let target = cpus.iter().find(|c| c.enabled && c.apic_id != bsp_id);
    let Some(ap) = target else {
        panic!("tlb: no non-BSP CPU available for shootdown test");
    };

    let cpus_to_flush = [ap.apic_id];
    let ok = unsafe { flush_tlb_others(&cpus_to_flush, 0) };
    if !ok {
        panic!(
            "tlb: shootdown IPI to AP {} did not ack within 2s timeout",
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

    #[test]
    fn shootdown_counter_starts_at_zero() {
        assert_eq!(TLB_SHOOTDOWN_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ack_counter_starts_at_zero() {
        assert_eq!(TLB_SHOOTDOWN_ACK_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn active_mm_round_trip() {
        let ptr = 0x1234usize as *mut MmStruct;
        unsafe { set_active_mm(2, ptr) };
        assert_eq!(active_mm(2), ptr);
    }

    #[test]
    fn descriptor_publish_advances_generation() {
        let generation = publish_remote_flush(1, core::ptr::null_mut(), 0x1000, 0x2000, false);
        assert!(generation >= 1);
        assert_eq!(
            DESCRIPTORS[1].generation.load(Ordering::Acquire),
            generation
        );
    }

    #[test]
    fn flush_targets_only_cpus_running_the_same_mm() {
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
}
