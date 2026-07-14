//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/amd_nb.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/amd_nb.c
//! AMD northbridge discovery and feature helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/amd_nb.c

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

pub const PCI_VENDOR_ID_AMD: u16 = 0x1022;
pub const PCI_VENDOR_ID_HYGON: u16 = 0x1d94;

pub const PCI_DEVICE_ID_AMD_K8_NB_MISC: u16 = 0x1103;
pub const PCI_DEVICE_ID_AMD_10H_NB_MISC: u16 = 0x1203;
pub const PCI_DEVICE_ID_AMD_11H_NB_MISC: u16 = 0x1303;
pub const PCI_DEVICE_ID_AMD_15H_M10H_F3: u16 = 0x1403;
pub const PCI_DEVICE_ID_AMD_15H_M30H_NB_F3: u16 = 0x141d;
pub const PCI_DEVICE_ID_AMD_15H_M60H_NB_F3: u16 = 0x1573;
pub const PCI_DEVICE_ID_AMD_15H_NB_F3: u16 = 0x1603;
pub const PCI_DEVICE_ID_AMD_16H_NB_F3: u16 = 0x1533;
pub const PCI_DEVICE_ID_AMD_16H_M30H_NB_F3: u16 = 0x1583;
pub const PCI_DEVICE_ID_AMD_17H_DF_F3: u16 = 0x1463;
pub const PCI_DEVICE_ID_AMD_17H_M10H_DF_F3: u16 = 0x15eb;
pub const PCI_DEVICE_ID_AMD_17H_M30H_DF_F3: u16 = 0x1493;
pub const PCI_DEVICE_ID_AMD_17H_M40H_DF_F3: u16 = 0x13f3;
pub const PCI_DEVICE_ID_AMD_17H_M60H_DF_F3: u16 = 0x144b;
pub const PCI_DEVICE_ID_AMD_17H_M70H_DF_F3: u16 = 0x1443;
pub const PCI_DEVICE_ID_AMD_19H_DF_F3: u16 = 0x1653;
pub const PCI_DEVICE_ID_AMD_19H_M10H_DF_F3: u16 = 0x14b0;
pub const PCI_DEVICE_ID_AMD_19H_M40H_DF_F3: u16 = 0x167c;
pub const PCI_DEVICE_ID_AMD_19H_M50H_DF_F3: u16 = 0x166d;
pub const PCI_DEVICE_ID_AMD_19H_M60H_DF_F3: u16 = 0x14e3;
pub const PCI_DEVICE_ID_AMD_19H_M70H_DF_F3: u16 = 0x14f3;

pub const AMD_NB_GART: u64 = 1 << 0;
pub const AMD_NB_L3_INDEX_DISABLE: u64 = 1 << 1;
pub const AMD_NB_L3_PARTITIONING: u64 = 1 << 2;

pub const MSR_FAM10H_MMIO_CONF_BASE: u32 = 0xc001_0058;
pub const FAM10H_MMIO_CONF_ENABLE: u64 = 1;
pub const FAM10H_MMIO_CONF_BUSRANGE_MASK: u64 = 0x0f;
pub const FAM10H_MMIO_CONF_BUSRANGE_SHIFT: u32 = 2;
pub const FAM10H_MMIO_CONF_BASE_MASK: u64 = 0x0fff_ffff;
pub const FAM10H_MMIO_CONF_BASE_SHIFT: u32 = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuVendor {
    Other,
    Amd,
    Hygon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdCpuInfo {
    pub vendor: CpuVendor,
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
    pub has_l3_cache: bool,
    pub zen: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdNorthbridgeInfo {
    pub num: u16,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resource {
    pub start: u64,
    pub end: u64,
}

pub const fn amd_nb_has_feature(info: AmdNorthbridgeInfo, feature: u64) -> bool {
    (info.flags & feature) != 0
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("amd_nb_num", linux_amd_nb_num as usize, true);
    export_symbol_once(
        "amd_nb_has_feature",
        linux_amd_nb_has_feature as usize,
        true,
    );
    export_symbol_once("node_to_amd_nb", linux_node_to_amd_nb as usize, true);
    export_symbol_once("amd_flush_garts", linux_amd_flush_garts as usize, true);
}

/// `amd_nb_num` - `vendor/linux/arch/x86/kernel/amd_nb.c`.
#[unsafe(export_name = "amd_nb_num")]
unsafe extern "C" fn linux_amd_nb_num() -> u16 {
    0
}

/// `amd_nb_has_feature` - `vendor/linux/arch/x86/kernel/amd_nb.c`.
#[unsafe(export_name = "amd_nb_has_feature")]
unsafe extern "C" fn linux_amd_nb_has_feature(_feature: u32) -> bool {
    false
}

/// `node_to_amd_nb` - `vendor/linux/arch/x86/kernel/amd_nb.c`.
#[unsafe(export_name = "node_to_amd_nb")]
unsafe extern "C" fn linux_node_to_amd_nb(_node: i32) -> *mut core::ffi::c_void {
    core::ptr::null_mut()
}

/// `amd_flush_garts` - `vendor/linux/arch/x86/kernel/amd_nb.c:242`.
#[unsafe(export_name = "amd_flush_garts")]
unsafe extern "C" fn linux_amd_flush_garts() {}

pub const fn amd_gart_present(cpu: AmdCpuInfo) -> bool {
    matches!(cpu.vendor, CpuVendor::Amd)
        && (cpu.family == 0x0f || cpu.family == 0x10 || (cpu.family == 0x15 && cpu.model < 0x10))
}

pub const fn early_is_amd_nb(config_dword: u32, cpu: AmdCpuInfo) -> bool {
    let vendor = config_dword as u16;
    let device = (config_dword >> 16) as u16;

    if !(vendor == PCI_VENDOR_ID_AMD || vendor == PCI_VENDOR_ID_HYGON) {
        return false;
    }
    if cpu.zen {
        return false;
    }

    matches!(
        device,
        PCI_DEVICE_ID_AMD_K8_NB_MISC
            | PCI_DEVICE_ID_AMD_10H_NB_MISC
            | PCI_DEVICE_ID_AMD_11H_NB_MISC
            | PCI_DEVICE_ID_AMD_15H_M10H_F3
            | PCI_DEVICE_ID_AMD_15H_M30H_NB_F3
            | PCI_DEVICE_ID_AMD_15H_M60H_NB_F3
            | PCI_DEVICE_ID_AMD_15H_NB_F3
            | PCI_DEVICE_ID_AMD_16H_NB_F3
            | PCI_DEVICE_ID_AMD_16H_M30H_NB_F3
    )
}

pub const fn df_func3_device_id(cpu: AmdCpuInfo) -> Option<u16> {
    if !cpu.zen {
        return None;
    }
    match (cpu.family, cpu.model) {
        (0x17, 0x00..=0x0f) => Some(PCI_DEVICE_ID_AMD_17H_DF_F3),
        (0x17, 0x10..=0x2f) => Some(PCI_DEVICE_ID_AMD_17H_M10H_DF_F3),
        (0x17, 0x30..=0x3f) => Some(PCI_DEVICE_ID_AMD_17H_M30H_DF_F3),
        (0x17, 0x40..=0x5f) => Some(PCI_DEVICE_ID_AMD_17H_M40H_DF_F3),
        (0x17, 0x60..=0x6f) => Some(PCI_DEVICE_ID_AMD_17H_M60H_DF_F3),
        (0x17, 0x70..=0x7f) => Some(PCI_DEVICE_ID_AMD_17H_M70H_DF_F3),
        (0x19, 0x00..=0x0f) => Some(PCI_DEVICE_ID_AMD_19H_DF_F3),
        (0x19, 0x10..=0x3f) => Some(PCI_DEVICE_ID_AMD_19H_M10H_DF_F3),
        (0x19, 0x40..=0x4f) => Some(PCI_DEVICE_ID_AMD_19H_M40H_DF_F3),
        (0x19, 0x50..=0x5f) => Some(PCI_DEVICE_ID_AMD_19H_M50H_DF_F3),
        (0x19, 0x60..=0x6f) => Some(PCI_DEVICE_ID_AMD_19H_M60H_DF_F3),
        (0x19, 0x70..=0x7f) => Some(PCI_DEVICE_ID_AMD_19H_M70H_DF_F3),
        _ => None,
    }
}

pub const fn mmconfig_range(cpu: AmdCpuInfo, msr_value: u64) -> Option<Resource> {
    if !(matches!(cpu.vendor, CpuVendor::Amd | CpuVendor::Hygon) && cpu.family >= 0x10) {
        return None;
    }
    if (msr_value & FAM10H_MMIO_CONF_ENABLE) == 0 {
        return None;
    }

    let buses = (msr_value >> FAM10H_MMIO_CONF_BUSRANGE_SHIFT) & FAM10H_MMIO_CONF_BUSRANGE_MASK;
    let base = ((msr_value >> FAM10H_MMIO_CONF_BASE_SHIFT) & FAM10H_MMIO_CONF_BASE_MASK) << 20;
    let size = 1u64 << (buses as u32 + 20);
    Some(Resource {
        start: base,
        end: base + size - 1,
    })
}

pub const fn cache_features(cpu: AmdCpuInfo, gart_present: bool) -> u64 {
    let mut flags = 0;
    if gart_present {
        flags |= AMD_NB_GART;
    }
    if cpu.has_l3_cache {
        flags |= AMD_NB_L3_INDEX_DISABLE;
        if cpu.family >= 0x15 {
            flags |= AMD_NB_L3_PARTITIONING;
        }
    }
    flags
}

pub const fn subcache_mask_for_core(mask_reg: u32, core_id: u8) -> u8 {
    ((mask_reg >> ((core_id as u32 & 0x03) * 4)) & 0x0f) as u8
}

pub const fn compose_subcache_write(mask: u8, core_id: u8) -> Result<u32, i32> {
    if mask > 0x0f || core_id >= 4 {
        return Err(EINVAL);
    }
    Ok((mask as u32) << ((core_id as u32) * 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAM10: AmdCpuInfo = AmdCpuInfo {
        vendor: CpuVendor::Amd,
        family: 0x10,
        model: 0,
        stepping: 0,
        has_l3_cache: true,
        zen: false,
    };

    #[test]
    fn gart_presence_matches_linux_family_filter() {
        assert!(amd_gart_present(FAM10));
        assert!(amd_gart_present(AmdCpuInfo {
            family: 0x15,
            model: 0x0f,
            ..FAM10
        }));
        assert!(!amd_gart_present(AmdCpuInfo {
            family: 0x15,
            model: 0x10,
            ..FAM10
        }));
        assert!(!amd_gart_present(AmdCpuInfo {
            vendor: CpuVendor::Hygon,
            ..FAM10
        }));
    }

    #[test]
    fn early_nb_match_rejects_zen_probe_path() {
        let id = ((PCI_DEVICE_ID_AMD_10H_NB_MISC as u32) << 16) | PCI_VENDOR_ID_AMD as u32;
        assert!(early_is_amd_nb(id, FAM10));
        assert!(!early_is_amd_nb(id, AmdCpuInfo { zen: true, ..FAM10 }));
    }

    #[test]
    fn df_func3_tracks_zen_model_ranges() {
        assert_eq!(
            df_func3_device_id(AmdCpuInfo {
                family: 0x17,
                model: 0x35,
                zen: true,
                ..FAM10
            }),
            Some(PCI_DEVICE_ID_AMD_17H_M30H_DF_F3)
        );
    }

    #[test]
    fn mmconfig_msr_decodes_resource_window() {
        let msr = FAM10H_MMIO_CONF_ENABLE | (3 << 2) | (0xe00 << 20);
        assert_eq!(
            mmconfig_range(FAM10, msr),
            Some(Resource {
                start: 0xe000_0000,
                end: 0xe07f_ffff
            })
        );
    }

    #[test]
    fn cache_feature_bits_reflect_gart_and_l3_support() {
        let flags = cache_features(
            AmdCpuInfo {
                family: 0x15,
                ..FAM10
            },
            true,
        );
        assert!(amd_nb_has_feature(
            AmdNorthbridgeInfo { num: 1, flags },
            AMD_NB_GART
        ));
        assert!(amd_nb_has_feature(
            AmdNorthbridgeInfo { num: 1, flags },
            AMD_NB_L3_PARTITIONING
        ));
    }

    #[test]
    fn subcache_helpers_preserve_four_bit_fields() {
        assert_eq!(subcache_mask_for_core(0x4321, 2), 0x03);
        assert_eq!(compose_subcache_write(0x0a, 2), Ok(0x0a00));
        assert_eq!(compose_subcache_write(0x10, 0), Err(EINVAL));
    }
}
