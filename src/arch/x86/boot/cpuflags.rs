//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/cpuflags.c
//! test-origin: linux:vendor/linux/arch/x86/boot/cpuflags.c
//! CPU feature flag collection for the real-mode setup stub.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/cpuflags.c
//! - vendor/linux/arch/x86/boot/cpuflags.h
//!
//! `get_cpuflags()` populates a `cpu_features` struct by issuing CPUID
//! leaves 0, 1, 7, 0x80000000, 0x80000001 and storing flag words in
//! `cpu.flags[...]`. Lupos reuses `crate::arch::x86::kernel::cpuid` for CPUID
//! access; only the orchestration and the flag-word indexing logic are
//! ported here.

use crate::arch::x86::kernel::cpuid::cpuid;

/// `NCAPINTS` — number of 32-bit CPU-feature words. Matches Linux's
/// `vendor/linux/arch/x86/include/asm/cpufeatures.h::NCAPINTS`.
pub const NCAPINTS: usize = 21;

/// Linux `EFLAGS.ID` bit — used to detect CPUID availability on 386/486.
pub const X86_EFLAGS_ID: u32 = 1 << 21;

/// Linux x86_64 always has CPUID. The 32-bit setup stub uses a
/// pushf/popf dance to flip EFLAGS.ID; lupos targets x86_64 so this
/// short-circuits to true. Mirrors the inline asm in cpuflags.c for
/// ABI/parity documentation.
#[inline]
pub fn has_eflag_id_supported() -> bool {
    true
}

/// `struct cpu_features` — three header fields + `NCAPINTS` flag words.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct CpuFeatures {
    pub level: u32,
    pub family: u32,
    pub model: u32,
    pub flags: [u32; NCAPINTS],
}

/// `cpu_vendor[3]` global — the 12-byte vendor string returned by
/// CPUID leaf 0 (EBX:EDX:ECX, in that order in memory).
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct CpuVendor(pub [u32; 3]);

/// `has_fpu()` — query the FPU by `fninit/fnstcw/fnstsw`. Lupos targets
/// x86_64 where the FPU is mandatory; mirror the predicate for parity.
#[inline]
pub fn has_fpu() -> bool {
    true
}

/// `get_cpuflags()` — populate `out_cpu` and `out_vendor` from CPUID.
/// Mirrors cpuflags.c lines 68-110. Idempotent guard via `loaded` so
/// repeated calls match Linux's `loaded_flags` static.
pub fn get_cpuflags(out_cpu: &mut CpuFeatures, out_vendor: &mut CpuVendor, loaded: &mut bool) {
    if *loaded {
        return;
    }
    *loaded = true;

    if has_fpu() {
        // X86_FEATURE_FPU lives in flags[0] bit 0 (Intel CPUID leaf 1 EDX[0]).
        out_cpu.flags[0] |= 1 << 0;
    }

    if !has_eflag_id_supported() {
        return;
    }

    let leaf0 = cpuid(0, 0);
    let max_intel = leaf0.eax;
    // Linux stores the 12-byte vendor as EBX, EDX, ECX (NOT EBX/ECX/EDX!).
    out_vendor.0 = [leaf0.ebx, leaf0.edx, leaf0.ecx];

    if max_intel >= 0x0000_0001 && max_intel <= 0x0000_ffff {
        let l1 = cpuid(1, 0);
        let tfms = l1.eax;
        out_cpu.flags[4] = l1.ecx; // word 4: ECX of leaf 1
        out_cpu.flags[0] = l1.edx; // word 0: EDX of leaf 1
        out_cpu.level = (tfms >> 8) & 0xf;
        out_cpu.family = out_cpu.level;
        out_cpu.model = (tfms >> 4) & 0xf;
        if out_cpu.level >= 6 {
            out_cpu.model += ((tfms >> 16) & 0xf) << 4;
        }
    }

    if max_intel >= 0x0000_0007 {
        let l7 = cpuid(7, 0);
        out_cpu.flags[16] = l7.ecx; // word 16: ECX of leaf 7
    }

    let leaf_max_ext = cpuid(0x8000_0000, 0).eax;
    if leaf_max_ext >= 0x8000_0001 && leaf_max_ext <= 0x8000_ffff {
        let lext1 = cpuid(0x8000_0001, 0);
        out_cpu.flags[6] = lext1.ecx; // word 6: ECX of ext leaf 1
        out_cpu.flags[1] = lext1.edx; // word 1: EDX of ext leaf 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ncapints_matches_linux_cpufeatures_h() {
        // Linux 6.18 NCAPINTS == 21 (verified against
        // arch/x86/include/asm/cpufeatures.h).
        assert_eq!(NCAPINTS, 21);
    }

    #[test]
    fn eflags_id_bit_matches_processor_flags_h() {
        assert_eq!(X86_EFLAGS_ID, 1 << 21);
    }

    #[test]
    fn get_cpuflags_is_idempotent_via_loaded_guard() {
        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor::default();
        let mut loaded = false;
        get_cpuflags(&mut cpu, &mut vendor, &mut loaded);
        assert!(loaded);
        // Second call must not panic and must not clobber the level.
        let saved_level = cpu.level;
        let saved_flags = cpu.flags;
        get_cpuflags(&mut cpu, &mut vendor, &mut loaded);
        assert_eq!(cpu.level, saved_level);
        assert_eq!(cpu.flags, saved_flags);
    }

    #[test]
    fn x86_feature_fpu_is_set_in_word_zero_when_fpu_present() {
        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor::default();
        let mut loaded = false;
        get_cpuflags(&mut cpu, &mut vendor, &mut loaded);
        // X86_FEATURE_FPU = bit 0 of word 0.
        assert_eq!(cpu.flags[0] & 1, 1);
    }
}
