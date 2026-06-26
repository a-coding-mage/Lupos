//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/virt/vmx/tdx/tdx_global_metadata.c
//! test-origin: linux:vendor/linux/arch/x86/virt/vmx/tdx/tdx_global_metadata.c
//! TDX global metadata field readers.

use crate::include::uapi::errno::EINVAL;

pub const TDX_FIELD_VERSION_MINOR: u64 = 0x0800_0001_0000_0003;
pub const TDX_FIELD_VERSION_MAJOR: u64 = 0x0800_0001_0000_0004;
pub const TDX_FIELD_VERSION_UPDATE: u64 = 0x0800_0001_0000_0005;
pub const TDX_FIELD_FEATURES0: u64 = 0x0a00_0003_0000_0008;
pub const TDX_FIELD_TDMR_MAX_TDMRS: u64 = 0x9100_0001_0000_0008;
pub const TDX_FIELD_TDMR_MAX_RESERVED_PER_TDMR: u64 = 0x9100_0001_0000_0009;
pub const TDX_FIELD_TDMR_PAMT_4K_ENTRY_SIZE: u64 = 0x9100_0001_0000_0010;
pub const TDX_FIELD_TDMR_PAMT_2M_ENTRY_SIZE: u64 = 0x9100_0001_0000_0011;
pub const TDX_FIELD_TDMR_PAMT_1G_ENTRY_SIZE: u64 = 0x9100_0001_0000_0012;
pub const TDX_FIELD_TD_CTRL_TDR_BASE_SIZE: u64 = 0x9800_0001_0000_0000;
pub const TDX_FIELD_TD_CTRL_TDCS_BASE_SIZE: u64 = 0x9800_0001_0000_0100;
pub const TDX_FIELD_TD_CTRL_TDVPS_BASE_SIZE: u64 = 0x9800_0001_0000_0200;
pub const TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED0: u64 = 0x1900_0003_0000_0000;
pub const TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED1: u64 = 0x1900_0003_0000_0001;
pub const TDX_FIELD_TD_CONF_XFAM_FIXED0: u64 = 0x1900_0003_0000_0002;
pub const TDX_FIELD_TD_CONF_XFAM_FIXED1: u64 = 0x1900_0003_0000_0003;
pub const TDX_FIELD_TD_CONF_NUM_CPUID_CONFIG: u64 = 0x9900_0001_0000_0004;
pub const TDX_FIELD_TD_CONF_MAX_VCPUS_PER_TD: u64 = 0x9900_0001_0000_0008;
pub const TDX_FIELD_TD_CONF_CPUID_CONFIG_LEAVES_BASE: u64 = 0x9900_0003_0000_0400;
pub const TDX_FIELD_TD_CONF_CPUID_CONFIG_VALUES_BASE: u64 = 0x9900_0003_0000_0500;
pub const TDX_MAX_CPUID_CONFIGS: usize = 128;

pub trait TdxMetadataReader {
    fn read_sys_metadata_field(&mut self, field: u64) -> Result<u64, i32>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxSysInfoVersion {
    pub minor_version: u16,
    pub major_version: u16,
    pub update_version: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxSysInfoFeatures {
    pub tdx_features0: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxSysInfoTdmr {
    pub max_tdmrs: u16,
    pub max_reserved_per_tdmr: u16,
    pub pamt_4k_entry_size: u16,
    pub pamt_2m_entry_size: u16,
    pub pamt_1g_entry_size: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxSysInfoTdCtrl {
    pub tdr_base_size: u16,
    pub tdcs_base_size: u16,
    pub tdvps_base_size: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxSysInfoTdConf {
    pub attributes_fixed0: u64,
    pub attributes_fixed1: u64,
    pub xfam_fixed0: u64,
    pub xfam_fixed1: u64,
    pub num_cpuid_config: u16,
    pub max_vcpus_per_td: u16,
    pub cpuid_config_leaves: [u64; TDX_MAX_CPUID_CONFIGS],
    pub cpuid_config_values: [[u64; 2]; TDX_MAX_CPUID_CONFIGS],
}

impl Default for TdxSysInfoTdConf {
    fn default() -> Self {
        Self {
            attributes_fixed0: 0,
            attributes_fixed1: 0,
            xfam_fixed0: 0,
            xfam_fixed1: 0,
            num_cpuid_config: 0,
            max_vcpus_per_td: 0,
            cpuid_config_leaves: [0; TDX_MAX_CPUID_CONFIGS],
            cpuid_config_values: [[0; 2]; TDX_MAX_CPUID_CONFIGS],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxSysInfo {
    pub version: TdxSysInfoVersion,
    pub features: TdxSysInfoFeatures,
    pub tdmr: TdxSysInfoTdmr,
    pub td_ctrl: TdxSysInfoTdCtrl,
    pub td_conf: TdxSysInfoTdConf,
}

pub fn get_tdx_sys_info_version(
    reader: &mut impl TdxMetadataReader,
) -> Result<TdxSysInfoVersion, i32> {
    Ok(TdxSysInfoVersion {
        minor_version: reader.read_sys_metadata_field(TDX_FIELD_VERSION_MINOR)? as u16,
        major_version: reader.read_sys_metadata_field(TDX_FIELD_VERSION_MAJOR)? as u16,
        update_version: reader.read_sys_metadata_field(TDX_FIELD_VERSION_UPDATE)? as u16,
    })
}

pub fn get_tdx_sys_info_features(
    reader: &mut impl TdxMetadataReader,
) -> Result<TdxSysInfoFeatures, i32> {
    Ok(TdxSysInfoFeatures {
        tdx_features0: reader.read_sys_metadata_field(TDX_FIELD_FEATURES0)?,
    })
}

pub fn get_tdx_sys_info_tdmr(reader: &mut impl TdxMetadataReader) -> Result<TdxSysInfoTdmr, i32> {
    Ok(TdxSysInfoTdmr {
        max_tdmrs: reader.read_sys_metadata_field(TDX_FIELD_TDMR_MAX_TDMRS)? as u16,
        max_reserved_per_tdmr: reader
            .read_sys_metadata_field(TDX_FIELD_TDMR_MAX_RESERVED_PER_TDMR)?
            as u16,
        pamt_4k_entry_size: reader.read_sys_metadata_field(TDX_FIELD_TDMR_PAMT_4K_ENTRY_SIZE)?
            as u16,
        pamt_2m_entry_size: reader.read_sys_metadata_field(TDX_FIELD_TDMR_PAMT_2M_ENTRY_SIZE)?
            as u16,
        pamt_1g_entry_size: reader.read_sys_metadata_field(TDX_FIELD_TDMR_PAMT_1G_ENTRY_SIZE)?
            as u16,
    })
}

pub fn get_tdx_sys_info_td_ctrl(
    reader: &mut impl TdxMetadataReader,
) -> Result<TdxSysInfoTdCtrl, i32> {
    Ok(TdxSysInfoTdCtrl {
        tdr_base_size: reader.read_sys_metadata_field(TDX_FIELD_TD_CTRL_TDR_BASE_SIZE)? as u16,
        tdcs_base_size: reader.read_sys_metadata_field(TDX_FIELD_TD_CTRL_TDCS_BASE_SIZE)? as u16,
        tdvps_base_size: reader.read_sys_metadata_field(TDX_FIELD_TD_CTRL_TDVPS_BASE_SIZE)? as u16,
    })
}

pub fn get_tdx_sys_info_td_conf(
    reader: &mut impl TdxMetadataReader,
) -> Result<TdxSysInfoTdConf, i32> {
    let mut conf = TdxSysInfoTdConf {
        attributes_fixed0: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED0)?,
        attributes_fixed1: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED1)?,
        xfam_fixed0: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_XFAM_FIXED0)?,
        xfam_fixed1: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_XFAM_FIXED1)?,
        num_cpuid_config: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_NUM_CPUID_CONFIG)?
            as u16,
        max_vcpus_per_td: reader.read_sys_metadata_field(TDX_FIELD_TD_CONF_MAX_VCPUS_PER_TD)?
            as u16,
        ..TdxSysInfoTdConf::default()
    };

    let count = conf.num_cpuid_config as usize;
    if count > conf.cpuid_config_leaves.len() || count > conf.cpuid_config_values.len() {
        return Err(-EINVAL);
    }
    for i in 0..count {
        conf.cpuid_config_leaves[i] = reader
            .read_sys_metadata_field(TDX_FIELD_TD_CONF_CPUID_CONFIG_LEAVES_BASE + i as u64)?;
    }
    for i in 0..count {
        for j in 0..2 {
            conf.cpuid_config_values[i][j] = reader.read_sys_metadata_field(
                TDX_FIELD_TD_CONF_CPUID_CONFIG_VALUES_BASE + (i * 2 + j) as u64,
            )?;
        }
    }

    Ok(conf)
}

pub fn get_tdx_sys_info(reader: &mut impl TdxMetadataReader) -> Result<TdxSysInfo, i32> {
    let version = get_tdx_sys_info_version(reader)?;
    Ok(TdxSysInfo {
        version,
        features: get_tdx_sys_info_features(reader)?,
        tdmr: get_tdx_sys_info_tdmr(reader)?,
        td_ctrl: get_tdx_sys_info_td_ctrl(reader)?,
        td_conf: get_tdx_sys_info_td_conf(reader)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScriptedReader<'a> {
        entries: &'a [(u64, u64)],
        cursor: usize,
    }

    impl TdxMetadataReader for ScriptedReader<'_> {
        fn read_sys_metadata_field(&mut self, field: u64) -> Result<u64, i32> {
            let entry = self.entries.get(self.cursor).copied().ok_or(-EINVAL)?;
            self.cursor += 1;
            assert_eq!(entry.0, field);
            Ok(entry.1)
        }
    }

    #[test]
    fn tdx_global_metadata_sequence_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/virt/vmx/tdx/tdx_global_metadata.c"
        ));
        assert!(source.contains("static __init int get_tdx_sys_info_version"));
        assert!(source.contains("read_sys_metadata_field(0x0800000100000003, &val)"));
        assert!(source.contains("sysinfo_version->minor_version = val;"));
        assert!(source.contains("read_sys_metadata_field(0x0A00000300000008, &val)"));
        assert!(source.contains("read_sys_metadata_field(0x9100000100000012, &val)"));
        assert!(source.contains("read_sys_metadata_field(0x9800000100000200, &val)"));
        assert!(source.contains("read_sys_metadata_field(0x1900000300000003, &val)"));
        assert!(source.contains("if (sysinfo_td_conf->num_cpuid_config > ARRAY_SIZE(sysinfo_td_conf->cpuid_config_leaves))"));
        assert!(source.contains("read_sys_metadata_field(0x9900000300000400 + i, &val)"));
        assert!(source.contains("read_sys_metadata_field(0x9900000300000500 + i * 2 + j, &val)"));
        assert!(source.contains("pr_info(\"Module version: %u.%u.%02u\\n\""));
        assert!(source.contains("ret = ret ?: get_tdx_sys_info_td_conf(&sysinfo->td_conf);"));

        let entries = [
            (TDX_FIELD_VERSION_MINOR, 2),
            (TDX_FIELD_VERSION_MAJOR, 1),
            (TDX_FIELD_VERSION_UPDATE, 7),
            (TDX_FIELD_FEATURES0, 0xaa),
            (TDX_FIELD_TDMR_MAX_TDMRS, 8),
            (TDX_FIELD_TDMR_MAX_RESERVED_PER_TDMR, 16),
            (TDX_FIELD_TDMR_PAMT_4K_ENTRY_SIZE, 1),
            (TDX_FIELD_TDMR_PAMT_2M_ENTRY_SIZE, 2),
            (TDX_FIELD_TDMR_PAMT_1G_ENTRY_SIZE, 3),
            (TDX_FIELD_TD_CTRL_TDR_BASE_SIZE, 4),
            (TDX_FIELD_TD_CTRL_TDCS_BASE_SIZE, 5),
            (TDX_FIELD_TD_CTRL_TDVPS_BASE_SIZE, 6),
            (TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED0, 0x10),
            (TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED1, 0x11),
            (TDX_FIELD_TD_CONF_XFAM_FIXED0, 0x12),
            (TDX_FIELD_TD_CONF_XFAM_FIXED1, 0x13),
            (TDX_FIELD_TD_CONF_NUM_CPUID_CONFIG, 1),
            (TDX_FIELD_TD_CONF_MAX_VCPUS_PER_TD, 64),
            (TDX_FIELD_TD_CONF_CPUID_CONFIG_LEAVES_BASE, 0x8000_0000),
            (TDX_FIELD_TD_CONF_CPUID_CONFIG_VALUES_BASE, 0x1),
            (TDX_FIELD_TD_CONF_CPUID_CONFIG_VALUES_BASE + 1, 0x2),
        ];
        let mut reader = ScriptedReader {
            entries: &entries,
            cursor: 0,
        };
        let sysinfo = get_tdx_sys_info(&mut reader).unwrap();
        assert_eq!(sysinfo.version.major_version, 1);
        assert_eq!(sysinfo.version.minor_version, 2);
        assert_eq!(sysinfo.version.update_version, 7);
        assert_eq!(sysinfo.features.tdx_features0, 0xaa);
        assert_eq!(sysinfo.tdmr.max_tdmrs, 8);
        assert_eq!(sysinfo.td_ctrl.tdvps_base_size, 6);
        assert_eq!(sysinfo.td_conf.num_cpuid_config, 1);
        assert_eq!(sysinfo.td_conf.cpuid_config_leaves[0], 0x8000_0000);
        assert_eq!(sysinfo.td_conf.cpuid_config_values[0], [1, 2]);
    }

    #[test]
    fn tdx_cpuid_config_count_is_bounded_by_linux_arrays() {
        let entries = [
            (TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED0, 0),
            (TDX_FIELD_TD_CONF_ATTRIBUTES_FIXED1, 0),
            (TDX_FIELD_TD_CONF_XFAM_FIXED0, 0),
            (TDX_FIELD_TD_CONF_XFAM_FIXED1, 0),
            (TDX_FIELD_TD_CONF_NUM_CPUID_CONFIG, 129),
            (TDX_FIELD_TD_CONF_MAX_VCPUS_PER_TD, 1),
        ];
        let mut reader = ScriptedReader {
            entries: &entries,
            cursor: 0,
        };
        assert_eq!(get_tdx_sys_info_td_conf(&mut reader), Err(-EINVAL));
    }
}
