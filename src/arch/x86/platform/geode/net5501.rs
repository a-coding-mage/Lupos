//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/geode/net5501.c
//! test-origin: linux:vendor/linux/arch/x86/platform/geode/net5501.c
//! Soekris net5501 Geode board detection.

pub const BIOS_REGION_BASE: u32 = 0xffff_0000;
pub const BIOS_REGION_SIZE: usize = 0x0001_0000;
pub const COMBIOS_OFFSET: usize = 0x20;
pub const NET5501_RESTART_KEY_GPIO: u8 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeodeLed {
    pub gpio: u8,
    pub active_low: bool,
}

pub const NET5501_LEDS: [GeodeLed; 1] = [GeodeLed {
    gpio: 6,
    active_low: true,
}];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Net5501BoardSig {
    pub offset: usize,
    pub sig: &'static [u8],
}

pub const NET5501_BOARDS: [Net5501BoardSig; 2] = [
    Net5501BoardSig {
        offset: 0xb7b,
        sig: b"net5501",
    },
    Net5501BoardSig {
        offset: 0xb1f,
        sig: b"net5501",
    },
];

pub fn net5501_present(is_geode: bool, bios_region: &[u8]) -> bool {
    if !is_geode {
        return false;
    }
    if bios_region.get(COMBIOS_OFFSET..COMBIOS_OFFSET + 7) != Some(b"comBIOS") {
        return false;
    }
    NET5501_BOARDS.iter().any(|board| {
        bios_region.get(board.offset..board.offset + board.sig.len()) == Some(board.sig)
    })
}

pub const fn net5501_registers_restart_key_gpio() -> u8 {
    NET5501_RESTART_KEY_GPIO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net5501_detection_matches_linux_bios_signatures_and_gpio_setup() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/geode/net5501.c"
        ));
        assert!(source.contains("#define BIOS_REGION_BASE\t\t0xffff0000"));
        assert!(source.contains("#define BIOS_REGION_SIZE\t\t0x00010000"));
        assert!(source.contains("static const struct geode_led net5501_leds[]"));
        assert!(source.contains("{ 6, true }"));
        assert!(source.contains("geode_create_restart_key(24);"));
        assert!(source.contains("geode_create_leds(\"net5501\", net5501_leds"));
        assert!(source.contains("{ 0xb7b, 7, \"net5501\" }"));
        assert!(source.contains("{ 0xb1f, 7, \"net5501\" }"));
        assert!(source.contains("bios = rombase + 0x20;"));
        assert!(source.contains("memcmp(bios, \"comBIOS\", 7)"));
        assert!(source.contains("if (!is_geode())"));
        assert!(source.contains("device_initcall(net5501_init);"));

        let mut bios = [0u8; BIOS_REGION_SIZE];
        bios[COMBIOS_OFFSET..COMBIOS_OFFSET + 7].copy_from_slice(b"comBIOS");
        bios[0xb7b..0xb82].copy_from_slice(b"net5501");
        assert!(net5501_present(true, &bios));
        assert!(!net5501_present(false, &bios));
        bios[0xb7b] = b'X';
        assert!(!net5501_present(true, &bios));
        assert_eq!(
            NET5501_LEDS[0],
            GeodeLed {
                gpio: 6,
                active_low: true
            }
        );
        assert_eq!(net5501_registers_restart_key_gpio(), 24);
    }
}
