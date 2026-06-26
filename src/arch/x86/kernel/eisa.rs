//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/eisa.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/eisa.c
//! EISA bus signature probing.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/eisa.c

pub const EISA_SIGNATURE: u32 = u32::from_le_bytes(*b"EISA");

pub const fn eisa_bus_probe(
    xen_pv_domain: bool,
    xen_initial_domain: bool,
    sev_snp_guest: bool,
    signature: Option<u32>,
) -> bool {
    if (xen_pv_domain && !xen_initial_domain) || sev_snp_guest {
        return false;
    }
    matches!(signature, Some(EISA_SIGNATURE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_constant_matches_little_endian_eisa_word() {
        assert_eq!(EISA_SIGNATURE, 0x4153_4945);
    }

    #[test]
    fn eisa_probe_is_disabled_for_restricted_guests() {
        assert!(eisa_bus_probe(false, false, false, Some(EISA_SIGNATURE)));
        assert!(!eisa_bus_probe(true, false, false, Some(EISA_SIGNATURE)));
        assert!(eisa_bus_probe(true, true, false, Some(EISA_SIGNATURE)));
        assert!(!eisa_bus_probe(false, false, true, Some(EISA_SIGNATURE)));
        assert!(!eisa_bus_probe(false, false, false, None));
    }
}
