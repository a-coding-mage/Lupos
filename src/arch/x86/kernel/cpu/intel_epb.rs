//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/intel_epb.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/intel_epb.c
//! Intel Energy Performance Bias (EPB) policy decoder.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/intel_epb.c

// The MSR_IA32_ENERGY_PERF_BIAS (0x1b0) holds a 4-bit value 0..=15 that
// hints at the desired energy/performance tradeoff. Linux exports five
// named presets through sysfs. We model the preset table.

pub const MSR_IA32_ENERGY_PERF_BIAS: u32 = 0x1b0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpbPreset {
    Performance,
    BalancedPerformance,
    Normal,
    BalancedPowersave,
    Powersave,
}

pub const fn preset_value(preset: EpbPreset) -> u8 {
    match preset {
        EpbPreset::Performance => 0,
        EpbPreset::BalancedPerformance => 4,
        EpbPreset::Normal => 6,
        EpbPreset::BalancedPowersave => 8,
        EpbPreset::Powersave => 15,
    }
}

pub const fn preset_from_value(value: u8) -> EpbPreset {
    match value & 0x0f {
        0..=3 => EpbPreset::Performance,
        4..=5 => EpbPreset::BalancedPerformance,
        6..=7 => EpbPreset::Normal,
        8..=12 => EpbPreset::BalancedPowersave,
        _ => EpbPreset::Powersave,
    }
}

pub const fn preset_label(preset: EpbPreset) -> &'static str {
    match preset {
        EpbPreset::Performance => "performance",
        EpbPreset::BalancedPerformance => "balance-performance",
        EpbPreset::Normal => "normal",
        EpbPreset::BalancedPowersave => "balance-power",
        EpbPreset::Powersave => "power",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_round_trip_for_canonical_values() {
        assert_eq!(preset_from_value(0), EpbPreset::Performance);
        assert_eq!(preset_from_value(6), EpbPreset::Normal);
        assert_eq!(preset_from_value(15), EpbPreset::Powersave);
        assert_eq!(preset_value(EpbPreset::Normal), 6);
    }

    #[test]
    fn labels_match_linux_sysfs_strings() {
        assert_eq!(preset_label(EpbPreset::Performance), "performance");
        assert_eq!(preset_label(EpbPreset::Powersave), "power");
    }
}
