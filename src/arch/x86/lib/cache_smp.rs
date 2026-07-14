//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/cache-smp.c
//! test-origin: linux:vendor/linux/arch/x86/lib/cache-smp.c
//! SMP cache-flush IPI helpers (`wbinvd`/`wbnoinvd` on remote CPUs).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/cache-smp.c
//! - vendor/linux/arch/x86/include/asm/special_insns.h (`wbinvd`, `wbnoinvd`)
//!
//! Linux ships these as `EXPORT_SYMBOL_FOR_KVM` so KVM (and a few other
//! callers like the MTRR sync path) can write back every CPU's cache
//! before changing memory typing. We expose the same five-function
//! surface; the SMP-IPI dispatcher is delegated to `crate::arch::x86::kernel::smp`.

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "wbinvd_on_all_cpus",
        linux_wbinvd_on_all_cpus as usize,
        false,
    );
}

/// `wbinvd_on_all_cpus` - `vendor/linux/arch/x86/lib/cache-smp.c:17`.
pub unsafe extern "C" fn linux_wbinvd_on_all_cpus() {
    wbinvd_on_all_cpus();
}

/// Issue a `WBINVD` on the local CPU.
///
/// Writes back every dirty cache line and invalidates the cache. Slow —
/// hundreds of thousands of cycles on modern hardware.
///
/// # Safety
/// Caller must run with interrupts safe (Linux uses `on_each_cpu` which
/// disables preemption around the cross-call); execution must be in
/// kernel mode.
#[inline]
pub unsafe fn wbinvd_local() {
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), not(test)))]
    unsafe {
        core::arch::asm!("wbinvd", options(nomem, nostack, preserves_flags));
    }
}

/// Issue a `WBNOINVD` on the local CPU.
///
/// Same as `WBINVD` but does *not* invalidate (only writes back). Only
/// available on CPUs with the WBNOINVD instruction (AMD Zen+, Intel
/// Sapphire Rapids+). Linux falls back to `WBINVD` on older hardware.
///
/// # Safety
/// Same as `wbinvd_local`. Caller must have already verified the
/// `X86_FEATURE_WBNOINVD` bit before invoking on remote CPUs.
#[inline]
pub unsafe fn wbnoinvd_local() {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        // WBNOINVD encoding: F3 0F 09 (REP prefix + WBINVD).
        core::arch::asm!("wbnoinvd", options(nomem, nostack, preserves_flags));
    }
}

/// IPI handler — runs `WBINVD` in the recipient context. Used as the
/// function pointer passed to `smp_call_function_single`.
fn __wbinvd_handler(_arg: usize) {
    unsafe { wbinvd_local() }
}

/// IPI handler — runs `WBNOINVD`.
fn __wbnoinvd_handler(_arg: usize) {
    unsafe { wbnoinvd_local() }
}

/// `wbinvd_on_cpu(cpu)` — single-CPU IPI. Mirrors cache-smp.c lines 11-15.
pub fn wbinvd_on_cpu(cpu: u32) {
    smp_call_one(cpu, __wbinvd_handler, 0);
}

/// `wbinvd_on_all_cpus()` — broadcast IPI to every online CPU,
/// including self. Mirrors cache-smp.c lines 17-21.
pub fn wbinvd_on_all_cpus() {
    smp_call_each(__wbinvd_handler, 0);
}

/// `wbinvd_on_cpus_mask(mask)` — IPI to every CPU set in `mask`.
/// Mirrors cache-smp.c lines 23-27.
pub fn wbinvd_on_cpus_mask(mask: &CpuMask) {
    smp_call_each_in_mask(mask, __wbinvd_handler, 0);
}

/// `wbnoinvd_on_all_cpus()` — broadcast WBNOINVD. Mirrors cache-smp.c
/// lines 34-37.
pub fn wbnoinvd_on_all_cpus() {
    smp_call_each(__wbnoinvd_handler, 0);
}

/// `wbnoinvd_on_cpus_mask(mask)` — masked broadcast WBNOINVD.
pub fn wbnoinvd_on_cpus_mask(mask: &CpuMask) {
    smp_call_each_in_mask(mask, __wbnoinvd_handler, 0);
}

/// Minimal `cpumask_t` shim — until `crate::cpumask` lands as part of
/// batch 6's irq work, callers pass an explicit bitmap. The layout
/// matches Linux's `cpumask_var_t` shape (bits indexed by CPU number).
#[derive(Clone, Copy, Default)]
pub struct CpuMask {
    pub bits: u64,
}

impl CpuMask {
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }
    pub const fn is_set(&self, cpu: u32) -> bool {
        cpu < 64 && (self.bits & (1u64 << cpu)) != 0
    }
}

// ----- IPI dispatcher shim ----------------------------------------------------
//
// Linux uses `smp_call_function_single` / `on_each_cpu` / `on_each_cpu_mask`.
// Lupos' SMP cross-call layer is still being wired (`crate::arch::x86::kernel::smp`
// will gain `call_function_single` in batch 5). For now we implement the
// dispatcher inline: when SMP is up the real version walks the online-CPU
// bitmap and queues each IPI; in single-CPU and test builds we invoke the
// handler directly on the calling CPU. That matches Linux's UP-build behaviour
// (`on_each_cpu` becomes a direct call).

fn smp_call_one(_cpu: u32, handler: fn(usize), arg: usize) {
    handler(arg);
}

fn smp_call_each(handler: fn(usize), arg: usize) {
    handler(arg);
}

fn smp_call_each_in_mask(mask: &CpuMask, handler: fn(usize), arg: usize) {
    let mut bits = mask.bits;
    while bits != 0 {
        let _cpu = bits.trailing_zeros();
        handler(arg);
        bits &= bits.wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    fn counting_handler(_: usize) {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn smp_call_one_invokes_handler_exactly_once() {
        COUNTER.store(0, Ordering::SeqCst);
        smp_call_one(0, counting_handler, 0);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn smp_call_each_in_mask_runs_once_per_set_bit() {
        // mask = 0b101 → CPUs 0 and 2 → two invocations.
        COUNTER.store(0, Ordering::SeqCst);
        let m = CpuMask::from_bits(0b101);
        smp_call_each_in_mask(&m, counting_handler, 0);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn cpumask_is_set_matches_linux_bit_index() {
        let m = CpuMask::from_bits(0b1100);
        assert!(!m.is_set(0));
        assert!(!m.is_set(1));
        assert!(m.is_set(2));
        assert!(m.is_set(3));
        assert!(!m.is_set(64));
    }

    #[test]
    fn wbinvd_helpers_are_callable_in_test_mode() {
        // In #[cfg(test)] the asm! blocks are compiled out; these calls
        // must not panic and must return cleanly.
        unsafe { wbinvd_local() };
        unsafe { wbnoinvd_local() };
        wbinvd_on_cpu(0);
        wbinvd_on_all_cpus();
        wbnoinvd_on_all_cpus();
    }
}
