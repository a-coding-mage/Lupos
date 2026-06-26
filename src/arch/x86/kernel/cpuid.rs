//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpuid.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpuid.c
//! CPUID instruction wrapper for x86_64 feature detection.
//!
//! CPUID allows software to query CPU capabilities before using optional
//! instruction sets or hardware features (APIC, SSE, SYSCALL, NX, etc.).
//! It is always safe to execute in 64-bit mode — the CPU will never fault.
//!
//! References:
//!   Intel SDM Vol. 2A Chapter 3 "CPUID — CPU Identification"
//!   https://wiki.osdev.org/CPUID
//!   https://www.sandpile.org/x86/cpuid.htm

use core::arch::asm;

/// Raw four-register result of a CPUID call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

/// Execute `CPUID` with the given leaf (`eax`) and subleaf (`ecx`).
///
/// Both leaf and subleaf are needed because some leaves (e.g., 0x7) use
/// `ecx` to select a sub-page of results.
///
/// Always valid on x86_64 — CPUID cannot fault in 64-bit mode.
///
/// # Why the push/pop rbx dance?
/// LLVM reserves `rbx` as a base pointer on some targets (notably when
/// compiling with `position-independent-code`).  Rust's inline asm
/// therefore disallows `out("ebx")` directly.  We work around this by
/// saving and restoring rbx around the `cpuid` instruction manually.
///
/// Reference: https://stackoverflow.com/a/35209119 — common workaround
#[inline]
pub fn cpuid(leaf: u32, subleaf: u32) -> CpuidResult {
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;
    // SAFETY: CPUID is a non-privileged instruction valid on all x86_64 CPUs.
    unsafe {
        asm!(
            // Save rbx, run cpuid, capture ebx output, restore rbx.
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf    => eax,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            ebx_out = out(reg) ebx,
            // nostack removed — we use push/pop above.
            options(nomem, preserves_flags),
        );
    }
    CpuidResult { eax, ebx, ecx, edx }
}

/// Feature bits from CPUID leaf 1 (standard feature flags).
///
/// Reference: Intel SDM Vol. 2A, Table 3-10 and 3-11 "Feature Information"
pub struct Features1 {
    pub ecx: u32,
    pub edx: u32,
}

impl Features1 {
    pub fn read() -> Self {
        let r = cpuid(1, 0);
        Self {
            ecx: r.ecx,
            edx: r.edx,
        }
    }

    /// Local APIC on-chip (EDX bit 9). Required for SMP and LAPIC timer.
    pub fn apic(&self) -> bool {
        self.edx & (1 << 9) != 0
    }

    /// Time Stamp Counter (EDX bit 4). Needed for `rdtsc`.
    pub fn tsc(&self) -> bool {
        self.edx & (1 << 4) != 0
    }

    /// XSAVE/XRSTOR (ECX bit 26). Needed for AVX state management.
    pub fn xsave(&self) -> bool {
        self.ecx & (1 << 26) != 0
    }
}

/// Extended feature bits from CPUID leaf 0x8000_0001 (AMD/Intel extended).
///
/// Reference: AMD64 APM Vol. 3 §3.1 "Function 8000_0001h — Extended Processor
///             and Processor Feature Identifiers"
pub struct ExtFeatures {
    pub edx: u32,
}

impl ExtFeatures {
    pub fn read() -> Self {
        let r = cpuid(0x8000_0001, 0);
        Self { edx: r.edx }
    }

    /// SYSCALL/SYSRET supported in 64-bit mode (EDX bit 11).
    ///
    /// All x86_64 CPUs support this — it is mandatory per the AMD64 spec.
    /// Reference: AMD64 APM Vol. 2 §2.5
    pub fn syscall(&self) -> bool {
        self.edx & (1 << 11) != 0
    }

    /// Long Mode (64-bit) available (EDX bit 29). Always set on x86_64.
    pub fn long_mode(&self) -> bool {
        self.edx & (1 << 29) != 0
    }

    /// NX (No-Execute / Execute-Disable) bit supported (EDX bit 20).
    ///
    /// Allows page table entries to forbid code execution, used for W^X policy.
    pub fn nx(&self) -> bool {
        self.edx & (1 << 20) != 0
    }
}

/// Return the 12-byte ASCII vendor string ("GenuineIntel", "AuthenticAMD", …).
///
/// CPUID leaf 0 returns the max supported basic leaf in EAX and the
/// vendor string packed across EBX, EDX, ECX (in that order).
pub fn vendor_string() -> [u8; 12] {
    let r = cpuid(0, 0);
    let mut s = [0u8; 12];
    s[0..4].copy_from_slice(&r.ebx.to_le_bytes());
    s[4..8].copy_from_slice(&r.edx.to_le_bytes());
    s[8..12].copy_from_slice(&r.ecx.to_le_bytes());
    s
}

/// Return the maximum basic CPUID leaf supported by this CPU.
///
/// If the returned value is < 1, `Features1::read()` would return
/// undefined results — but this never happens on x86_64.
pub fn max_basic_leaf() -> u32 {
    cpuid(0, 0).eax
}

/// Return the maximum extended CPUID leaf supported by this CPU (0x80000000+).
#[inline]
pub fn max_extended_leaf() -> u32 {
    cpuid(0x8000_0000, 0).eax
}

/// Return the 48-byte ASCII processor brand string from CPUID leaves
/// 0x80000002–0x80000004 (the order is EAX, EBX, ECX, EDX in each leaf).
///
/// Linux: `vendor/linux/arch/x86/kernel/cpu/common.c` (`get_model_name`).
/// Result is NUL-padded; the unused tail bytes are zero.
pub fn brand_string() -> [u8; 48] {
    let mut out = [0u8; 48];
    if max_extended_leaf() < 0x8000_0004 {
        return out;
    }
    for (i, leaf) in [0x8000_0002u32, 0x8000_0003, 0x8000_0004]
        .into_iter()
        .enumerate()
    {
        let r = cpuid(leaf, 0);
        let base = i * 16;
        out[base..base + 4].copy_from_slice(&r.eax.to_le_bytes());
        out[base + 4..base + 8].copy_from_slice(&r.ebx.to_le_bytes());
        out[base + 8..base + 12].copy_from_slice(&r.ecx.to_le_bytes());
        out[base + 12..base + 16].copy_from_slice(&r.edx.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpuid_leaf0_max_leaf_at_least_1() {
        // Every x86_64 CPU supports at least leaf 1.
        assert!(
            max_basic_leaf() >= 1,
            "max CPUID leaf must be >= 1 on x86_64"
        );
    }

    #[test]
    fn cpuid_result_is_deterministic() {
        // Two consecutive calls to the same leaf must return the same result.
        let a = cpuid(0, 0);
        let b = cpuid(0, 0);
        assert_eq!(a, b, "CPUID leaf 0 must return consistent results");
    }

    #[test]
    fn vendor_string_is_printable_ascii() {
        let s = vendor_string();
        for b in &s {
            assert!(
                b.is_ascii_alphanumeric(),
                "vendor byte {:#02x} is not alphanumeric ASCII",
                b
            );
        }
    }

    #[test]
    fn vendor_string_known_value() {
        // Under QEMU (the test environment) and most host CPUs this will be
        // one of the two well-known vendor strings.
        let s = vendor_string();
        let known = [
            *b"GenuineIntel",
            *b"AuthenticAMD",
            *b"GenuineIotel", // Some QEMU builds
        ];
        assert!(
            known.iter().any(|k| k == &s),
            "unexpected vendor string: {:?}",
            core::str::from_utf8(&s).unwrap_or("<invalid utf8>")
        );
    }

    #[test]
    fn features1_apic_present_on_x86_64() {
        // The Local APIC has been mandatory since the Pentium Pro.
        // Every x86_64 CPU must have it.
        assert!(
            Features1::read().apic(),
            "x86_64 CPU must have an on-chip APIC"
        );
    }

    #[test]
    fn ext_features_syscall_and_lm_always_set() {
        // SYSCALL and Long Mode are mandatory for x86_64.
        let f = ExtFeatures::read();
        assert!(f.syscall(), "SYSCALL must be available on x86_64");
        assert!(f.long_mode(), "Long Mode must be available on x86_64");
    }
}
