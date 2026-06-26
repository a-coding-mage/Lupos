//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/srat.c
//! test-origin: linux:vendor/linux/arch/x86/mm/srat.c
//! ACPI SRAT NUMA extraction policy.
//!
//! Mirrors the x86 ACPI NUMA initialization gate from
//! `vendor/linux/arch/x86/mm/srat.c`. Without a usable SRAT, Lupos falls back
//! to the single-node policy in `numa`.

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SratMemoryAffinity {
    pub base: u64,
    pub length: u64,
    pub proximity_domain: u32,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcpiNumaInit {
    pub nodes: u8,
    pub memory_ranges: u8,
}

pub const fn srat_range_valid(entry: SratMemoryAffinity) -> bool {
    entry.enabled && entry.length != 0 && entry.base.checked_add(entry.length).is_some()
}

pub const fn x86_acpi_numa_init(entries: &[SratMemoryAffinity]) -> Result<AcpiNumaInit, i32> {
    if entries.is_empty() {
        return Err(ENODEV);
    }
    let mut ranges = 0u8;
    let mut max_domain = 0u32;
    let mut i = 0;
    while i < entries.len() {
        let entry = entries[i];
        if srat_range_valid(entry) {
            ranges = match ranges.checked_add(1) {
                Some(v) => v,
                None => return Err(EINVAL),
            };
            if entry.proximity_domain > max_domain {
                max_domain = entry.proximity_domain;
            }
        }
        i += 1;
    }
    if ranges == 0 {
        return Err(ENODEV);
    }
    if max_domain >= 255 {
        return Err(EINVAL);
    }
    Ok(AcpiNumaInit {
        nodes: (max_domain as u8) + 1,
        memory_ranges: ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_or_empty_srat_returns_enodev() {
        assert_eq!(x86_acpi_numa_init(&[]), Err(ENODEV));
        assert_eq!(
            x86_acpi_numa_init(&[SratMemoryAffinity {
                base: 0,
                length: 0,
                proximity_domain: 0,
                enabled: true
            }]),
            Err(ENODEV)
        );
    }

    #[test]
    fn valid_srat_counts_ranges_and_nodes() {
        assert_eq!(
            x86_acpi_numa_init(&[SratMemoryAffinity {
                base: 0,
                length: 0x1000,
                proximity_domain: 1,
                enabled: true
            }]),
            Ok(AcpiNumaInit {
                nodes: 2,
                memory_ranges: 1
            })
        );
    }
}
