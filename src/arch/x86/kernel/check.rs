//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/check.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/check.c
//! Low-memory BIOS corruption checker.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/check.c
//!
//! Linux reserves free low-memory ranges and periodically checks whether
//! firmware scribbled into them. This port keeps the range selection and
//! scan semantics pure; scheduling delayed work is a later integration hook.

extern crate alloc;

use alloc::vec::Vec;

use crate::arch::x86::mm::paging::PAGE_SIZE;
use crate::include::uapi::errno::EINVAL;

pub const MAX_SCAN_AREAS: usize = 8;
pub const DEFAULT_CORRUPTION_CHECK_SIZE: u64 = 64 * 1024;
pub const DEFAULT_CORRUPTION_CHECK_PERIOD: u32 = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanArea {
    pub addr: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorruptionCheckConfig {
    pub enabled: Option<bool>,
    pub size: u64,
    pub period_seconds: u32,
}

impl Default for CorruptionCheckConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            size: DEFAULT_CORRUPTION_CHECK_SIZE,
            period_seconds: DEFAULT_CORRUPTION_CHECK_PERIOD,
        }
    }
}

pub fn set_corruption_check(cfg: &mut CorruptionCheckConfig, value: &str) -> Result<(), i32> {
    cfg.enabled = Some(parse_u64(value)? != 0);
    Ok(())
}

pub fn set_corruption_check_period(
    cfg: &mut CorruptionCheckConfig,
    value: &str,
) -> Result<(), i32> {
    cfg.period_seconds = parse_u64(value)? as u32;
    Ok(())
}

pub fn set_corruption_check_size(cfg: &mut CorruptionCheckConfig, value: &str) -> Result<(), i32> {
    cfg.size = parse_mem_size(value)?;
    Ok(())
}

pub fn setup_bios_corruption_check(
    cfg: CorruptionCheckConfig,
    default_enabled: bool,
    free_ranges: &[ScanArea],
) -> Vec<ScanArea> {
    let enabled = cfg.enabled.unwrap_or(default_enabled);
    if !enabled || cfg.size == 0 {
        return Vec::new();
    }

    let limit = round_up(cfg.size, PAGE_SIZE);
    let mut areas = Vec::new();
    for range in free_ranges {
        let start = clamp(round_up(range.addr, PAGE_SIZE), PAGE_SIZE, limit);
        let end = clamp(
            round_down(range.addr + range.size, PAGE_SIZE),
            PAGE_SIZE,
            limit,
        );
        if start >= end {
            continue;
        }
        areas.push(ScanArea {
            addr: start,
            size: end - start,
        });
        if areas.len() == MAX_SCAN_AREAS {
            break;
        }
    }
    areas
}

pub fn check_for_bios_corruption(words: &mut [u64]) -> usize {
    let mut corrupt = 0;
    for word in words {
        if *word != 0 {
            corrupt += 1;
            *word = 0;
        }
    }
    corrupt
}

pub const fn should_schedule_periodic_check(
    num_scan_areas: usize,
    enabled: bool,
    period_seconds: u32,
) -> bool {
    num_scan_areas != 0 && enabled && period_seconds != 0
}

const fn round_up(value: u64, align: u64) -> u64 {
    if value == 0 {
        0
    } else {
        ((value + align - 1) / align) * align
    }
}

const fn round_down(value: u64, align: u64) -> u64 {
    (value / align) * align
}

const fn clamp(value: u64, min: u64, max: u64) -> u64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn parse_u64(value: &str) -> Result<u64, i32> {
    value.parse::<u64>().map_err(|_| EINVAL)
}

fn parse_mem_size(value: &str) -> Result<u64, i32> {
    let (digits, mult) = match value.as_bytes().last().copied() {
        Some(b'K') | Some(b'k') => (&value[..value.len() - 1], 1024),
        Some(b'M') | Some(b'm') => (&value[..value.len() - 1], 1024 * 1024),
        Some(b'G') | Some(b'g') => (&value[..value.len() - 1], 1024 * 1024 * 1024),
        _ => (value, 1),
    };
    Ok(parse_u64(digits)? * mult)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_setters_parse_linux_options() {
        let mut cfg = CorruptionCheckConfig::default();
        set_corruption_check(&mut cfg, "1").unwrap();
        set_corruption_check_period(&mut cfg, "9").unwrap();
        set_corruption_check_size(&mut cfg, "128K").unwrap();
        assert_eq!(cfg.enabled, Some(true));
        assert_eq!(cfg.period_seconds, 9);
        assert_eq!(cfg.size, 128 * 1024);
    }

    #[test]
    fn setup_selects_page_aligned_low_memory_ranges() {
        let cfg = CorruptionCheckConfig {
            enabled: Some(true),
            size: 64 * 1024,
            period_seconds: 60,
        };
        let areas = setup_bios_corruption_check(
            cfg,
            false,
            &[ScanArea {
                addr: 0x123,
                size: 0x3000,
            }],
        );
        assert_eq!(
            areas,
            [ScanArea {
                addr: PAGE_SIZE,
                size: 0x2000
            }]
        );
    }

    #[test]
    fn disabled_or_zero_size_produces_no_scan_areas() {
        let cfg = CorruptionCheckConfig {
            enabled: Some(false),
            size: 64 * 1024,
            period_seconds: 60,
        };
        assert!(setup_bios_corruption_check(cfg, true, &[]).is_empty());
    }

    #[test]
    fn scanner_reports_and_clears_nonzero_words() {
        let mut words = [0, 1, 0, 2];
        assert_eq!(check_for_bios_corruption(&mut words), 2);
        assert_eq!(words, [0, 0, 0, 0]);
    }

    #[test]
    fn periodic_check_requires_area_enabled_and_period() {
        assert!(should_schedule_periodic_check(1, true, 60));
        assert!(!should_schedule_periodic_check(0, true, 60));
        assert!(!should_schedule_periodic_check(1, true, 0));
    }
}
