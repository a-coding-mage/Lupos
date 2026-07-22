//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/tsc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/tsc.c
//! x86 Time Stamp Counter support.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/tsc.c`

use core::sync::atomic::{AtomicU32, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

use super::cpuid::cpuid;
use super::cpuid::{CpuidResult, max_basic_leaf, vendor_string};

pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const USEC_PER_SEC: u64 = 1_000_000;
const INTEL_VENDOR: [u8; 12] = *b"GenuineIntel";
const ACRN_HYPERVISOR_SIGNATURE: [u8; 12] = *b"ACRNACRNACRN";
const ACRN_CPUID_TIMING_INFO: u32 = 0x4000_0010;
const MIN_REASONABLE_TSC_KHZ: u64 = 100_000;
const MAX_REASONABLE_TSC_KHZ: u64 = 10_000_000;

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

pub const fn plausible_tsc_khz(khz: u64) -> Option<u64> {
    if khz >= MIN_REASONABLE_TSC_KHZ && khz <= MAX_REASONABLE_TSC_KHZ {
        Some(khz)
    } else {
        None
    }
}

fn khz_from_cpuid_tsc_ratio(
    vendor: [u8; 12],
    max_basic: u32,
    leaf15: CpuidResult,
    leaf16: CpuidResult,
) -> Option<u64> {
    if vendor != INTEL_VENDOR || max_basic < 0x15 {
        return None;
    }

    let denominator = leaf15.eax as u64;
    let numerator = leaf15.ebx as u64;
    if denominator == 0 || numerator == 0 {
        return None;
    }

    let mut crystal_khz = leaf15.ecx as u64 / 1000;
    if crystal_khz == 0 && max_basic >= 0x16 {
        let base_mhz = (leaf16.eax & 0xffff) as u64;
        crystal_khz = base_mhz.saturating_mul(1000).saturating_mul(denominator) / numerator;
    }
    if crystal_khz == 0 {
        return None;
    }

    Some(crystal_khz.saturating_mul(numerator) / denominator)
}

fn khz_from_cpuid_cpu_frequency(
    vendor: [u8; 12],
    max_basic: u32,
    leaf16: CpuidResult,
) -> Option<u64> {
    if vendor != INTEL_VENDOR || max_basic < 0x16 {
        return None;
    }
    khz_from_cpuid_leaf16(leaf16)
}

/// Global TSC frequency in kHz, populated by `calibrate()`. Zero means
/// "uncalibrated, do not trust" — callers should fall back to jiffies.
static TSC_KHZ: AtomicU32 = AtomicU32::new(0);
static CPU_KHZ: AtomicU32 = AtomicU32::new(0);

#[inline]
pub fn tsc_khz() -> u64 {
    TSC_KHZ.load(Ordering::Relaxed) as u64
}

#[inline]
fn store_tsc_khz(khz: u64) {
    let khz = khz.min(u32::MAX as u64) as u32;
    TSC_KHZ.store(khz, Ordering::Relaxed);
    CPU_KHZ.store(khz, Ordering::Relaxed);
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("tsc_khz", core::ptr::addr_of!(TSC_KHZ) as usize, false);
    export_symbol_once("cpu_khz", core::ptr::addr_of!(CPU_KHZ) as usize, false);
}

/// Calibrate the TSC and cache the result in [`tsc_khz`].
///
/// Linux: `vendor/linux/arch/x86/kernel/tsc.c::native_calibrate_tsc` /
/// `tsc_init`. Strategy mirror:
///   1. Prefer a Linux-wired hypervisor-published TSC frequency when present.
///   2. Try Intel CPUID leaf 0x15 (TSC / Core Crystal Clock), including
///      Linux's leaf 0x16-derived crystal fallback.
///   3. Try Intel CPUID leaf 0x16 as a last fast CPU-frequency fallback.
///   4. Fall back to PIT channel-2 gated calibration.
///
/// Returns the cached kHz value. Idempotent — re-calibration only runs if
/// the global is still zero.
pub fn calibrate() -> u64 {
    let existing = tsc_khz();
    if existing != 0 {
        return existing;
    }

    // Linux wires hypervisor TSC calibration through vendor-specific platform
    // hooks before native CPUID/PIT calibration. Do the same only for the
    // CPUID timing leaf Linux documents as such (ACRN); KVM, Hyper-V, and
    // VMware use pvclock/MSRs/hypercalls rather than a generic 0x40000010
    // contract, and QEMU TCG can expose a plausible-but-wrong value there.
    if let Some(khz) = khz_from_hypervisor().and_then(plausible_tsc_khz) {
        store_tsc_khz(khz);
        return khz;
    }

    let vendor = vendor_string();
    let max_basic = max_basic_leaf();
    let leaf16 = cpuid(0x16, 0);

    if let Some(khz) = khz_from_cpuid_tsc_ratio(vendor, max_basic, cpuid(0x15, 0), leaf16)
        .and_then(plausible_tsc_khz)
    {
        store_tsc_khz(khz);
        return khz;
    }

    if let Some(khz) =
        khz_from_cpuid_cpu_frequency(vendor, max_basic, leaf16).and_then(plausible_tsc_khz)
    {
        store_tsc_khz(khz);
        return khz;
    }

    let khz = plausible_tsc_khz(pit_calibrate_khz()).unwrap_or(0);
    store_tsc_khz(khz);
    khz
}

/// TSC kHz from a Linux-recognized CPUID paravirtual timing leaf.
///
/// Linux's ACRN guest path uses `ACRN_CPUID_TIMING_INFO` (`0x40000010`) as a
/// TSC-frequency leaf. Other hypervisors do not share that ABI in Linux:
/// KVM uses pvclock, Hyper-V uses frequency MSRs, and VMware uses a hypercall.
fn khz_from_hypervisor() -> Option<u64> {
    // Bit 31 of CPUID.1:ECX is the architectural "running on a hypervisor" flag.
    let hypervisor_present = cpuid(1, 0).ecx & (1 << 31) != 0;
    let hv_info = cpuid(0x4000_0000, 0);
    khz_from_hypervisor_timing_leaf(
        hypervisor_present,
        hv_info.eax,
        hypervisor_signature(hv_info),
        cpuid(ACRN_CPUID_TIMING_INFO, 0),
    )
}

fn hypervisor_signature(info_leaf: CpuidResult) -> [u8; 12] {
    let mut signature = [0u8; 12];
    // Hypervisor signatures are EBX, ECX, EDX. This differs from CPUID.0's
    // CPU vendor string order (EBX, EDX, ECX).
    signature[0..4].copy_from_slice(&info_leaf.ebx.to_le_bytes());
    signature[4..8].copy_from_slice(&info_leaf.ecx.to_le_bytes());
    signature[8..12].copy_from_slice(&info_leaf.edx.to_le_bytes());
    signature
}

fn khz_from_hypervisor_timing_leaf(
    hypervisor_present: bool,
    max_hv_leaf: u32,
    signature: [u8; 12],
    timing_leaf: CpuidResult,
) -> Option<u64> {
    if !hypervisor_present
        || signature != ACRN_HYPERVISOR_SIGNATURE
        || max_hv_leaf < ACRN_CPUID_TIMING_INFO
    {
        return None;
    }
    let tsc_khz = timing_leaf.eax as u64;
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
    fn plausible_tsc_frequency_rejects_tcg_placeholder_values() {
        assert_eq!(plausible_tsc_khz(1_000), None);
        assert_eq!(plausible_tsc_khz(0), None);
        assert_eq!(plausible_tsc_khz(2_400_000), Some(2_400_000));
        assert_eq!(plausible_tsc_khz(20_000_000), None);
    }

    #[test]
    fn cpuid_tsc_ratio_matches_linux_intel_only_path() {
        let leaf15 = CpuidResult {
            eax: 2,
            ebx: 192,
            ecx: 25_000_000,
            edx: 0,
        };
        let zero = CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };
        assert_eq!(
            khz_from_cpuid_tsc_ratio(INTEL_VENDOR, 0x16, leaf15, zero),
            Some(2_400_000)
        );
        assert_eq!(
            khz_from_cpuid_tsc_ratio(*b"AuthenticAMD", 0x16, leaf15, zero),
            None
        );
    }

    #[test]
    fn cpuid_tsc_ratio_uses_leaf16_crystal_fallback_like_linux() {
        let leaf15 = CpuidResult {
            eax: 2,
            ebx: 192,
            ecx: 0,
            edx: 0,
        };
        let leaf16 = CpuidResult {
            eax: 2400,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };

        assert_eq!(
            khz_from_cpuid_tsc_ratio(INTEL_VENDOR, 0x16, leaf15, leaf16),
            Some(2_400_000)
        );
    }

    #[test]
    fn cpuid_leaf16_cpu_frequency_matches_linux_intel_only_path() {
        let leaf16 = CpuidResult {
            eax: 2400,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };

        assert_eq!(
            khz_from_cpuid_cpu_frequency(INTEL_VENDOR, 0x16, leaf16),
            Some(2_400_000)
        );
        assert_eq!(
            khz_from_cpuid_cpu_frequency(*b"AuthenticAMD", 0x16, leaf16),
            None
        );
        assert_eq!(
            khz_from_cpuid_cpu_frequency(INTEL_VENDOR, 0x15, leaf16),
            None
        );
    }

    #[test]
    fn hypervisor_timing_leaf_matches_linux_acrn_only_contract() {
        let timing_leaf = CpuidResult {
            eax: 2_400_000,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };

        assert_eq!(
            khz_from_hypervisor_timing_leaf(
                true,
                ACRN_CPUID_TIMING_INFO,
                ACRN_HYPERVISOR_SIGNATURE,
                timing_leaf,
            ),
            Some(2_400_000)
        );
        assert_eq!(
            khz_from_hypervisor_timing_leaf(
                false,
                ACRN_CPUID_TIMING_INFO,
                ACRN_HYPERVISOR_SIGNATURE,
                timing_leaf,
            ),
            None
        );
        assert_eq!(
            khz_from_hypervisor_timing_leaf(
                true,
                ACRN_CPUID_TIMING_INFO - 1,
                ACRN_HYPERVISOR_SIGNATURE,
                timing_leaf,
            ),
            None
        );
        assert_eq!(
            khz_from_hypervisor_timing_leaf(
                true,
                ACRN_CPUID_TIMING_INFO,
                ACRN_HYPERVISOR_SIGNATURE,
                CpuidResult {
                    eax: 0,
                    ebx: 0,
                    ecx: 0,
                    edx: 0,
                },
            ),
            None
        );
    }

    #[test]
    fn hypervisor_timing_leaf_rejects_non_acrn_signatures() {
        let timing_leaf = CpuidResult {
            eax: 543_000,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };

        for signature in [
            *b"TCGTCGTCGTCG",
            *b"KVMKVMKVM\0\0\0",
            *b"Microsoft Hv",
            *b"VMwareVMware",
        ] {
            assert_eq!(
                khz_from_hypervisor_timing_leaf(
                    true,
                    ACRN_CPUID_TIMING_INFO,
                    signature,
                    timing_leaf,
                ),
                None,
                "Linux does not treat {:?} leaf 0x40000010 as ACRN timing info",
                core::str::from_utf8(&signature).unwrap_or("<non-utf8>")
            );
        }
    }

    #[test]
    fn cycles_to_usec_returns_zero_when_uncalibrated() {
        // TSC_KHZ defaults to 0; the host-test harness never calls calibrate().
        assert_eq!(cycles_to_usec(1_000_000_000), 0);
    }

    #[test]
    fn tsc_frequency_exports_are_linux_width_data_symbols() {
        register_module_exports();
        assert_eq!(core::mem::size_of_val(&TSC_KHZ), 4);
        assert_eq!(
            crate::kernel::module::find_symbol("tsc_khz"),
            Some(core::ptr::addr_of!(TSC_KHZ) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("cpu_khz"),
            Some(core::ptr::addr_of!(CPU_KHZ) as usize)
        );
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
