//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/pci/broadcom_bus.c
//! test-origin: linux:vendor/linux/arch/x86/pci/broadcom_bus.c
//! Broadcom/ServerWorks CNB20LE host-bridge resource discovery.

pub const PCI_VENDOR_ID_SERVERWORKS: u16 = 0x1166;
pub const PCI_DEVICE_ID_SERVERWORKS_LE: u16 = 0x0009;

pub const IORESOURCE_IO: u32 = 0x0000_0100;
pub const IORESOURCE_MEM: u32 = 0x0000_0200;
pub const IORESOURCE_BUS: u32 = 0x0000_1000;
pub const IORESOURCE_PREFETCH: u32 = 0x0000_2000;

pub const BROADCOM_MAX_RESOURCES: usize = 9;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Resource {
    pub start: u64,
    pub end: u64,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cnb20leConfig {
    pub first_bus: u8,
    pub last_bus: u8,
    pub non_prefetch_start_word: u16,
    pub non_prefetch_end_word: u16,
    pub prefetch_start_word: u16,
    pub prefetch_end_word: u16,
    pub io_start_word: u16,
    pub io_end_word: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cnb20leResources {
    pub entries: [Option<Resource>; BROADCOM_MAX_RESOURCES],
    pub len: usize,
}

impl Cnb20leResources {
    const fn new() -> Self {
        Self {
            entries: [None; BROADCOM_MAX_RESOURCES],
            len: 0,
        }
    }

    fn push(&mut self, resource: Resource) {
        if self.len < BROADCOM_MAX_RESOURCES {
            self.entries[self.len] = Some(resource);
            self.len += 1;
        }
    }
}

pub fn cnb20le_res(config: Cnb20leConfig) -> Cnb20leResources {
    let mut resources = Cnb20leResources::new();

    if config.first_bus == 0 {
        resources.push(Resource {
            start: 0x01f0,
            end: 0x01f7,
            flags: IORESOURCE_IO,
        });
        resources.push(Resource {
            start: 0x03f6,
            end: 0x03f6,
            flags: IORESOURCE_IO,
        });
        resources.push(Resource {
            start: 0x0170,
            end: 0x0177,
            flags: IORESOURCE_IO,
        });
        resources.push(Resource {
            start: 0x0376,
            end: 0x0376,
            flags: IORESOURCE_IO,
        });
        resources.push(Resource {
            start: 0xffa0,
            end: 0xffaf,
            flags: IORESOURCE_IO,
        });
    }

    push_window(
        &mut resources,
        config.non_prefetch_start_word,
        config.non_prefetch_end_word,
        IORESOURCE_MEM,
        16,
        0xffff,
    );
    push_window(
        &mut resources,
        config.prefetch_start_word,
        config.prefetch_end_word,
        IORESOURCE_MEM | IORESOURCE_PREFETCH,
        16,
        0xffff,
    );
    push_window(
        &mut resources,
        config.io_start_word,
        config.io_end_word,
        IORESOURCE_IO,
        0,
        0,
    );
    resources.push(Resource {
        start: config.first_bus as u64,
        end: config.last_bus as u64,
        flags: IORESOURCE_BUS,
    });

    resources
}

fn push_window(
    resources: &mut Cnb20leResources,
    start_word: u16,
    end_word: u16,
    flags: u32,
    shift: u32,
    end_mask: u64,
) {
    if start_word == end_word {
        return;
    }
    resources.push(Resource {
        start: (start_word as u64) << shift,
        end: ((end_word as u64) << shift) | end_mask,
        flags,
    });
}

pub const fn broadcom_postcore_should_probe(
    acpi_disabled: bool,
    acpi_root_pointer: bool,
    vendor: u16,
    device: u16,
) -> bool {
    if !acpi_disabled && acpi_root_pointer {
        return false;
    }
    vendor == PCI_VENDOR_ID_SERVERWORKS && device == PCI_DEVICE_ID_SERVERWORKS_LE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcom_cnb20le_resources_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/pci/broadcom_bus.c"
        ));
        assert!(source.contains("static void __init cnb20le_res(u8 bus, u8 slot, u8 func)"));
        assert!(source.contains("fbus = read_pci_config_byte(bus, slot, func, 0x44);"));
        assert!(source.contains("lbus = read_pci_config_byte(bus, slot, func, 0x45);"));
        assert!(source.contains("update_res(info, 0x01f0, 0x01f7, IORESOURCE_IO, 0);"));
        assert!(source.contains("word1 = read_pci_config_16(bus, slot, func, 0xc0);"));
        assert!(source.contains("res.flags = IORESOURCE_MEM | IORESOURCE_PREFETCH;"));
        assert!(source.contains("word1 = read_pci_config_16(bus, slot, func, 0xd0);"));
        assert!(source.contains("res.flags = IORESOURCE_BUS;"));
        assert!(source.contains("if (!acpi_disabled && acpi_os_get_root_pointer())"));
        assert!(source.contains("vendor == PCI_VENDOR_ID_SERVERWORKS"));
        assert!(source.contains("device == PCI_DEVICE_ID_SERVERWORKS_LE"));
        assert!(source.contains("postcore_initcall(broadcom_postcore_init);"));

        let resources = cnb20le_res(Cnb20leConfig {
            first_bus: 0,
            last_bus: 2,
            non_prefetch_start_word: 0x1000,
            non_prefetch_end_word: 0x1001,
            prefetch_start_word: 0x2000,
            prefetch_end_word: 0x2002,
            io_start_word: 0x3000,
            io_end_word: 0x3010,
        });
        assert_eq!(resources.entries[0].unwrap().start, 0x01f0);
        assert_eq!(resources.entries[5].unwrap().start, 0x1000_0000);
        assert_eq!(resources.entries[5].unwrap().end, 0x1001_ffff);
        assert_eq!(
            resources.entries[6].unwrap().flags,
            IORESOURCE_MEM | IORESOURCE_PREFETCH
        );
        assert_eq!(resources.entries[7].unwrap().start, 0x3000);
        assert_eq!(resources.entries[8].unwrap().flags, IORESOURCE_BUS);
        assert_eq!(resources.len, 9);
    }

    #[test]
    fn broadcom_postcore_skips_when_acpi_root_exists() {
        assert!(broadcom_postcore_should_probe(
            true,
            false,
            PCI_VENDOR_ID_SERVERWORKS,
            PCI_DEVICE_ID_SERVERWORKS_LE
        ));
        assert!(!broadcom_postcore_should_probe(
            false,
            true,
            PCI_VENDOR_ID_SERVERWORKS,
            PCI_DEVICE_ID_SERVERWORKS_LE
        ));
        assert!(!broadcom_postcore_should_probe(
            true,
            false,
            PCI_VENDOR_ID_SERVERWORKS,
            0xffff
        ));
    }
}
