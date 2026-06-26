//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/events/zhaoxin
//! test-origin: linux:vendor/linux/arch/x86/events/zhaoxin
//! Zhaoxin x86 PMU model.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZhaoxinPmuFile {
    pub rust_module: Option<&'static str>,
    pub linux_source: &'static str,
    pub required_markers: &'static [&'static str],
}

pub mod core;

pub const ZHAOXIN_PMU_SOURCES: &[ZhaoxinPmuFile] = &[ZhaoxinPmuFile {
    rust_module: Some("core"),
    linux_source: "vendor/linux/arch/x86/events/zhaoxin/core.c",
    required_markers: &[
        "Zhaoxin PMU; like Intel Architectural PerfMon-v2",
        "static u64 zx_pmon_event_map[PERF_COUNT_HW_MAX]",
        "FIXED_EVENT_CONSTRAINT(0x0082, 1)",
        "FIXED_EVENT_CONSTRAINT(0x00c0, 0)",
        "zxd_hw_cache_event_ids",
        "zxe_hw_cache_event_ids",
        "zhaoxin_pmu_handle_irq",
        "apic_write(APIC_LVTPC, APIC_DM_NMI);",
        "static const struct x86_pmu zhaoxin_pmu",
        ".name\t\t\t= \"zhaoxin\"",
        "__init int zhaoxin_pmu_init(void)",
    ],
}];

pub const ZHAOXIN_PMU_BUILD_FILES: &[ZhaoxinPmuFile] = &[ZhaoxinPmuFile {
    rust_module: None,
    linux_source: "vendor/linux/arch/x86/events/zhaoxin/Makefile",
    required_markers: &["obj-y\t+= core.o"],
}];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_inventory_matches_linux_directory() {
        let rust = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/arch/x86/events/zhaoxin/core.rs"
        ));
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/zhaoxin/core.c"
        ));
        let entry = ZHAOXIN_PMU_SOURCES[0];

        assert_eq!(entry.rust_module, Some("core"));
        assert_eq!(
            entry.linux_source,
            "vendor/linux/arch/x86/events/zhaoxin/core.c"
        );
        assert!(rust.contains("//! linux-parity: complete"));
        assert!(rust.contains(concat!(
            "//! linux-source: ",
            "vendor/linux/arch/x86/events/zhaoxin/core.c"
        )));
        assert!(source.contains("SPDX-License-Identifier: GPL-2.0-only"));
        for marker in entry.required_markers {
            assert!(source.contains(marker), "missing {}", marker);
        }
        assert_eq!(ZHAOXIN_PMU_SOURCES.len(), 1);
    }

    #[test]
    fn aggregate_build_file_selects_core_object() {
        let makefile = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/events/zhaoxin/Makefile"
        ));
        let entry = ZHAOXIN_PMU_BUILD_FILES[0];

        assert_eq!(entry.rust_module, None);
        assert_eq!(
            entry.linux_source,
            "vendor/linux/arch/x86/events/zhaoxin/Makefile"
        );
        assert!(makefile.contains("SPDX-License-Identifier: GPL-2.0"));
        for marker in entry.required_markers {
            assert!(makefile.contains(marker), "missing {}", marker);
        }
        assert_eq!(ZHAOXIN_PMU_BUILD_FILES.len(), 1);
    }

    #[test]
    fn aggregate_exposes_core_contract() {
        assert_eq!(core::ZHAOXIN_PMU_DESCRIPTOR.name, "zhaoxin");
        assert_eq!(core::ZHAOXIN_PMU_DESCRIPTOR.max_period, (1u64 << 47) - 1);
        assert_eq!(
            core::ZX_PMON_EVENT_MAP[core::PERF_COUNT_HW_CPU_CYCLES],
            0x0082
        );
        assert_eq!(
            core::ZX_PMON_EVENT_MAP[core::PERF_COUNT_HW_INSTRUCTIONS],
            0x00c0
        );
        assert_eq!(
            core::ZXC_EVENT_CONSTRAINTS[0],
            core::fixed_event_constraint(0x0082, 1)
        );
        assert_eq!(
            core::ZXD_EVENT_CONSTRAINTS[2],
            core::fixed_event_constraint(0x0083, 2)
        );
    }
}
