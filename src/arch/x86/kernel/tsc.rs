//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tsc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tsc.c
//! x86 Time Stamp Counter support.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/tsc.c`

use core::sync::atomic::{AtomicU64, Ordering};

use super::cpuid::CpuidResult;
use super::cpuid::cpuid;

pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const USEC_PER_SEC: u64 = 1_000_000;

#[inline]
pub fn read() -> u64 {
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), not(test)))]
    unsafe {
        let lo: u32;
        let hi: u32;
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
        ((hi as u64) << 32) | lo as u64
    }
    #[cfg(any(not(any(target_arch = "x86_64", target_arch = "x86")), test))]
    {
        0
    }
}

#[inline]
pub fn read_ordered() -> u64 {
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), not(test)))]
    unsafe {
        core::arch::asm!("lfence", options(nomem, nostack, preserves_flags));
        read()
    }
    #[cfg(any(not(any(target_arch = "x86_64", target_arch = "x86")), test))]
    {
        read()
    }
}

pub fn invariant_tsc_available() -> bool {
    let max = cpuid(0x8000_0000, 0).eax;
    max >= 0x8000_0007 && (cpuid(0x8000_0007, 0).edx & (1 << 8)) != 0
}

pub const fn cycles_to_ns(cycles: u64, khz: u64) -> u64 {
    if khz == 0 {
        0
    } else {
        cycles.saturating_mul(USEC_PER_SEC) / khz
    }
}

pub const fn ns_to_cycles(ns: u64, khz: u64) -> u64 {
    ns.saturating_mul(khz) / USEC_PER_SEC
}

pub const fn khz_from_cpuid_leaf15(leaf15: CpuidResult) -> Option<u64> {
    let denominator = leaf15.eax as u64;
    let numerator = leaf15.ebx as u64;
    let crystal_hz = leaf15.ecx as u64;
    if denominator == 0 || numerator == 0 || crystal_hz == 0 {
        return None;
    }
    Some(crystal_hz.saturating_mul(numerator) / denominator / 1000)
}

/// Derive TSC kHz from CPUID leaf 0x16 (Intel "Processor Frequency").
/// EAX bits 15:0 contain the processor base frequency in MHz.
/// Ref: Intel SDM Vol. 2A §3.2 — CPUID leaf 16H.
pub const fn khz_from_cpuid_leaf16(leaf16: CpuidResult) -> Option<u64> {
    let base_mhz = (leaf16.eax & 0xffff) as u64;
    if base_mhz == 0 {
        return None;
    }
    Some(base_mhz.saturating_mul(1000))
}

/// Global TSC frequency in kHz, populated by `calibrate()`. Zero means
/// "uncalibrated, do not trust" — callers should fall back to jiffies.
static TSC_KHZ: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn tsc_khz() -> u64 {
    TSC_KHZ.load(Ordering::Relaxed)
}

#[inline]
fn store_tsc_khz(khz: u64) {
    TSC_KHZ.store(khz, Ordering::Relaxed);
}

/// Calibrate the TSC and cache the result in [`tsc_khz`].
///
/// Linux: `vendor/linux/arch/x86/kernel/tsc.c::native_calibrate_tsc` /
/// `tsc_init`. Strategy mirror:
///   1. Try CPUID leaf 0x15 (TSC / Core Crystal Clock).
///   2. Try CPUID leaf 0x16 (Processor Frequency Information).
///   3. Fall back to PIT channel-2 gated calibration.
///
/// Returns the cached kHz value. Idempotent — re-calibration only runs if
/// the global is still zero.
pub fn calibrate() -> u64 {
    let existing = tsc_khz();
    if existing != 0 {
        return existing;
    }

    if let Some(khz) = khz_from_cpuid_leaf15(cpuid(0x15, 0)) {
        store_tsc_khz(khz);
        return khz;
    }

    let max_basic = cpuid(0, 0).eax;
    if max_basic >= 0x16 {
        if let Some(khz) = khz_from_cpuid_leaf16(cpuid(0x16, 0)) {
            store_tsc_khz(khz);
            return khz;
        }
    }

    // Under a hypervisor (KVM/Hyper-V/WHPX/VMware), the host publishes the TSC
    // frequency directly so the guest need not — and must not — rely on the
    // i8254 PIT. The PIT channel-2 OUT2 line (port 0x61 bit 5) is not reliably
    // emulated by hardware-assisted hypervisors, so `pit_calibrate_khz()` can
    // spin forever there. Prefer the paravirt timing leaf when present.
    if let Some(khz) = khz_from_hypervisor() {
        store_tsc_khz(khz);
        return khz;
    }

    let khz = pit_calibrate_khz();
    store_tsc_khz(khz);
    khz
}

/// TSC kHz from the hypervisor's paravirtual timing leaf, when running under a
/// VM. Returns `None` on bare metal or if the leaf is absent/zero.
///
/// Convention (KVM, Hyper-V, VMware): CPUID.1:ECX[31] flags "hypervisor
/// present"; the max paravirt leaf lives at `0x4000_0000`; leaf `0x4000_0010`
/// EAX carries the (virtual) TSC frequency in kHz. Mirrors Linux's
/// `vendor/linux/arch/x86/kernel/cpu/vmware.c` / `kvmclock.c` TSC-from-host
/// path, used before the native PIT calibration.
fn khz_from_hypervisor() -> Option<u64> {
    // Bit 31 of CPUID.1:ECX is the architectural "running on a hypervisor" flag.
    if cpuid(1, 0).ecx & (1 << 31) == 0 {
        return None;
    }
    let max_hv_leaf = cpuid(0x4000_0000, 0).eax;
    if max_hv_leaf < 0x4000_0010 {
        return None;
    }
    let tsc_khz = cpuid(0x4000_0010, 0).eax as u64;
    if tsc_khz == 0 { None } else { Some(tsc_khz) }
}

/// PIT channel-2 gated TSC calibration.
///
/// Uses ~50 ms of wall-clock time to measure TSC ticks. Mirrors the gate
/// dance from `vendor/linux/arch/x86/kernel/tsc.c::pit_calibrate_tsc`.
///
/// PIT input clock is 1.193182 MHz (NTSC colour subcarrier / 3). Channel 2
/// is the gateable channel — port 0x61 bit 0 enables it, bit 5 reflects
/// output state (HIGH when count reaches 0 in mode 0).
///
/// Returns 0 if RDTSC reads identically twice (no-x86 / under qemu icount
/// freeze) so callers can leave TSC_KHZ at 0 and stay on jiffies.
fn pit_calibrate_khz() -> u64 {
    #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), not(test)))]
    {
        use crate::arch::x86::include::asm::io::{inb, outb};
        const PIT_HZ: u64 = 1_193_182;
        const CALIBRATE_MS: u64 = 50;
        let pit_ticks = (PIT_HZ * CALIBRATE_MS / 1000) as u16;

        unsafe {
            // Disable the speaker (bit 1 of 0x61), enable channel-2 gate (bit 0).
            let val = inb(0x61);
            outb(0x61, (val & !0x02) | 0x01);

            // Mode word 0xB0 = channel 2, lo+hi byte, mode 0 (interrupt on
            // terminal count), binary count.
            outb(0x43, 0xB0);
            outb(0x42, (pit_ticks & 0xff) as u8);
            outb(0x42, (pit_ticks >> 8) as u8);

            let start = read_ordered();
            // Bit 5 of port 0x61 mirrors OUT2; in mode 0 it goes HIGH when the
            // down-counter reaches zero. The line is *not* reliably driven by
            // hardware-assisted hypervisors (KVM/WHPX), where each `inb` is a
            // VM exit — a raw spin bound of 10^9 there means ~minutes of wedged
            // boot. Bound by wall-clock TSC instead: the real measurement is
            // ~50 ms, so if many seconds of cycles elapse the OUT2 line is stuck
            // and we give up (return 0 -> caller falls back to jiffies). The
            // threshold is generous enough that a legitimately slow PIT — which
            // completes in <=~50 ms regardless of TSC frequency — never trips
            // it. A spin backstop covers hypervisors (e.g. WHPX) that freeze
            // RDTSC inside the busy loop so the wall-clock bound never trips:
            // a legitimate calibration sees OUT2 within ~50 ms, i.e. at most
            // ~10^6 `inb` reads even on the fastest port emulation, so a 20M
            // backstop bails in well under a second on a stuck PIT without ever
            // tripping on real hardware.
            const PIT_CALIBRATE_TIMEOUT_CYCLES: u64 = 5_000_000_000;
            const PIT_CALIBRATE_MAX_SPINS: u64 = 20_000_000;
            let mut spins: u64 = 0;
            while inb(0x61) & 0x20 == 0 {
                spins += 1;
                if spins > PIT_CALIBRATE_MAX_SPINS
                    || read_ordered().wrapping_sub(start) > PIT_CALIBRATE_TIMEOUT_CYCLES
                {
                    return 0;
                }
            }
            let end = read_ordered();
            let delta = end.saturating_sub(start);
            if delta == 0 {
                return 0;
            }
            // delta is TSC ticks in CALIBRATE_MS milliseconds, so kHz = delta / ms.
            delta / CALIBRATE_MS
        }
    }
    #[cfg(any(not(any(target_arch = "x86_64", target_arch = "x86")), test))]
    {
        0
    }
}

/// Convert a raw TSC reading to microseconds using the calibrated kHz.
/// Returns 0 when uncalibrated so callers can detect the unconverted path.
#[inline]
pub fn cycles_to_usec(cycles: u64) -> u64 {
    let khz = tsc_khz();
    if khz == 0 {
        0
    } else {
        cycles.saturating_mul(1000) / khz
    }
}

pub const fn tsc_delta_within_ppm(a: u64, b: u64, ppm: u64) -> bool {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let delta = hi - lo;
    delta.saturating_mul(1_000_000) <= hi.saturating_mul(ppm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_time_conversion_is_inverse_at_1ghz() {
        let khz = 1_000_000;
        assert_eq!(cycles_to_ns(1_000_000, khz), 1_000_000);
        assert_eq!(ns_to_cycles(1_000_000, khz), 1_000_000);
    }

    #[test]
    fn tsc_sync_tolerance_uses_ppm_window() {
        assert!(tsc_delta_within_ppm(1_000_000, 1_000_500, 1000));
        assert!(!tsc_delta_within_ppm(1_000_000, 1_002_000, 1000));
    }

    #[test]
    fn cpuid_leaf16_derives_tsc_khz_from_base_mhz() {
        let leaf16 = CpuidResult {
            eax: 2400, // 2.4 GHz base
            ebx: 4000,
            ecx: 100,
            edx: 0,
        };
        assert_eq!(khz_from_cpuid_leaf16(leaf16), Some(2_400_000));
        assert_eq!(
            khz_from_cpuid_leaf16(CpuidResult {
                eax: 0,
                ebx: 0,
                ecx: 0,
                edx: 0,
            }),
            None
        );
    }

    #[test]
    fn cycles_to_usec_returns_zero_when_uncalibrated() {
        // TSC_KHZ defaults to 0; the host-test harness never calls calibrate().
        assert_eq!(cycles_to_usec(1_000_000_000), 0);
    }

    #[test]
    fn cpuid_leaf15_derives_tsc_khz_when_crystal_is_known() {
        let leaf15 = CpuidResult {
            eax: 2,
            ebx: 192,
            ecx: 25_000_000,
            edx: 0,
        };
        assert_eq!(khz_from_cpuid_leaf15(leaf15), Some(2_400_000));
        assert_eq!(
            khz_from_cpuid_leaf15(CpuidResult {
                eax: 0,
                ebx: 192,
                ecx: 25_000_000,
                edx: 0,
            }),
            None
        );
    }
}
