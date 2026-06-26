//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/cpu.c
//! test-origin: linux:vendor/linux/arch/x86/boot/cpu.c
//! Real-mode CPU level / capability validation.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/cpu.c
//!
//! `validate_cpu()` is the final go/no-go check the setup stub runs
//! before handing control to the protected-mode kernel. It compares
//! detected CPU level (`x86-64`, `i686`, `i486`, etc.) and the
//! required level configured by `CONFIG_X86_MINIMUM_CPU_FAMILY`, and
//! enumerates any missing capability bits. Returns 0 on success, -1
//! on insufficient CPU.

/// Required level when targeting x86_64 — Linux sets this to 64.
pub const X86_64_LEVEL: i32 = 64;

/// Render the CPU level as a vendor-neutral string. Mirrors
/// `cpu_name()` in cpu.c lines 19-31.
pub fn cpu_name(level: i32) -> alloc::string::String {
    use alloc::string::ToString;
    if level == 64 {
        return "x86-64".to_string();
    }
    let lvl = if level == 15 { 6 } else { level };
    alloc::format!("i{lvl}86")
}

/// Mirror of `validate_cpu()` decision shape. Returns Ok if the CPU
/// meets `req_level` and has every required feature bit; Err otherwise.
/// The detected/required levels are passed in from a `cpuflags`
/// invocation rather than reading globals like Linux does, so the
/// function is testable from host code without touching CPUID.
pub fn validate_cpu(cpu_level: i32, req_level: i32, missing_flag_bits: &[u32]) -> Result<(), ()> {
    if cpu_level < req_level {
        return Err(());
    }
    if missing_flag_bits.iter().any(|&w| w != 0) {
        return Err(());
    }
    Ok(())
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn cpu_name_returns_x86_64_for_level_64() {
        assert_eq!(cpu_name(64), "x86-64".to_string());
    }

    #[test]
    fn cpu_name_collapses_15_to_686() {
        // Linux maps i586+ extensions of the family-15 (P4) class
        // back to "i686" — matches cpu.c line 27.
        assert_eq!(cpu_name(15), "i686".to_string());
    }

    #[test]
    fn cpu_name_renders_iN86_for_other_levels() {
        assert_eq!(cpu_name(3), "i386".to_string());
        assert_eq!(cpu_name(4), "i486".to_string());
        assert_eq!(cpu_name(6), "i686".to_string());
    }

    #[test]
    fn validate_cpu_passes_when_level_meets_and_no_missing_flags() {
        assert_eq!(validate_cpu(64, X86_64_LEVEL, &[0, 0, 0]), Ok(()));
    }

    #[test]
    fn validate_cpu_fails_when_level_below_required() {
        assert_eq!(validate_cpu(6, X86_64_LEVEL, &[0]), Err(()));
    }

    #[test]
    fn validate_cpu_fails_when_any_required_flag_missing() {
        assert_eq!(validate_cpu(64, X86_64_LEVEL, &[0, 1 << 5, 0]), Err(()));
    }
}
