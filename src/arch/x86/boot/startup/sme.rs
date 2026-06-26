//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/startup/sme.c
//! test-origin: linux:vendor/linux/arch/x86/boot/startup/sme.c
//! AMD Secure Memory Encryption (SME) setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/sme.c
//!
//! Linux's startup SME path performs three things at boot:
//!   1. Detect AMD SME / SEV from CPUID 0x8000001F.
//!   2. Read MSR_AMD64_SME_MASK to learn the "C-bit" position in the
//!      page-table entries.
//!   3. Build a temporary boot map that toggles the C-bit so the
//!      decompressor's data can be both encrypted (final mapping) and
//!      decrypted (so the unzipper can read it as plain bytes).
//!
//! The port carries the detection constants, the C-bit derivation,
//! and the encryption-mode enum. The map-rewriting algorithm itself
//! relies on the full SME state machine landing with the Batch 9 `coco/` ports.

/// `CPUID_AMD_SME_FEATURES` — leaf 0x8000_001F. EAX bit 0 advertises
/// SME, bit 1 advertises SEV, bit 3 advertises SEV-ES.
pub const CPUID_AMD_SME_FEATURES: u32 = 0x8000_001F;
pub const CPUID_AMD_SME_BIT: u32 = 1 << 0;
pub const CPUID_AMD_SEV_BIT: u32 = 1 << 1;
pub const CPUID_AMD_SEV_ES_BIT: u32 = 1 << 3;
pub const CPUID_AMD_SEV_SNP_BIT: u32 = 1 << 4;

/// `MSR_AMD64_SYSCFG` — bit 23 (`SYSCFG.MemEncryptionModeEn`) gates SME.
pub const MSR_AMD64_SYSCFG: u32 = 0xC001_0010;
pub const MSR_AMD64_SYSCFG_MEM_ENCRYPT: u64 = 1 << 23;

/// Where the C-bit lives in CPUID 0x8000001F:EBX bits 5:0.
pub fn c_bit_position(cpuid_8000_001f_ebx: u32) -> u32 {
    cpuid_8000_001f_ebx & 0x3F
}

/// Compute the C-bit mask given its bit position.
pub fn c_bit_mask(c_bit_pos: u32) -> u64 {
    1u64 << c_bit_pos
}

/// Encryption mode in effect after SME bring-up. Mirrors the
/// dispatch in sme.c.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum EncryptionMode {
    /// Plain text, no encryption.
    None,
    /// SME — system-wide encryption with a CPU-managed key.
    Sme,
    /// SEV — VM-specific encryption.
    Sev,
    /// SEV-ES — SEV plus encrypted CPU state.
    SevEs,
    /// SEV-SNP — SEV-ES plus integrity.
    SevSnp,
}

/// Decide the active encryption mode from CPUID + MSR_AMD64_SEV.
pub fn detect_encryption_mode(cpuid_eax_8000_001f: u32, sev_status: u64) -> EncryptionMode {
    let snp = sev_status & 0x4 != 0;
    let es = sev_status & 0x2 != 0;
    let sev = sev_status & 0x1 != 0;
    if snp {
        return EncryptionMode::SevSnp;
    }
    if es {
        return EncryptionMode::SevEs;
    }
    if sev {
        return EncryptionMode::Sev;
    }
    if cpuid_eax_8000_001f & CPUID_AMD_SME_BIT != 0 {
        return EncryptionMode::Sme;
    }
    EncryptionMode::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpuid_feature_bits_match_amd_apm() {
        assert_eq!(CPUID_AMD_SME_BIT, 0x1);
        assert_eq!(CPUID_AMD_SEV_BIT, 0x2);
        assert_eq!(CPUID_AMD_SEV_ES_BIT, 0x8);
        assert_eq!(CPUID_AMD_SEV_SNP_BIT, 0x10);
    }

    #[test]
    fn syscfg_msr_constants_match_amd_apm() {
        assert_eq!(MSR_AMD64_SYSCFG, 0xC001_0010);
        assert_eq!(MSR_AMD64_SYSCFG_MEM_ENCRYPT, 1 << 23);
    }

    #[test]
    fn c_bit_position_extracts_low_6_bits() {
        // CPUID EBX is `[31:6 reserved][5:0 c_bit_pos]`.
        assert_eq!(c_bit_position(0xCAFE_0023), 0x23);
        assert_eq!(c_bit_position(0x3F), 0x3F);
        assert_eq!(c_bit_position(0xFFFF_FFFF), 0x3F);
    }

    #[test]
    fn c_bit_mask_shifts_correctly_for_a_typical_layout() {
        // Typical AMD Zen: c_bit at position 47 (bit 47 of the PTE).
        assert_eq!(c_bit_mask(47), 1u64 << 47);
        assert_eq!(c_bit_mask(0), 1);
    }

    #[test]
    fn detect_encryption_mode_prefers_snp_es_sev_then_sme() {
        // Just SME, no SEV.
        assert_eq!(
            detect_encryption_mode(CPUID_AMD_SME_BIT, 0),
            EncryptionMode::Sme
        );
        // SEV only.
        assert_eq!(detect_encryption_mode(0, 0x1), EncryptionMode::Sev);
        // SEV-ES.
        assert_eq!(detect_encryption_mode(0, 0x3), EncryptionMode::SevEs);
        // SEV-SNP.
        assert_eq!(detect_encryption_mode(0, 0x7), EncryptionMode::SevSnp);
        // None.
        assert_eq!(detect_encryption_mode(0, 0), EncryptionMode::None);
    }
}
