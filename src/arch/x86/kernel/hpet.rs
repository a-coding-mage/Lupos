//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/hpet.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/hpet.c
//! High Precision Event Timer register helpers.
//!
//! The boot path still uses LAPIC timer/jiffies for scheduling, but HPET
//! parsing belongs to Phase 1 because Linux x86 wires it beside TSC evidence.
//!
//! Reference: `vendor/linux/arch/x86/kernel/hpet.c`

pub const HPET_ID: u64 = 0x000;
pub const HPET_PERIOD: u64 = 0x004;
pub const HPET_CFG: u64 = 0x010;
pub const HPET_COUNTER: u64 = 0x0f0;
pub const HPET_T0_CFG: u64 = 0x100;
pub const HPET_T0_CMP: u64 = 0x108;

pub const HPET_CFG_ENABLE: u64 = 1 << 0;
pub const HPET_CFG_LEGACY: u64 = 1 << 1;
pub const HPET_TN_ENABLE: u64 = 1 << 2;
pub const HPET_TN_PERIODIC: u64 = 1 << 3;
pub const HPET_TN_64BIT: u64 = 1 << 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HpetCapabilities {
    pub revision: u8,
    pub timers: u8,
    pub counter_is_64bit: bool,
    pub legacy_replacement: bool,
    pub period_fs: u32,
}

impl HpetCapabilities {
    pub const fn from_id_period(id: u64, period: u32) -> Self {
        Self {
            revision: (id & 0xff) as u8,
            timers: (((id >> 8) & 0x1f) as u8) + 1,
            counter_is_64bit: (id & (1 << 13)) != 0,
            legacy_replacement: (id & (1 << 15)) != 0,
            period_fs: period,
        }
    }

    pub const fn frequency_hz(self) -> u64 {
        if self.period_fs == 0 {
            0
        } else {
            1_000_000_000_000_000u64 / self.period_fs as u64
        }
    }
}

pub const fn comparator_delta(period_fs: u32, ns: u64) -> u64 {
    if period_fs == 0 {
        0
    } else {
        ns.saturating_mul(1_000_000) / period_fs as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hpet_capability_bits_match_linux_layout() {
        let caps = HpetCapabilities::from_id_period((1 << 13) | (1 << 15) | (2 << 8) | 1, 100);
        assert_eq!(caps.revision, 1);
        assert_eq!(caps.timers, 3);
        assert!(caps.counter_is_64bit);
        assert!(caps.legacy_replacement);
        assert_eq!(caps.frequency_hz(), 10_000_000_000_000);
    }

    #[test]
    fn hpet_comparator_delta_uses_femtosecond_period() {
        assert_eq!(comparator_delta(100_000_000, 1_000_000), 10_000);
    }
}
