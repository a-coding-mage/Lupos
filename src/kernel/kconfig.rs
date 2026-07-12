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
pub const CONFIG_USB: Tristate = config_value!(CONFIG_USB);
pub const CONFIG_USB_XHCI: Tristate = config_value!(CONFIG_USB_XHCI);
pub const CONFIG_SOUND: Tristate = config_value!(CONFIG_SOUND);
pub const CONFIG_SND: Tristate = config_value!(CONFIG_SND);
pub const CONFIG_SND_TIMER: Tristate = config_value!(CONFIG_SND_TIMER);
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
pub const CONFIG_VIRTIO: Tristate = config_value!(CONFIG_VIRTIO);
pub const CONFIG_VIRTIO_PCI: Tristate = config_value!(CONFIG_VIRTIO_PCI);
pub const CONFIG_VIRTIO_BLK: Tristate = config_value!(CONFIG_VIRTIO_BLK);
pub const CONFIG_VIRTIO_NET: Tristate = config_value!(CONFIG_VIRTIO_NET);
pub const CONFIG_NET: Tristate = config_value!(CONFIG_NET);
pub const CONFIG_NETDEVICES: Tristate = config_value!(CONFIG_NETDEVICES);
pub const CONFIG_E1000: Tristate = config_value!(CONFIG_E1000);

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
        name: "CONFIG_USB",
        value: CONFIG_USB,
    },
    ConfigSymbol {
        name: "CONFIG_USB_XHCI",
        value: CONFIG_USB_XHCI,
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
        name: "CONFIG_E1000",
        value: CONFIG_E1000,
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
        assert_eq!(lookup("CONFIG_E1000"), Some(CONFIG_E1000));
        assert_eq!(lookup("CONFIG_DOES_NOT_EXIST"), None);
    }
}
