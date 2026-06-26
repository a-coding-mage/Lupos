//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apic/probe_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apic/probe_32.c
//! 32-bit generic APIC driver probe and command-line selection.

use crate::include::uapi::errno::EINVAL;

pub const APIC_DEFAULT_NAME: &str = "default";
pub const APIC_DEFAULT_MAX_APIC_ID: u32 = 0xfe;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApicProbeDriver {
    pub name: &'static str,
    pub probe_result: bool,
}

pub const fn default_get_apic_id(raw_id_register: u32, xapic: bool, extd_apicid: bool) -> u32 {
    if xapic || extd_apicid {
        (raw_id_register >> 24) & 0xff
    } else {
        (raw_id_register >> 24) & 0x0f
    }
}

pub const fn probe_default() -> bool {
    true
}

pub fn parse_apic(
    arg: Option<&str>,
    drivers: &[ApicProbeDriver],
) -> Result<Option<&'static str>, i32> {
    let arg = arg.ok_or(-EINVAL)?;
    for driver in drivers {
        if driver.name == arg {
            return Ok(Some(driver.name));
        }
    }
    Ok(None)
}

pub fn x86_32_probe_apic(cmdline_apic: bool, drivers: &[ApicProbeDriver]) -> Option<&'static str> {
    if cmdline_apic {
        return None;
    }
    drivers
        .iter()
        .find(|driver| driver.probe_result)
        .map(|driver| driver.name)
}

pub const APIC_DEFAULT: ApicProbeDriver = ApicProbeDriver {
    name: APIC_DEFAULT_NAME,
    probe_result: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_32_default_driver_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/apic/probe_32.c"
        ));
        assert!(source.contains("static u32 default_get_apic_id(u32 x)"));
        assert!(source.contains("return (x >> 24) & 0xFF;"));
        assert!(source.contains("return (x >> 24) & 0x0F;"));
        assert!(source.contains("static int probe_default(void)"));
        assert!(source.contains(".name\t\t\t\t= \"default\""));
        assert!(source.contains(".dest_mode_logical\t\t= true"));
        assert!(source.contains(".max_apic_id\t\t\t= 0xFE"));
        assert!(source.contains("struct apic *apic __ro_after_init = &apic_default;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(apic);"));
        assert!(source.contains("early_param(\"apic\", parse_apic);"));
        assert!(source.contains("void __init x86_32_probe_apic(void)"));
        assert!(source.contains("panic(\"Didn't find an APIC driver\");"));

        assert_eq!(default_get_apic_id(0xab00_0000, true, false), 0xab);
        assert_eq!(default_get_apic_id(0xab00_0000, false, false), 0x0b);
        assert!(probe_default());
        assert_eq!(APIC_DEFAULT.name, APIC_DEFAULT_NAME);
        assert_eq!(APIC_DEFAULT_MAX_APIC_ID, 0xfe);
    }

    #[test]
    fn parse_apic_and_probe_walk_driver_table_in_linux_order() {
        let drivers = [
            ApicProbeDriver {
                name: "bigsmp",
                probe_result: false,
            },
            ApicProbeDriver {
                name: "default",
                probe_result: true,
            },
        ];
        assert_eq!(parse_apic(None, &drivers), Err(-EINVAL));
        assert_eq!(parse_apic(Some("default"), &drivers), Ok(Some("default")));
        assert_eq!(parse_apic(Some("missing"), &drivers), Ok(None));
        assert_eq!(x86_32_probe_apic(false, &drivers), Some("default"));
        assert_eq!(x86_32_probe_apic(true, &drivers), None);
    }
}
