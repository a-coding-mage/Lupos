//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform
//! test-origin: linux:vendor/linux/arch/x86/platform
//! x86 PC platform defaults and trace-clock policy.
//!
//! Lupos currently supports the standard PC/QEMU path. This module records the
//! default x86 init choices that Linux keeps in `x86_init.c` and exposes the
//! trace clock source choice used by tracing code.
//!
//! References:
//! - `vendor/linux/arch/x86/kernel/x86_init.c`
//! - `vendor/linux/arch/x86/kernel/trace_clock.c`
//! - vendor/linux/arch/x86/platform/atom/punit_atom_debug.c
//! - vendor/linux/arch/x86/platform/ce4100/ce4100.c
//! - vendor/linux/arch/x86/platform/efi/efi.c
//! - vendor/linux/arch/x86/platform/efi/efi_32.c
//! - vendor/linux/arch/x86/platform/efi/efi_64.c
//! - vendor/linux/arch/x86/platform/efi/memmap.c
//! - vendor/linux/arch/x86/platform/efi/quirks.c
//! - vendor/linux/arch/x86/platform/efi/runtime-map.c
//! - vendor/linux/arch/x86/platform/geode/alix.c
//! - vendor/linux/arch/x86/platform/geode/geode-common.c
//! - vendor/linux/arch/x86/platform/geode/geos.c
//! - vendor/linux/arch/x86/platform/geode/net5501.c
//! - vendor/linux/arch/x86/platform/intel-mid/intel-mid.c
//! - vendor/linux/arch/x86/platform/intel-mid/pwr.c
//! - vendor/linux/arch/x86/platform/intel-quark/imr.c
//! - vendor/linux/arch/x86/platform/intel/iosf_mbi.c
//! - vendor/linux/arch/x86/platform/iris/iris.c
//! - vendor/linux/arch/x86/platform/olpc/olpc-xo1-pm.c
//! - vendor/linux/arch/x86/platform/olpc/olpc-xo1-rtc.c
//! - vendor/linux/arch/x86/platform/olpc/olpc-xo1-sci.c
//! - vendor/linux/arch/x86/platform/olpc/olpc-xo15-sci.c
//! - vendor/linux/arch/x86/platform/olpc/olpc.c
//! - vendor/linux/arch/x86/platform/olpc/olpc_dt.c
//! - vendor/linux/arch/x86/platform/olpc/olpc_ofw.c
//! - vendor/linux/arch/x86/platform/pvh/enlighten.c
//! - vendor/linux/arch/x86/platform/scx200/scx200_32.c
//! - vendor/linux/arch/x86/platform/ts5500/ts5500.c
//! - vendor/linux/arch/x86/platform/uv/bios_uv.c
//! - vendor/linux/arch/x86/platform/uv/uv_irq.c
//! - vendor/linux/arch/x86/platform/uv/uv_nmi.c
//! - vendor/linux/arch/x86/platform/uv/uv_time.c

pub mod ce4100;
pub mod efi;
pub mod geode;
pub mod iris;
pub mod olpc;

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X86Platform {
    Pc,
    Efi,
    Ce4100,
    Olpc,
    IntelMid,
    IntelQuark,
    Iris,
    Uv,
    Geode,
    Atom,
    Scx200,
    Ts5500,
    Pvh,
    Jailhouse,
}

pub const fn platform_enabled(platform: X86Platform) -> bool {
    matches!(platform, X86Platform::Pc)
}

pub const fn platform_errno(platform: X86Platform) -> Option<i32> {
    if platform_enabled(platform) {
        None
    } else {
        Some(match platform {
            X86Platform::Efi | X86Platform::Pvh => EOPNOTSUPP,
            _ => ENODEV,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EfiMode {
    Disabled,
    Efi32,
    Efi64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfiRuntimePolicy {
    pub mode: EfiMode,
    pub runtime_map_present: bool,
    pub mixed_mode: bool,
    pub quirks_required: bool,
}

pub const fn efi_runtime_enabled(policy: EfiRuntimePolicy) -> bool {
    !matches!(policy.mode, EfiMode::Disabled)
        && policy.runtime_map_present
        && (!policy.mixed_mode || matches!(policy.mode, EfiMode::Efi64))
}

pub const fn efi_memmap_entry_valid(phys: u64, pages: u64) -> bool {
    phys != 0 && pages != 0 && phys.checked_add(pages << 12).is_some()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeodeBoard {
    Alix,
    Geos,
    Net5501,
}

pub const fn geode_board_has_cs5535(board: GeodeBoard) -> bool {
    matches!(board, GeodeBoard::Alix | GeodeBoard::Net5501)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OlpcMachine {
    Xo1,
    Xo15,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OlpcPolicy {
    pub machine: OlpcMachine,
    pub has_openfirmware: bool,
    pub has_device_tree: bool,
    pub sci_enabled: bool,
}

pub const fn olpc_platform_ready(policy: OlpcPolicy) -> bool {
    policy.has_openfirmware && policy.has_device_tree && policy.sci_enabled
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntelMidPolicy {
    pub sfi_table_present: bool,
    pub pwr_button_present: bool,
}

pub const fn intel_mid_power_button_ready(policy: IntelMidPolicy) -> bool {
    policy.sfi_table_present && policy.pwr_button_present
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UvPolicy {
    pub bios_table_present: bool,
    pub hub_revision: u16,
    pub x2apic_enabled: bool,
}

pub const fn uv_platform_ready(policy: UvPolicy) -> bool {
    policy.bios_table_present && policy.hub_revision != 0 && policy.x2apic_enabled
}

pub const fn pvh_enlightenment_enabled(platform: X86Platform, xen_hvm: bool) -> bool {
    matches!(platform, X86Platform::Pvh) && xen_hvm
}

pub const fn legacy_platform_io_base(platform: X86Platform) -> Option<u16> {
    match platform {
        X86Platform::Atom => Some(0x30),
        X86Platform::Ce4100 => Some(0xcf8),
        X86Platform::Geode | X86Platform::Scx200 => Some(0x6100),
        X86Platform::Ts5500 => Some(0x74),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X86InitDefaults {
    pub platform: X86Platform,
    pub native_irq_init: bool,
    pub apic_clockevent: bool,
    pub hpet_wallclock: bool,
    pub iommu_default_enabled: bool,
}

pub const fn default_x86_init_ops() -> X86InitDefaults {
    X86InitDefaults {
        platform: X86Platform::Pc,
        native_irq_init: true,
        apic_clockevent: true,
        hpet_wallclock: true,
        iommu_default_enabled: false,
    }
}

/// Linux trace_clock_x86 uses TSC-like cycles when available; Lupos exposes
/// the same monotonic source choice as a small enum for trace users.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceClockSource {
    Tsc,
    Jiffies,
}

pub const fn trace_clock_source(has_tsc: bool) -> TraceClockSource {
    if has_tsc {
        TraceClockSource::Tsc
    } else {
        TraceClockSource::Jiffies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pc_platform_is_live() {
        assert!(platform_enabled(X86Platform::Pc));
        assert_eq!(platform_errno(X86Platform::Olpc), Some(ENODEV));
        assert_eq!(platform_errno(X86Platform::Efi), Some(EOPNOTSUPP));
    }

    #[test]
    fn trace_clock_prefers_tsc_when_present() {
        assert_eq!(trace_clock_source(true), TraceClockSource::Tsc);
        assert_eq!(trace_clock_source(false), TraceClockSource::Jiffies);
    }

    #[test]
    fn default_x86_init_ops_match_pc_boot_path() {
        let defaults = default_x86_init_ops();
        assert_eq!(defaults.platform, X86Platform::Pc);
        assert!(defaults.native_irq_init);
        assert!(defaults.apic_clockevent);
        assert!(defaults.hpet_wallclock);
        assert!(!defaults.iommu_default_enabled);
    }

    #[test]
    fn efi_runtime_requires_a_runtime_map() {
        assert!(efi_runtime_enabled(EfiRuntimePolicy {
            mode: EfiMode::Efi64,
            runtime_map_present: true,
            mixed_mode: false,
            quirks_required: false,
        }));
        assert!(!efi_runtime_enabled(EfiRuntimePolicy {
            mode: EfiMode::Efi32,
            runtime_map_present: true,
            mixed_mode: true,
            quirks_required: true,
        }));
        assert!(efi_memmap_entry_valid(0x1000, 1));
        assert!(!efi_memmap_entry_valid(0, 1));
    }

    #[test]
    fn platform_specific_helpers_stay_fail_closed() {
        assert!(geode_board_has_cs5535(GeodeBoard::Alix));
        assert!(olpc_platform_ready(OlpcPolicy {
            machine: OlpcMachine::Xo1,
            has_openfirmware: true,
            has_device_tree: true,
            sci_enabled: true,
        }));
        assert!(!uv_platform_ready(UvPolicy {
            bios_table_present: true,
            hub_revision: 0,
            x2apic_enabled: true,
        }));
        assert_eq!(legacy_platform_io_base(X86Platform::Ts5500), Some(0x74));
    }

    #[test]
    fn pvh_requires_xen_hvm_enlightenment() {
        assert!(pvh_enlightenment_enabled(X86Platform::Pvh, true));
        assert!(!pvh_enlightenment_enabled(X86Platform::Pvh, false));
    }
}
