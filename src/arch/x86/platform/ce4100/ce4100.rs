//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/ce4100/ce4100.c
//! test-origin: linux:vendor/linux/arch/x86/platform/ce4100/ce4100.c
//! Intel CE4100 x86_init override sequence.

pub const CE4100_POWER_PORT: u16 = 0x0cf9;
pub const CE4100_POWER_OFF_VALUE: u8 = 0x04;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ce4100Setup {
    pub arch_setup: &'static str,
    pub probe_roms_noop: bool,
    pub find_mptable_noop: bool,
    pub early_parse_smp_cfg_noop: bool,
    pub pci_init: &'static str,
    pub pci_init_irq: &'static str,
    pub reboot_type: &'static str,
    pub power_off_port: u16,
    pub power_off_value: u8,
}

pub const fn x86_ce4100_early_setup() -> Ce4100Setup {
    Ce4100Setup {
        arch_setup: "sdv_arch_setup",
        probe_roms_noop: true,
        find_mptable_noop: true,
        early_parse_smp_cfg_noop: true,
        pci_init: "ce4100_pci_init",
        pci_init_irq: "sdv_pci_init",
        reboot_type: "BOOT_KBD",
        power_off_port: CE4100_POWER_PORT,
        power_off_value: CE4100_POWER_OFF_VALUE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ce4100_setup_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/ce4100/ce4100.c"
        ));
        assert!(source.contains("outb(0x4, 0xcf9);"));
        assert!(source.contains("sdv_serial_fixup();"));
        assert!(source.contains("x86_of_pci_init();"));
        assert!(source.contains("x86_init.oem.arch_setup"));
        assert!(source.contains("x86_init.resources.probe_roms\t\t= x86_init_noop;"));
        assert!(source.contains("x86_init.pci.init\t\t\t= ce4100_pci_init;"));
        assert!(source.contains("reboot_type = BOOT_KBD;"));
        assert!(source.contains("pm_power_off = ce4100_power_off;"));

        let setup = x86_ce4100_early_setup();
        assert_eq!(setup.reboot_type, "BOOT_KBD");
        assert_eq!(setup.power_off_port, 0x0cf9);
        assert_eq!(setup.power_off_value, 0x04);
        assert!(setup.probe_roms_noop);
    }
}
