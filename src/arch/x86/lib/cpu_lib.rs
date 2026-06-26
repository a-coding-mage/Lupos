//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/cpu.c
//! test-origin: linux:vendor/linux/arch/x86/lib/cpu.c
//! CPU signature decode helpers shared with KVM and microcode.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/cpu.c
//! - vendor/linux/arch/x86/include/asm/cpu.h (`x86_family`, `x86_model`,
//!   `x86_stepping`)
//!
//! Linux exposes `x86_family()`, `x86_model()`, `x86_stepping()` as
//! `EXPORT_SYMBOL_GPL` library helpers so KVM, microcode loaders, and
//! the early-detect code can extract the canonical (family, model,
//! stepping) tuple from CPUID.01H:EAX without each subsystem repeating
//! the bit twiddling.

/// Decode the canonical x86 family from CPUID.01H:EAX.
///
/// Mirrors `x86_family()` from `vendor/linux/arch/x86/lib/cpu.c`:
///
/// ```text
///   x86 = (sig >> 8) & 0xf;
///   if (x86 == 0xf) x86 += (sig >> 20) & 0xff;
/// ```
///
/// When the base family is `0xf` the *extended family* bits (CPUID
/// 27:20) are added on top — this is how AMD/Intel encode families
/// beyond 0x0F (e.g. AMD Zen → family 0x17 = 0xf + 0x08).
pub const fn x86_family(sig: u32) -> u32 {
    let mut x86 = (sig >> 8) & 0xf;
    if x86 == 0xf {
        x86 += (sig >> 20) & 0xff;
    }
    x86
}

/// Decode the canonical x86 model from CPUID.01H:EAX.
///
/// Mirrors `x86_model()` from cpu.c:
///
/// ```text
///   fam   = x86_family(sig);
///   model = (sig >> 4) & 0xf;
///   if (fam >= 0x6) model += ((sig >> 16) & 0xf) << 4;
/// ```
///
/// The *extended model* bits (CPUID 19:16) are folded in only for
/// family ≥ 6, matching Intel/AMD docs and Linux behaviour.
pub const fn x86_model(sig: u32) -> u32 {
    let fam = x86_family(sig);
    let mut model = (sig >> 4) & 0xf;
    if fam >= 0x6 {
        model += ((sig >> 16) & 0xf) << 4;
    }
    model
}

/// Decode the stepping from CPUID.01H:EAX — bottom four bits.
///
/// Mirrors `x86_stepping()` from cpu.c.
pub const fn x86_stepping(sig: u32) -> u32 {
    sig & 0xf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-world signatures harvested from Linux's CPU table.
    /// Each row: (CPUID EAX, expected family, expected model, expected stepping).
    const FIXTURES: &[(u32, u32, u32, u32)] = &[
        // Intel Pentium (P5): family 5, model 1, stepping 7.
        (0x0000_0517, 0x5, 0x1, 0x7),
        // Intel Pentium III (Coppermine): family 6, model 8, stepping 6.
        (0x0000_0686, 0x6, 0x8, 0x6),
        // Intel Core 2 Duo "Conroe": family 6, ext model 0xf+6=22 (Wikipedia),
        // base sig 0x000006f6 → fam 6, base model 0xf, ext model 0, model=0xf.
        (0x0000_06f6, 0x6, 0xf, 0x6),
        // Intel Skylake-S: family 6, model 0x5e, stepping 0x3.
        (0x0005_06e3, 0x6, 0x5e, 0x3),
        // Intel Pentium 4 Northwood: family 0xf, model 2, stepping 7.
        (0x0000_0f27, 0xf, 0x2, 0x7),
        // AMD K8 (Athlon 64): family 0xf, ext family 0, model 4, step 8.
        // sig 0x00000f48 → x86_family = 0xf + 0 = 0xf, model = 4 (fam>=6 so
        // ext model bits 19:16 = 0 fold in, still 4), stepping 8.
        (0x0000_0f48, 0xf, 0x4, 0x8),
        // AMD Zen (Ryzen): family 0x17, model 0x1, stepping 0x1.
        // ext family bits 27:20 = 0x08, base family = 0xf → 0xf + 0x08 = 0x17.
        (0x0080_0f11, 0x17, 0x1, 0x1),
        // AMD Zen 3 (5950X): family 0x19, model 0x21, stepping 0x0.
        // ext family bits = 0x0a, base = 0xf → 0x19; ext model bits 19:16 = 2,
        // base model 0x1 → 0x21.
        (0x00a2_0f10, 0x19, 0x21, 0x0),
    ];

    #[test]
    fn x86_family_matches_linux_decode() {
        for &(sig, fam, _, _) in FIXTURES {
            assert_eq!(
                x86_family(sig),
                fam,
                "x86_family({:#010x}) decoded incorrectly",
                sig
            );
        }
    }

    #[test]
    fn x86_model_matches_linux_decode() {
        for &(sig, _, model, _) in FIXTURES {
            assert_eq!(
                x86_model(sig),
                model,
                "x86_model({:#010x}) decoded incorrectly",
                sig
            );
        }
    }

    #[test]
    fn x86_stepping_matches_low_nibble() {
        for &(sig, _, _, step) in FIXTURES {
            assert_eq!(x86_stepping(sig), step);
        }
    }

    #[test]
    fn extended_family_only_folds_when_base_is_0xf() {
        // family base 0x6, ext family 0xff → still family 6 (no fold).
        assert_eq!(x86_family(0x0ff0_0600), 0x6);
        // family base 0xf, ext family 0x01 → 0xf + 1 = 0x10.
        assert_eq!(x86_family(0x0010_0f00), 0x10);
    }

    #[test]
    fn extended_model_only_folds_for_family_ge_6() {
        // family 5, ext model bits 19:16 = 0xf → ignored, model stays 0x1.
        assert_eq!(x86_model(0x000f_0510), 0x1);
        // family 6, ext model 0xf → model = (0x1 << 0) | (0xf << 4) = 0xf1.
        assert_eq!(x86_model(0x000f_0610), 0xf1);
    }
}
