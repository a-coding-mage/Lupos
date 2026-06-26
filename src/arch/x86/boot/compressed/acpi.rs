//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/acpi.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/acpi.c
//! Decompressor ACPI RSDP locator.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/acpi.c
//!
//! The compressed stub finds the ACPI Root System Description Pointer
//! before any kernel page tables exist. Three sources are tried in
//! order: `boot_params.acpi_rsdp_addr` (KEXEC/cmdline override), the
//! EFI configuration table (under EFI boot), and the legacy BIOS scan
//! (EBDA + E0000h-FFFFFh).
//!
//! Lupos boots as a Linux boot-protocol bzImage. GRUB-provided EFI/ACPI
//! discovery data is consumed through the Linux `boot_params` fields, while
//! this port preserves the algorithmic core (signature check, standard +
//! extended checksum, EBDA pointer decode) for decompressor parity.

/// RSDP signature `"RSD PTR "` — 8 ASCII bytes.
pub const ACPI_RSDP_SIG: &[u8; 8] = b"RSD PTR ";

/// `ACPI_RSDP_CHECKSUM_LENGTH` — first-20-bytes ACPI 1.0 RSDP checksum.
pub const ACPI_RSDP_CHECKSUM_LENGTH: usize = 20;
/// `ACPI_RSDP_XCHECKSUM_LENGTH` — full-36-bytes ACPI 2.0+ RSDP checksum.
pub const ACPI_RSDP_XCHECKSUM_LENGTH: usize = 36;
/// `ACPI_RSDP_SCAN_STEP` — RSDP is paragraph-aligned (16 bytes).
pub const ACPI_RSDP_SCAN_STEP: usize = 16;

/// EBDA pointer location — `0x40E` (real-mode address). Mirrors
/// `ACPI_EBDA_PTR_LOCATION` in `include/acpi/actbl.h`.
pub const ACPI_EBDA_PTR_LOCATION: u32 = 0x40E;
/// `ACPI_EBDA_WINDOW_SIZE` — 1 KiB (the architectural minimum).
pub const ACPI_EBDA_WINDOW_SIZE: usize = 1024;
/// `ACPI_HI_RSDP_WINDOW_BASE` — start of the upper-BIOS scan window.
pub const ACPI_HI_RSDP_WINDOW_BASE: u32 = 0xE_0000;
/// `ACPI_HI_RSDP_WINDOW_SIZE` — extends to 0xFFFFFh (128 KiB).
pub const ACPI_HI_RSDP_WINDOW_SIZE: usize = 0x2_0000;

/// `compute_checksum(buffer)` — byte-sum modulo 256. Valid when zero.
/// Mirrors acpi.c lines 79-88.
pub fn compute_checksum(buffer: &[u8]) -> u8 {
    let mut sum: u8 = 0;
    for &b in buffer {
        sum = sum.wrapping_add(b);
    }
    sum
}

/// `ACPI_VALIDATE_RSDP_SIG(sig)` — first 8 bytes equal `"RSD PTR "`.
#[inline]
pub fn validate_rsdp_sig(sig: &[u8; 8]) -> bool {
    sig == ACPI_RSDP_SIG
}

/// `scan_mem_for_rsdp(window)` — paragraph-aligned scan returning the
/// offset of the first valid RSDP or `None`. Mirrors acpi.c lines
/// 90-125.
pub fn scan_mem_for_rsdp(window: &[u8]) -> Option<usize> {
    let mut off = 0usize;
    while off + ACPI_RSDP_XCHECKSUM_LENGTH <= window.len() {
        let sig: [u8; 8] = window[off..off + 8].try_into().ok()?;
        if !validate_rsdp_sig(&sig) {
            off += ACPI_RSDP_SCAN_STEP;
            continue;
        }
        if compute_checksum(&window[off..off + ACPI_RSDP_CHECKSUM_LENGTH]) != 0 {
            off += ACPI_RSDP_SCAN_STEP;
            continue;
        }
        let revision = window[off + 15];
        if revision >= 2 && compute_checksum(&window[off..off + ACPI_RSDP_XCHECKSUM_LENGTH]) != 0 {
            off += ACPI_RSDP_SCAN_STEP;
            continue;
        }
        return Some(off);
    }
    None
}

/// Decode the EBDA paragraph pointer at physical 0x40E into a linear
/// address. Linux: `address = *(u16*)0x40E; address <<= 4;`. Returns
/// `None` if EBDA appears unconfigured (address <= 0x400).
#[inline]
pub fn decode_ebda_paragraph(raw_paragraph: u16) -> Option<u32> {
    let linear = (raw_paragraph as u32) << 4;
    if linear > 0x400 { Some(linear) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal v2 RSDP at offset 0 of a small buffer; pad with
    /// noise so `scan_mem_for_rsdp` actually walks paragraphs.
    fn synth_rsdp() -> [u8; 64] {
        let mut buf = [0u8; 64];
        // Signature.
        buf[0..8].copy_from_slice(ACPI_RSDP_SIG);
        // Revision = 2 (so extended checksum is required).
        buf[15] = 2;
        // Make standard checksum zero by setting buf[19] to the
        // negation of the running sum of buf[0..19].
        let s = compute_checksum(&buf[..19]);
        buf[19] = 0u8.wrapping_sub(s);
        // Make extended checksum zero.
        let xs = compute_checksum(&buf[..ACPI_RSDP_XCHECKSUM_LENGTH - 1]);
        buf[ACPI_RSDP_XCHECKSUM_LENGTH - 1] = 0u8.wrapping_sub(xs);
        buf
    }

    #[test]
    fn signature_constant_matches_acpi_spec() {
        assert_eq!(ACPI_RSDP_SIG, b"RSD PTR ");
        assert_eq!(ACPI_RSDP_SIG.len(), 8);
    }

    #[test]
    fn scan_step_is_paragraph_aligned() {
        assert_eq!(ACPI_RSDP_SCAN_STEP, 16);
    }

    #[test]
    fn compute_checksum_wraps_modulo_256() {
        assert_eq!(compute_checksum(&[0; 4]), 0);
        assert_eq!(compute_checksum(&[1, 1, 1, 1]), 4);
        assert_eq!(compute_checksum(&[0xff, 0xff, 0x02]), 0x00);
    }

    #[test]
    fn scan_mem_for_rsdp_finds_valid_rsdp() {
        let rsdp = synth_rsdp();
        // Place RSDP at offset 32 inside a 96-byte search window.
        let mut window = [0u8; 96];
        window[32..32 + 64].copy_from_slice(&rsdp);
        assert_eq!(scan_mem_for_rsdp(&window), Some(32));
    }

    #[test]
    fn scan_mem_for_rsdp_rejects_bad_extended_checksum() {
        let mut rsdp = synth_rsdp();
        // Break the extended checksum.
        rsdp[ACPI_RSDP_XCHECKSUM_LENGTH - 1] ^= 0xff;
        assert_eq!(scan_mem_for_rsdp(&rsdp), None);
    }

    #[test]
    fn ebda_decode_returns_none_when_pointer_unconfigured() {
        // Linux: if the linear addr is <= 0x400 the EBDA isn't here.
        assert_eq!(decode_ebda_paragraph(0x0000), None);
        assert_eq!(decode_ebda_paragraph(0x0040), None); // 0x0040<<4 = 0x400
        assert_eq!(decode_ebda_paragraph(0x9F00), Some(0x9_F000));
    }

    #[test]
    fn window_size_constants_match_acpi_search_table() {
        assert_eq!(ACPI_EBDA_WINDOW_SIZE, 1024);
        assert_eq!(ACPI_HI_RSDP_WINDOW_BASE, 0xE_0000);
        assert_eq!(ACPI_HI_RSDP_WINDOW_SIZE, 0x2_0000);
    }
}
