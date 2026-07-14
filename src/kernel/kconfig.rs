//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Kconfig bridge for Rust code.
//!
//! `build.rs` consumes Linux's generated `src/include/generated/rustc_cfg` and
//! turns each `CONFIG_*` entry into `--cfg` flags.  This module is the small
//! const-table layer the kernel can query without reparsing `.config`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tristate {
    No,
    Module,
    Yes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfigSymbol {
    pub name: &'static str,
    pub value: Tristate,
}

macro_rules! config_value {
    ($symbol:ident) => {{
        if cfg!($symbol = "y") {
            Tristate::Yes
        } else if cfg!($symbol = "m") {
            Tristate::Module
        } else {
            Tristate::No
        }
    }};
}

pub const CONFIG_LUPOS: Tristate = config_value!(CONFIG_LUPOS);
pub const CONFIG_MODULES: Tristate = config_value!(CONFIG_MODULES);
pub const CONFIG_SMP: Tristate = config_value!(CONFIG_SMP);
pub const CONFIG_X86_64: Tristate = config_value!(CONFIG_X86_64);
pub const CONFIG_PCI: Tristate = config_value!(CONFIG_PCI);
pub const CONFIG_ACPI: Tristate = config_value!(CONFIG_ACPI);
pub const CONFIG_IOMMU: Tristate = config_value!(CONFIG_IOMMU);
pub const CONFIG_SERIAL_8250: Tristate = config_value!(CONFIG_SERIAL_8250);
pub const CONFIG_FRAMEBUFFER: Tristate = config_value!(CONFIG_FRAMEBUFFER);
pub const CONFIG_AGP: Tristate = config_value!(CONFIG_AGP);
pub const CONFIG_AGP_AMD64: Tristate = config_value!(CONFIG_AGP_AMD64);
pub const CONFIG_AGP_INTEL: Tristate = config_value!(CONFIG_AGP_INTEL);
pub const CONFIG_DRM: Tristate = config_value!(CONFIG_DRM);
pub const CONFIG_DRM_I915: Tristate = config_value!(CONFIG_DRM_I915);
pub const CONFIG_DRM_VIRTIO_GPU: Tristate = config_value!(CONFIG_DRM_VIRTIO_GPU);
pub const CONFIG_INPUT: Tristate = config_value!(CONFIG_INPUT);
pub const CONFIG_HID: Tristate = config_value!(CONFIG_HID);
pub const CONFIG_I2C: Tristate = config_value!(CONFIG_I2C);
pub const CONFIG_I2C_SMBUS: Tristate = config_value!(CONFIG_I2C_SMBUS);
pub const CONFIG_I2C_ALGOBIT: Tristate = config_value!(CONFIG_I2C_ALGOBIT);
pub const CONFIG_I2C_I801: Tristate = config_value!(CONFIG_I2C_I801);
pub const CONFIG_USB: Tristate = config_value!(CONFIG_USB);
pub const CONFIG_USB_XHCI: Tristate = config_value!(CONFIG_USB_XHCI);
pub const CONFIG_USB_PCI: Tristate = config_value!(CONFIG_USB_PCI);
pub const CONFIG_USB_EHCI_HCD: Tristate = config_value!(CONFIG_USB_EHCI_HCD);
pub const CONFIG_USB_EHCI_PCI: Tristate = config_value!(CONFIG_USB_EHCI_PCI);
pub const CONFIG_USB_OHCI_HCD: Tristate = config_value!(CONFIG_USB_OHCI_HCD);
pub const CONFIG_USB_OHCI_HCD_PCI: Tristate = config_value!(CONFIG_USB_OHCI_HCD_PCI);
pub const CONFIG_USB_UHCI_HCD: Tristate = config_value!(CONFIG_USB_UHCI_HCD);
pub const CONFIG_USB_MON: Tristate = config_value!(CONFIG_USB_MON);
pub const CONFIG_USB_PRINTER: Tristate = config_value!(CONFIG_USB_PRINTER);
pub const CONFIG_SOUND: Tristate = config_value!(CONFIG_SOUND);
pub const CONFIG_SND: Tristate = config_value!(CONFIG_SND);
pub const CONFIG_SND_TIMER: Tristate = config_value!(CONFIG_SND_TIMER);
pub const CONFIG_SND_HRTIMER: Tristate = config_value!(CONFIG_SND_HRTIMER);
pub const CONFIG_SND_SEQ_DEVICE: Tristate = config_value!(CONFIG_SND_SEQ_DEVICE);
pub const CONFIG_SND_SEQUENCER: Tristate = config_value!(CONFIG_SND_SEQUENCER);
pub const CONFIG_SND_SEQ_DUMMY: Tristate = config_value!(CONFIG_SND_SEQ_DUMMY);
pub const CONFIG_SND_PCM: Tristate = config_value!(CONFIG_SND_PCM);
pub const CONFIG_SND_HWDEP: Tristate = config_value!(CONFIG_SND_HWDEP);
pub const CONFIG_SND_HDA_CORE: Tristate = config_value!(CONFIG_SND_HDA_CORE);
pub const CONFIG_SND_INTEL_SOUNDWIRE_ACPI: Tristate =
    config_value!(CONFIG_SND_INTEL_SOUNDWIRE_ACPI);
pub const CONFIG_SND_INTEL_DSP_CONFIG: Tristate = config_value!(CONFIG_SND_INTEL_DSP_CONFIG);
pub const CONFIG_SND_HDA: Tristate = config_value!(CONFIG_SND_HDA);
pub const CONFIG_SND_HDA_HWDEP: Tristate = config_value!(CONFIG_SND_HDA_HWDEP);
pub const CONFIG_SND_HDA_INTEL: Tristate = config_value!(CONFIG_SND_HDA_INTEL);
pub const CONFIG_SND_HDA_GENERIC: Tristate = config_value!(CONFIG_SND_HDA_GENERIC);
pub const CONFIG_SCSI_COMMON: Tristate = config_value!(CONFIG_SCSI_COMMON);
pub const CONFIG_SCSI: Tristate = config_value!(CONFIG_SCSI);
pub const CONFIG_BLK_DEV_SD: Tristate = config_value!(CONFIG_BLK_DEV_SD);
pub const CONFIG_CDROM: Tristate = config_value!(CONFIG_CDROM);
pub const CONFIG_BLK_DEV_SR: Tristate = config_value!(CONFIG_BLK_DEV_SR);
pub const CONFIG_CHR_DEV_SG: Tristate = config_value!(CONFIG_CHR_DEV_SG);
pub const CONFIG_SCSI_SPI_ATTRS: Tristate = config_value!(CONFIG_SCSI_SPI_ATTRS);
pub const CONFIG_SCSI_VIRTIO: Tristate = config_value!(CONFIG_SCSI_VIRTIO);
pub const CONFIG_USB_STORAGE: Tristate = config_value!(CONFIG_USB_STORAGE);
pub const CONFIG_ATA: Tristate = config_value!(CONFIG_ATA);
pub const CONFIG_SATA_AHCI: Tristate = config_value!(CONFIG_SATA_AHCI);
pub const CONFIG_ATA_PIIX: Tristate = config_value!(CONFIG_ATA_PIIX);
pub const CONFIG_PATA_AMD: Tristate = config_value!(CONFIG_PATA_AMD);
pub const CONFIG_PATA_OLDPIIX: Tristate = config_value!(CONFIG_PATA_OLDPIIX);
pub const CONFIG_PATA_SCH: Tristate = config_value!(CONFIG_PATA_SCH);
pub const CONFIG_VIRTIO: Tristate = config_value!(CONFIG_VIRTIO);
pub const CONFIG_VIRTIO_PCI: Tristate = config_value!(CONFIG_VIRTIO_PCI);
pub const CONFIG_VIRTIO_BLK: Tristate = config_value!(CONFIG_VIRTIO_BLK);
pub const CONFIG_VIRTIO_CONSOLE: Tristate = config_value!(CONFIG_VIRTIO_CONSOLE);
pub const CONFIG_VIRTIO_INPUT: Tristate = config_value!(CONFIG_VIRTIO_INPUT);
pub const CONFIG_VIRTIO_NET: Tristate = config_value!(CONFIG_VIRTIO_NET);
pub const CONFIG_NET: Tristate = config_value!(CONFIG_NET);
pub const CONFIG_NETDEVICES: Tristate = config_value!(CONFIG_NETDEVICES);
pub const CONFIG_NETCONSOLE: Tristate = config_value!(CONFIG_NETCONSOLE);
pub const CONFIG_NET_9P: Tristate = config_value!(CONFIG_NET_9P);
pub const CONFIG_NET_9P_VIRTIO: Tristate = config_value!(CONFIG_NET_9P_VIRTIO);
pub const CONFIG_MII: Tristate = config_value!(CONFIG_MII);
pub const CONFIG_PHYLIB: Tristate = config_value!(CONFIG_PHYLIB);
pub const CONFIG_REALTEK_PHY: Tristate = config_value!(CONFIG_REALTEK_PHY);
pub const CONFIG_E100: Tristate = config_value!(CONFIG_E100);
pub const CONFIG_E1000: Tristate = config_value!(CONFIG_E1000);
pub const CONFIG_E1000E: Tristate = config_value!(CONFIG_E1000E);
pub const CONFIG_SKY2: Tristate = config_value!(CONFIG_SKY2);
pub const CONFIG_TIGON3: Tristate = config_value!(CONFIG_TIGON3);
pub const CONFIG_FORCEDETH: Tristate = config_value!(CONFIG_FORCEDETH);
pub const CONFIG_8139TOO: Tristate = config_value!(CONFIG_8139TOO);
pub const CONFIG_R8169: Tristate = config_value!(CONFIG_R8169);
pub const CONFIG_X86_PKG_TEMP_THERMAL: Tristate = config_value!(CONFIG_X86_PKG_TEMP_THERMAL);

pub const CONFIG_SYMBOLS: &[ConfigSymbol] = &[
    ConfigSymbol {
        name: "CONFIG_LUPOS",
        value: CONFIG_LUPOS,
    },
    ConfigSymbol {
        name: "CONFIG_MODULES",
        value: CONFIG_MODULES,
    },
    ConfigSymbol {
        name: "CONFIG_SMP",
        value: CONFIG_SMP,
    },
    ConfigSymbol {
        name: "CONFIG_X86_64",
        value: CONFIG_X86_64,
    },
    ConfigSymbol {
        name: "CONFIG_PCI",
        value: CONFIG_PCI,
    },
    ConfigSymbol {
        name: "CONFIG_ACPI",
        value: CONFIG_ACPI,
    },
    ConfigSymbol {
        name: "CONFIG_IOMMU",
        value: CONFIG_IOMMU,
    },
    ConfigSymbol {
        name: "CONFIG_SERIAL_8250",
        value: CONFIG_SERIAL_8250,
    },
    ConfigSymbol {
        name: "CONFIG_FRAMEBUFFER",
        value: CONFIG_FRAMEBUFFER,
    },
    ConfigSymbol {
        name: "CONFIG_AGP",
        value: CONFIG_AGP,
    },
    ConfigSymbol {
        name: "CONFIG_AGP_AMD64",
        value: CONFIG_AGP_AMD64,
    },
    ConfigSymbol {
        name: "CONFIG_AGP_INTEL",
        value: CONFIG_AGP_INTEL,
    },
    ConfigSymbol {
        name: "CONFIG_DRM",
        value: CONFIG_DRM,
    },
    ConfigSymbol {
        name: "CONFIG_DRM_I915",
        value: CONFIG_DRM_I915,
    },
    ConfigSymbol {
        name: "CONFIG_DRM_VIRTIO_GPU",
        value: CONFIG_DRM_VIRTIO_GPU,
    },
    ConfigSymbol {
        name: "CONFIG_INPUT",
        value: CONFIG_INPUT,
    },
    ConfigSymbol {
        name: "CONFIG_HID",
        value: CONFIG_HID,
    },
    ConfigSymbol {
        name: "CONFIG_I2C",
        value: CONFIG_I2C,
    },
    ConfigSymbol {
        name: "CONFIG_I2C_SMBUS",
        value: CONFIG_I2C_SMBUS,
    },
    ConfigSymbol {
        name: "CONFIG_I2C_ALGOBIT",
        value: CONFIG_I2C_ALGOBIT,
    },
    ConfigSymbol {
        name: "CONFIG_I2C_I801",
        value: CONFIG_I2C_I801,
    },
    ConfigSymbol {
        name: "CONFIG_USB",
        value: CONFIG_USB,
    },
    ConfigSymbol {
        name: "CONFIG_USB_XHCI",
        value: CONFIG_USB_XHCI,
    },
    ConfigSymbol {
        name: "CONFIG_USB_PCI",
        value: CONFIG_USB_PCI,
    },
    ConfigSymbol {
        name: "CONFIG_USB_EHCI_HCD",
        value: CONFIG_USB_EHCI_HCD,
    },
    ConfigSymbol {
        name: "CONFIG_USB_EHCI_PCI",
        value: CONFIG_USB_EHCI_PCI,
    },
    ConfigSymbol {
        name: "CONFIG_USB_OHCI_HCD",
        value: CONFIG_USB_OHCI_HCD,
    },
    ConfigSymbol {
        name: "CONFIG_USB_OHCI_HCD_PCI",
        value: CONFIG_USB_OHCI_HCD_PCI,
    },
    ConfigSymbol {
        name: "CONFIG_USB_UHCI_HCD",
        value: CONFIG_USB_UHCI_HCD,
    },
    ConfigSymbol {
        name: "CONFIG_USB_MON",
        value: CONFIG_USB_MON,
    },
    ConfigSymbol {
        name: "CONFIG_USB_PRINTER",
        value: CONFIG_USB_PRINTER,
    },
    ConfigSymbol {
        name: "CONFIG_SOUND",
        value: CONFIG_SOUND,
    },
    ConfigSymbol {
        name: "CONFIG_SND",
        value: CONFIG_SND,
    },
    ConfigSymbol {
        name: "CONFIG_SND_TIMER",
        value: CONFIG_SND_TIMER,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HRTIMER",
        value: CONFIG_SND_HRTIMER,
    },
    ConfigSymbol {
        name: "CONFIG_SND_SEQ_DEVICE",
        value: CONFIG_SND_SEQ_DEVICE,
    },
    ConfigSymbol {
        name: "CONFIG_SND_SEQUENCER",
        value: CONFIG_SND_SEQUENCER,
    },
    ConfigSymbol {
        name: "CONFIG_SND_SEQ_DUMMY",
        value: CONFIG_SND_SEQ_DUMMY,
    },
    ConfigSymbol {
        name: "CONFIG_SND_PCM",
        value: CONFIG_SND_PCM,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HWDEP",
        value: CONFIG_SND_HWDEP,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HDA_CORE",
        value: CONFIG_SND_HDA_CORE,
    },
    ConfigSymbol {
        name: "CONFIG_SND_INTEL_SOUNDWIRE_ACPI",
        value: CONFIG_SND_INTEL_SOUNDWIRE_ACPI,
    },
    ConfigSymbol {
        name: "CONFIG_SND_INTEL_DSP_CONFIG",
        value: CONFIG_SND_INTEL_DSP_CONFIG,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HDA",
        value: CONFIG_SND_HDA,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HDA_HWDEP",
        value: CONFIG_SND_HDA_HWDEP,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HDA_INTEL",
        value: CONFIG_SND_HDA_INTEL,
    },
    ConfigSymbol {
        name: "CONFIG_SND_HDA_GENERIC",
        value: CONFIG_SND_HDA_GENERIC,
    },
    ConfigSymbol {
        name: "CONFIG_SCSI_COMMON",
        value: CONFIG_SCSI_COMMON,
    },
    ConfigSymbol {
        name: "CONFIG_SCSI",
        value: CONFIG_SCSI,
    },
    ConfigSymbol {
        name: "CONFIG_BLK_DEV_SD",
        value: CONFIG_BLK_DEV_SD,
    },
    ConfigSymbol {
        name: "CONFIG_CDROM",
        value: CONFIG_CDROM,
    },
    ConfigSymbol {
        name: "CONFIG_BLK_DEV_SR",
        value: CONFIG_BLK_DEV_SR,
    },
    ConfigSymbol {
        name: "CONFIG_CHR_DEV_SG",
        value: CONFIG_CHR_DEV_SG,
    },
    ConfigSymbol {
        name: "CONFIG_SCSI_SPI_ATTRS",
        value: CONFIG_SCSI_SPI_ATTRS,
    },
    ConfigSymbol {
        name: "CONFIG_SCSI_VIRTIO",
        value: CONFIG_SCSI_VIRTIO,
    },
    ConfigSymbol {
        name: "CONFIG_USB_STORAGE",
        value: CONFIG_USB_STORAGE,
    },
    ConfigSymbol {
        name: "CONFIG_ATA",
        value: CONFIG_ATA,
    },
    ConfigSymbol {
        name: "CONFIG_SATA_AHCI",
        value: CONFIG_SATA_AHCI,
    },
    ConfigSymbol {
        name: "CONFIG_ATA_PIIX",
        value: CONFIG_ATA_PIIX,
    },
    ConfigSymbol {
        name: "CONFIG_PATA_AMD",
        value: CONFIG_PATA_AMD,
    },
    ConfigSymbol {
        name: "CONFIG_PATA_OLDPIIX",
        value: CONFIG_PATA_OLDPIIX,
    },
    ConfigSymbol {
        name: "CONFIG_PATA_SCH",
        value: CONFIG_PATA_SCH,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO",
        value: CONFIG_VIRTIO,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO_PCI",
        value: CONFIG_VIRTIO_PCI,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO_BLK",
        value: CONFIG_VIRTIO_BLK,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO_CONSOLE",
        value: CONFIG_VIRTIO_CONSOLE,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO_INPUT",
        value: CONFIG_VIRTIO_INPUT,
    },
    ConfigSymbol {
        name: "CONFIG_VIRTIO_NET",
        value: CONFIG_VIRTIO_NET,
    },
    ConfigSymbol {
        name: "CONFIG_NET",
        value: CONFIG_NET,
    },
    ConfigSymbol {
        name: "CONFIG_NETDEVICES",
        value: CONFIG_NETDEVICES,
    },
    ConfigSymbol {
        name: "CONFIG_NETCONSOLE",
        value: CONFIG_NETCONSOLE,
    },
    ConfigSymbol {
        name: "CONFIG_NET_9P",
        value: CONFIG_NET_9P,
    },
    ConfigSymbol {
        name: "CONFIG_NET_9P_VIRTIO",
        value: CONFIG_NET_9P_VIRTIO,
    },
    ConfigSymbol {
        name: "CONFIG_MII",
        value: CONFIG_MII,
    },
    ConfigSymbol {
        name: "CONFIG_PHYLIB",
        value: CONFIG_PHYLIB,
    },
    ConfigSymbol {
        name: "CONFIG_REALTEK_PHY",
        value: CONFIG_REALTEK_PHY,
    },
    ConfigSymbol {
        name: "CONFIG_E100",
        value: CONFIG_E100,
    },
    ConfigSymbol {
        name: "CONFIG_E1000",
        value: CONFIG_E1000,
    },
    ConfigSymbol {
        name: "CONFIG_E1000E",
        value: CONFIG_E1000E,
    },
    ConfigSymbol {
        name: "CONFIG_SKY2",
        value: CONFIG_SKY2,
    },
    ConfigSymbol {
        name: "CONFIG_TIGON3",
        value: CONFIG_TIGON3,
    },
    ConfigSymbol {
        name: "CONFIG_FORCEDETH",
        value: CONFIG_FORCEDETH,
    },
    ConfigSymbol {
        name: "CONFIG_8139TOO",
        value: CONFIG_8139TOO,
    },
    ConfigSymbol {
        name: "CONFIG_R8169",
        value: CONFIG_R8169,
    },
    ConfigSymbol {
        name: "CONFIG_X86_PKG_TEMP_THERMAL",
        value: CONFIG_X86_PKG_TEMP_THERMAL,
    },
];

pub fn lookup(name: &str) -> Option<Tristate> {
    CONFIG_SYMBOLS
        .iter()
        .find(|symbol| symbol.name == name)
        .map(|symbol| symbol.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_symbol_table_is_queryable() {
        assert_eq!(lookup("CONFIG_MODULES"), Some(CONFIG_MODULES));
        assert_eq!(lookup("CONFIG_AGP"), Some(CONFIG_AGP));
        assert_eq!(lookup("CONFIG_AGP_AMD64"), Some(CONFIG_AGP_AMD64));
        assert_eq!(lookup("CONFIG_AGP_INTEL"), Some(CONFIG_AGP_INTEL));
        assert_eq!(lookup("CONFIG_DRM_I915"), Some(CONFIG_DRM_I915));
        assert_eq!(lookup("CONFIG_DRM_VIRTIO_GPU"), Some(CONFIG_DRM_VIRTIO_GPU));
        assert_eq!(lookup("CONFIG_I2C"), Some(CONFIG_I2C));
        assert_eq!(lookup("CONFIG_I2C_SMBUS"), Some(CONFIG_I2C_SMBUS));
        assert_eq!(lookup("CONFIG_I2C_ALGOBIT"), Some(CONFIG_I2C_ALGOBIT));
        assert_eq!(lookup("CONFIG_I2C_I801"), Some(CONFIG_I2C_I801));
        assert_eq!(lookup("CONFIG_USB_PCI"), Some(CONFIG_USB_PCI));
        assert_eq!(lookup("CONFIG_USB_EHCI_HCD"), Some(CONFIG_USB_EHCI_HCD));
        assert_eq!(lookup("CONFIG_USB_EHCI_PCI"), Some(CONFIG_USB_EHCI_PCI));
        assert_eq!(lookup("CONFIG_USB_OHCI_HCD"), Some(CONFIG_USB_OHCI_HCD));
        assert_eq!(
            lookup("CONFIG_USB_OHCI_HCD_PCI"),
            Some(CONFIG_USB_OHCI_HCD_PCI)
        );
        assert_eq!(lookup("CONFIG_USB_UHCI_HCD"), Some(CONFIG_USB_UHCI_HCD));
        assert_eq!(lookup("CONFIG_USB_MON"), Some(CONFIG_USB_MON));
        assert_eq!(lookup("CONFIG_USB_PRINTER"), Some(CONFIG_USB_PRINTER));
        assert_eq!(lookup("CONFIG_USB_STORAGE"), Some(CONFIG_USB_STORAGE));
        assert_eq!(lookup("CONFIG_NETCONSOLE"), Some(CONFIG_NETCONSOLE));
        assert_eq!(lookup("CONFIG_MII"), Some(CONFIG_MII));
        assert_eq!(lookup("CONFIG_PHYLIB"), Some(CONFIG_PHYLIB));
        assert_eq!(lookup("CONFIG_REALTEK_PHY"), Some(CONFIG_REALTEK_PHY));
        assert_eq!(lookup("CONFIG_E100"), Some(CONFIG_E100));
        assert_eq!(lookup("CONFIG_E1000"), Some(CONFIG_E1000));
        assert_eq!(lookup("CONFIG_E1000E"), Some(CONFIG_E1000E));
        assert_eq!(lookup("CONFIG_SKY2"), Some(CONFIG_SKY2));
        assert_eq!(lookup("CONFIG_TIGON3"), Some(CONFIG_TIGON3));
        assert_eq!(lookup("CONFIG_FORCEDETH"), Some(CONFIG_FORCEDETH));
        assert_eq!(lookup("CONFIG_8139TOO"), Some(CONFIG_8139TOO));
        assert_eq!(lookup("CONFIG_R8169"), Some(CONFIG_R8169));
        assert_eq!(lookup("CONFIG_DOES_NOT_EXIST"), None);
    }
}
