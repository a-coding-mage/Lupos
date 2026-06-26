//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/cpuflags.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/cpuflags.c
//! Compressed-kernel `has_cpuflag(flag)` shim.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/cpuflags.c
//!
//! Three lines in C: include `../cpuflags.c`, expose `has_cpuflag` as
//! `test_bit(flag, cpu.flags)`. The parent port already populates
//! `cpu.flags[]` via `boot_setup::cpuflags::get_cpuflags`.

use crate::arch::x86::boot::cpuflags::{CpuFeatures, CpuVendor, NCAPINTS, get_cpuflags};

/// `has_cpuflag(flag)` — bit `flag` set in the per-CPU `cpu.flags`?
/// Mirrors boot/compressed/cpuflags.c::has_cpuflag.
pub fn has_cpuflag(flag: u32, cpu: &CpuFeatures) -> bool {
    let word = (flag / 32) as usize;
    let bit = flag % 32;
    if word >= NCAPINTS {
        return false;
    }
    (cpu.flags[word] & (1u32 << bit)) != 0
}

/// Convenience: populate `cpu` and return the predicate result. Mirrors
/// the inline `get_cpuflags(); test_bit(...)` sequence in the .c file.
pub fn has_cpuflag_lazy(flag: u32) -> bool {
    let mut cpu = CpuFeatures::default();
    let mut vendor = CpuVendor::default();
    let mut loaded = false;
    get_cpuflags(&mut cpu, &mut vendor, &mut loaded);
    has_cpuflag(flag, &cpu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_cpuflag_reads_correct_word_and_bit() {
        let mut cpu = CpuFeatures::default();
        // Set bit 5 of word 2 → linear flag index = 32*2 + 5 = 69.
        cpu.flags[2] = 1u32 << 5;
        assert!(has_cpuflag(69, &cpu));
        assert!(!has_cpuflag(70, &cpu));
        assert!(!has_cpuflag(68, &cpu));
    }

    #[test]
    fn has_cpuflag_returns_false_for_out_of_range_word() {
        let cpu = CpuFeatures::default();
        let beyond_table = (NCAPINTS as u32) * 32 + 1;
        assert!(!has_cpuflag(beyond_table, &cpu));
    }

    #[test]
    fn has_cpuflag_lazy_does_not_panic() {
        // CPUID is invoked in production; in tests our `cpuid` wrapper
        // returns a deterministic zero leaf set. The call must complete.
        let _ = has_cpuflag_lazy(0);
    }
}
