//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/geode/geos.c
//! test-origin: linux:vendor/linux/arch/x86/platform/geode/geos.c
//! Traverse Technologies GEOS Geode board detection.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeodeLed {
    pub gpio: u8,
    pub active_low: bool,
}

pub const GEOS_LEDS: [GeodeLed; 3] = [
    GeodeLed {
        gpio: 6,
        active_low: true,
    },
    GeodeLed {
        gpio: 25,
        active_low: false,
    },
    GeodeLed {
        gpio: 27,
        active_low: false,
    },
];

pub const fn geos_recognized(is_geode: bool, vendor: &str, product: &str) -> bool {
    is_geode && str_eq(vendor, "Traverse Technologies") && str_eq(product, "Geos")
}

pub const fn geos_restart_key() -> u8 {
    3
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0usize;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geos_dmi_detection_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/geode/geos.c"
        ));
        assert!(source.contains("static const struct geode_led geos_leds[]"));
        assert!(source.contains("{ 6, true }"));
        assert!(source.contains("{ 25, false }"));
        assert!(source.contains("geode_create_restart_key(3);"));
        assert!(source.contains("geode_create_leds(\"geos\", geos_leds"));
        assert!(source.contains("if (!is_geode())"));
        assert!(source.contains("DMI_SYS_VENDOR"));
        assert!(source.contains("\"Traverse Technologies\""));
        assert!(source.contains("DMI_PRODUCT_NAME"));
        assert!(source.contains("\"Geos\""));
        assert!(source.contains("device_initcall(geos_init);"));

        assert!(geos_recognized(true, "Traverse Technologies", "Geos"));
        assert!(!geos_recognized(false, "Traverse Technologies", "Geos"));
        assert!(!geos_recognized(true, "Traverse Technologies", "Other"));
        assert_eq!(GEOS_LEDS[0].gpio, 6);
        assert_eq!(geos_restart_key(), 3);
    }
}
