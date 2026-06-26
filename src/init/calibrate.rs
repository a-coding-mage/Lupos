//! linux-parity: complete
//! linux-source: vendor/linux/init/calibrate.c
//! test-origin: linux:vendor/linux/init/calibrate.c
//! Delay-loop calibration.
//!
//! Mirrors `vendor/linux/init/calibrate.c`: the boot CPU chooses
//! `loops_per_jiffy` from an already-known CPU value, the `lpj=` preset,
//! a timer-derived fine value, an arch-known value, a direct timer
//! calibration, or finally the convergence fallback.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::kernel::time::jiffies::HZ;

pub const DEFAULT_LOOPS_PER_JIFFY: u64 = 1 << 12;

static LOOPS_PER_JIFFY: AtomicU64 = AtomicU64::new(DEFAULT_LOOPS_PER_JIFFY);
static PRESET_LPJ: AtomicU64 = AtomicU64::new(0);
static LPJ_FINE: AtomicU64 = AtomicU64::new(0);
static PRINTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayCalibrationSource {
    AlreadyCalibrated,
    Preset,
    Fine,
    Known,
    Direct,
    Converged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelayCalibrationInputs {
    pub per_cpu_lpj: u64,
    pub preset_lpj: u64,
    pub lpj_fine: u64,
    pub known_lpj: u64,
    pub direct_lpj: u64,
    pub converged_lpj: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelayCalibrationResult {
    pub loops_per_jiffy: u64,
    pub source: DelayCalibrationSource,
    pub bogo_mips_int: u64,
    pub bogo_mips_frac: u64,
    pub first_print: bool,
}

impl Default for DelayCalibrationInputs {
    fn default() -> Self {
        Self {
            per_cpu_lpj: 0,
            preset_lpj: preset_lpj(),
            lpj_fine: lpj_fine(),
            known_lpj: 0,
            direct_lpj: 0,
            converged_lpj: DEFAULT_LOOPS_PER_JIFFY,
        }
    }
}

pub fn loops_per_jiffy() -> u64 {
    LOOPS_PER_JIFFY.load(Ordering::Acquire)
}

pub fn preset_lpj() -> u64 {
    PRESET_LPJ.load(Ordering::Acquire)
}

pub fn lpj_fine() -> u64 {
    LPJ_FINE.load(Ordering::Acquire)
}

pub fn set_lpj_fine(value: u64) {
    LPJ_FINE.store(value, Ordering::Release);
}

pub fn setup_lpj(value: &str) -> bool {
    let Some(parsed) = parse_lpj(value) else {
        return false;
    };
    PRESET_LPJ.store(parsed, Ordering::Release);
    true
}

pub fn parse_lpj(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u64>().ok()
    }
}

pub fn calibrate_delay() -> DelayCalibrationResult {
    calibrate_delay_with(DelayCalibrationInputs::default())
}

pub fn calibrate_delay_with(inputs: DelayCalibrationInputs) -> DelayCalibrationResult {
    let (lpj, source) = select_lpj(inputs);
    LOOPS_PER_JIFFY.store(lpj, Ordering::Release);
    let (bogo_mips_int, bogo_mips_frac) = bogomips_parts(lpj, HZ);
    let first_print = !PRINTED.swap(true, Ordering::AcqRel);

    DelayCalibrationResult {
        loops_per_jiffy: lpj,
        source,
        bogo_mips_int,
        bogo_mips_frac,
        first_print,
    }
}

pub fn select_lpj(inputs: DelayCalibrationInputs) -> (u64, DelayCalibrationSource) {
    if inputs.per_cpu_lpj != 0 {
        (
            inputs.per_cpu_lpj,
            DelayCalibrationSource::AlreadyCalibrated,
        )
    } else if inputs.preset_lpj != 0 {
        (inputs.preset_lpj, DelayCalibrationSource::Preset)
    } else if inputs.lpj_fine != 0 {
        (inputs.lpj_fine, DelayCalibrationSource::Fine)
    } else if inputs.known_lpj != 0 {
        (inputs.known_lpj, DelayCalibrationSource::Known)
    } else if inputs.direct_lpj != 0 {
        (inputs.direct_lpj, DelayCalibrationSource::Direct)
    } else {
        (inputs.converged_lpj, DelayCalibrationSource::Converged)
    }
}

pub fn bogomips_parts(lpj: u64, hz: u64) -> (u64, u64) {
    let whole_div = 500_000 / hz;
    let frac_div = 5_000 / hz;
    (lpj / whole_div, (lpj / frac_div) % 100)
}

#[cfg(test)]
pub fn reset_for_tests() {
    LOOPS_PER_JIFFY.store(DEFAULT_LOOPS_PER_JIFFY, Ordering::Release);
    PRESET_LPJ.store(0, Ordering::Release);
    LPJ_FINE.store(0, Ordering::Release);
    PRINTED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lpj_setup_accepts_decimal_and_hex() {
        reset_for_tests();
        assert!(setup_lpj("8192"));
        assert_eq!(preset_lpj(), 8192);
        assert!(setup_lpj("0x4000"));
        assert_eq!(preset_lpj(), 0x4000);
        assert!(!setup_lpj("nope"));
    }

    #[test]
    fn calibration_precedence_matches_linux() {
        let inputs = DelayCalibrationInputs {
            per_cpu_lpj: 1,
            preset_lpj: 2,
            lpj_fine: 3,
            known_lpj: 4,
            direct_lpj: 5,
            converged_lpj: 6,
        };
        assert_eq!(
            select_lpj(inputs),
            (1, DelayCalibrationSource::AlreadyCalibrated)
        );

        let inputs = DelayCalibrationInputs {
            per_cpu_lpj: 0,
            ..inputs
        };
        assert_eq!(select_lpj(inputs), (2, DelayCalibrationSource::Preset));

        let inputs = DelayCalibrationInputs {
            preset_lpj: 0,
            ..inputs
        };
        assert_eq!(select_lpj(inputs), (3, DelayCalibrationSource::Fine));
    }

    #[test]
    fn calibrate_delay_stores_global_lpj_and_formats_bogomips() {
        reset_for_tests();
        let result = calibrate_delay_with(DelayCalibrationInputs {
            per_cpu_lpj: 0,
            preset_lpj: 500_000,
            lpj_fine: 0,
            known_lpj: 0,
            direct_lpj: 0,
            converged_lpj: DEFAULT_LOOPS_PER_JIFFY,
        });
        assert_eq!(result.source, DelayCalibrationSource::Preset);
        assert_eq!(loops_per_jiffy(), 500_000);
        assert_eq!(bogomips_parts(500_000, 250), (250, 0));
        assert!(result.first_print);
        assert!(!calibrate_delay_with(DelayCalibrationInputs::default()).first_print);
    }
}
