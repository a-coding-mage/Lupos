//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/cpuid-deps.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/cpuid-deps.c
//! CPUID feature dependency table.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/cpuid-deps.c

// `cpuid-deps.c` stores a small table mapping each X86_FEATURE_* to the
// feature it transitively depends on (e.g. AVX → XSAVE, AVX2 → AVX).
// When userspace disables a feature, the linker pass also clears all
// dependents. We mirror the relation as a const lookup table.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Feature {
    Fpu,
    Xsave,
    Avx,
    Avx2,
    Avx512f,
    Aes,
    Sse,
    Sse2,
    Sse4_1,
    Sse4_2,
    Fma,
    BmI1,
}

pub const fn parent_of(feature: X86Feature) -> Option<X86Feature> {
    match feature {
        X86Feature::Avx => Some(X86Feature::Xsave),
        X86Feature::Avx2 => Some(X86Feature::Avx),
        X86Feature::Avx512f => Some(X86Feature::Avx),
        X86Feature::Aes => Some(X86Feature::Sse2),
        X86Feature::Sse => Some(X86Feature::Fpu),
        X86Feature::Sse2 => Some(X86Feature::Sse),
        X86Feature::Sse4_1 => Some(X86Feature::Sse2),
        X86Feature::Sse4_2 => Some(X86Feature::Sse4_1),
        X86Feature::Fma => Some(X86Feature::Avx),
        X86Feature::BmI1 => Some(X86Feature::Sse2),
        X86Feature::Fpu | X86Feature::Xsave => None,
    }
}

pub const fn depends_on(feature: X86Feature, ancestor: X86Feature) -> bool {
    let mut cursor = parent_of(feature);
    let mut steps = 0;
    while let Some(parent) = cursor {
        if parent as u32 == ancestor as u32 {
            return true;
        }
        cursor = parent_of(parent);
        steps += 1;
        if steps > 16 {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avx2_depends_transitively_on_xsave() {
        assert!(depends_on(X86Feature::Avx2, X86Feature::Avx));
        assert!(depends_on(X86Feature::Avx2, X86Feature::Xsave));
        assert!(!depends_on(X86Feature::Avx2, X86Feature::Sse));
    }

    #[test]
    fn fpu_has_no_parent() {
        assert!(parent_of(X86Feature::Fpu).is_none());
    }
}
