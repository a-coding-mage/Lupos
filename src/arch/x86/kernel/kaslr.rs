//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! KASLR entropy mixer.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/kaslr.c
//! - vendor/linux/arch/x86/include/asm/kaslr.h
//!
//! Provides `kaslr_get_random_long()` — the entropy primitive used by both
//! the compressed bootloader (for base randomization) and the regular
//! kernel's memory-randomization path. The function XOR-folds entropy from
//! RDRAND (if available), the TSC (if available), and the legacy i8254 PIT
//! (always-on fallback), then runs the accumulator through a circular
//! multiplication step for bit diffusion.
//!
//! The Linux source is shared between compressed and regular kernel builds
//! via `KASLR_COMPRESSED_BOOT`; this port targets the regular-kernel path.

use crate::arch::x86::include::asm::io::{inb, outb};

/// Linux `mix_const` on x86_64 — odd constant chosen for full-rank
/// multiplication. Matches `kaslr.c` line 52.
pub const MIX_CONST_64: u64 = 0x5d6008cbf3848dd3;

/// Linux `mix_const` on x86_32 — kept for parity even though lupos targets
/// x86_64. Matches `kaslr.c` line 54.
pub const MIX_CONST_32: u32 = 0x3f39e593;

// i8254 PIT control / counter ports — exactly as in kaslr.c.
const I8254_PORT_CONTROL: u16 = 0x43;
const I8254_PORT_COUNTER0: u16 = 0x40;
const I8254_CMD_READBACK: u8 = 0xC0;
const I8254_SELECT_COUNTER0: u8 = 0x02;
const I8254_STATUS_NOTREADY: u8 = 0x40;

/// Read the i8254 PIT counter via the readback command, retrying until
/// the status byte clears `NOTREADY`. Mirrors `static inline u16 i8254()`
/// in `vendor/linux/arch/x86/lib/kaslr.c` lines 34-47.
///
/// # Safety
/// Performs raw port I/O on 0x40/0x43. Caller must be at a boot stage
/// where the legacy PIT exists and is not yet repurposed.
#[inline]
unsafe fn i8254() -> u16 {
    loop {
        unsafe {
            outb(
                I8254_PORT_CONTROL,
                I8254_CMD_READBACK | I8254_SELECT_COUNTER0,
            );
            let status = inb(I8254_PORT_COUNTER0);
            let lo = inb(I8254_PORT_COUNTER0);
            let hi = inb(I8254_PORT_COUNTER0);
            if status & I8254_STATUS_NOTREADY == 0 {
                return ((hi as u16) << 8) | lo as u16;
            }
        }
    }
}

/// Test-only deterministic i8254 stub — keeps unit tests host-runnable
/// while exposing the mix path. Returns a 16-bit value with no I/O.
#[cfg(test)]
fn i8254_stub() -> u16 {
    0x9c8a // chosen for distinctness from RDRAND/TSC stub values
}

/// Trait letting tests inject deterministic entropy sources without
/// performing real RDRAND/RDTSC/PIO. The default impl uses the real
/// hardware paths exactly as `kaslr.c` does.
pub trait KaslrEntropy {
    /// Returns Some(raw) if RDRAND succeeded. Linux's `rdrand_long()`
    /// retries up to 10 times before giving up; we mirror that behaviour.
    fn rdrand_long(&self) -> Option<u64>;
    /// Returns true if X86_FEATURE_RDRAND is enabled.
    fn has_rdrand(&self) -> bool;
    /// Returns true if X86_FEATURE_TSC is enabled.
    fn has_tsc(&self) -> bool;
    /// Returns the current TSC value. Caller has already gated on `has_tsc`.
    fn rdtsc(&self) -> u64;
    /// Returns the boot seed — Linux's `get_boot_seed()` returns the
    /// kaslr_offset for the regular kernel and 0 for compressed.
    fn boot_seed(&self) -> u64;
    /// Returns the legacy i8254 PIT counter.
    fn i8254(&self) -> u16;
}

/// Production entropy source — real CPUID / RDRAND / RDTSC / PIO.
pub struct HardwareEntropy {
    pub kaslr_offset: u64,
}

impl KaslrEntropy for HardwareEntropy {
    fn rdrand_long(&self) -> Option<u64> {
        // Linux: up to 10 retries on RDRAND. CF=0 means failure.
        #[cfg(not(test))]
        for _ in 0..10 {
            let mut val: u64;
            let mut ok: u8;
            unsafe {
                core::arch::asm!(
                    "rdrand {val}",
                    "setc {ok}",
                    val = out(reg) val,
                    ok = out(reg_byte) ok,
                    options(nomem, nostack),
                );
            }
            if ok != 0 {
                return Some(val);
            }
        }
        #[cfg(test)]
        let _ = ();
        None
    }
    fn has_rdrand(&self) -> bool {
        // RDRAND advertised in CPUID.01H:ECX[30].
        // Ref: Intel SDM Vol. 2A — CPUID instruction.
        let r = crate::arch::x86::kernel::cpuid::cpuid(1, 0);
        (r.ecx & (1 << 30)) != 0
    }
    fn has_tsc(&self) -> bool {
        // TSC advertised in CPUID.01H:EDX[4]. Matches the bit Linux
        // checks via `cpu_feature_enabled(X86_FEATURE_TSC)`.
        let r = crate::arch::x86::kernel::cpuid::cpuid(1, 0);
        (r.edx & (1 << 4)) != 0
    }
    fn rdtsc(&self) -> u64 {
        super::tsc::read()
    }
    fn boot_seed(&self) -> u64 {
        self.kaslr_offset
    }
    fn i8254(&self) -> u16 {
        unsafe { i8254() }
    }
}

/// Mix `random` with `MIX_CONST_64` via the wide multiply (Linux uses
/// `mulq`). The Linux assembly returns the low half in `random` and the
/// high half in `raw`, then sums them: `random + raw`.
#[inline]
fn mul_mix(random: u64) -> u64 {
    let prod = (random as u128).wrapping_mul(MIX_CONST_64 as u128);
    let low = prod as u64;
    let high = (prod >> 64) as u64;
    low.wrapping_add(high)
}

/// Entropy-folding heart of KASLR. Mirrors `kaslr_get_random_long()`
/// from `vendor/linux/arch/x86/lib/kaslr.c` lines 49-98.
///
/// Behaviour:
/// 1. Seed `random` with `get_boot_seed()`.
/// 2. If RDRAND is available and produces a value, XOR it in; skip i8254.
/// 3. If TSC is available, XOR it in; skip i8254.
/// 4. If both prior sources were unavailable, fold in the i8254 PIT.
/// 5. Run the accumulator through a circular wide-mul mix.
pub fn kaslr_get_random_long<E: KaslrEntropy>(source: &E) -> u64 {
    let mut random = source.boot_seed();
    let mut use_i8254 = true;

    if source.has_rdrand() {
        if let Some(raw) = source.rdrand_long() {
            random ^= raw;
            use_i8254 = false;
        }
    }
    if source.has_tsc() {
        random ^= source.rdtsc();
        use_i8254 = false;
    }
    if use_i8254 {
        random ^= source.i8254() as u64;
    }
    mul_mix(random)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubAll {
        boot: u64,
        rdrand: Option<u64>,
        tsc: Option<u64>,
        i8254: u16,
    }
    impl KaslrEntropy for StubAll {
        fn rdrand_long(&self) -> Option<u64> {
            self.rdrand
        }
        fn has_rdrand(&self) -> bool {
            self.rdrand.is_some()
        }
        fn has_tsc(&self) -> bool {
            self.tsc.is_some()
        }
        fn rdtsc(&self) -> u64 {
            self.tsc.unwrap_or(0)
        }
        fn boot_seed(&self) -> u64 {
            self.boot
        }
        fn i8254(&self) -> u16 {
            self.i8254
        }
    }

    #[test]
    fn mix_const_constants_match_linux_kaslr_c() {
        assert_eq!(MIX_CONST_64, 0x5d6008cbf3848dd3);
        assert_eq!(MIX_CONST_32, 0x3f39e593);
    }

    #[test]
    fn rdrand_path_xors_in_raw_and_skips_i8254() {
        // Seed=0, RDRAND=0x1234, TSC absent, i8254 would be 0xdead but is
        // skipped because RDRAND succeeded. Result must depend on 0x1234,
        // not on 0xdead.
        let a = kaslr_get_random_long(&StubAll {
            boot: 0,
            rdrand: Some(0x1234),
            tsc: None,
            i8254: 0xdead,
        });
        let b = kaslr_get_random_long(&StubAll {
            boot: 0,
            rdrand: Some(0x1234),
            tsc: None,
            i8254: 0xbeef,
        });
        assert_eq!(a, b);
    }

    #[test]
    fn i8254_fallback_runs_when_rdrand_and_tsc_absent() {
        let a = kaslr_get_random_long(&StubAll {
            boot: 0,
            rdrand: None,
            tsc: None,
            i8254: 0xdead,
        });
        let b = kaslr_get_random_long(&StubAll {
            boot: 0,
            rdrand: None,
            tsc: None,
            i8254: 0xbeef,
        });
        // Different PIT readings produce different outputs once mixed.
        assert_ne!(a, b);
        assert_ne!(a, 0);
    }

    #[test]
    fn mix_is_deterministic_for_fixed_inputs() {
        // Same inputs → same outputs. Confirms the wide-mul mix is pure.
        let s = StubAll {
            boot: 0x1111,
            rdrand: Some(0x2222),
            tsc: Some(0x3333),
            i8254: 0x4444,
        };
        let a = kaslr_get_random_long(&s);
        let b = kaslr_get_random_long(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn mul_mix_is_full_64_bit_circular_multiply() {
        // For random=1, mul_mix(1) = (1 * MIX_CONST_64) low + high. With
        // such a small input the high half is 0, so the result is exactly
        // MIX_CONST_64.
        assert_eq!(mul_mix(1), MIX_CONST_64);
        // For random=0 → both halves zero → mix returns 0.
        assert_eq!(mul_mix(0), 0);
    }

    #[test]
    fn i8254_stub_path_is_pure() {
        // Sanity that the test-mode i8254 stub does not perform port I/O
        // (calling it from host tests must not panic).
        assert_eq!(i8254_stub(), 0x9c8a);
    }
}
