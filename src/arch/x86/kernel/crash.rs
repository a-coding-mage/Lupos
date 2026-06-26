//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/crash.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/crash.c
//! x86 crash-kernel memory range helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/crash.c

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

pub const KEXEC_UPDATE_ELFCOREHDR: u32 = 0x0000_0004;
pub const KEXEC_CRASH_HOTPLUG_SUPPORT: u32 = 0x0000_0008;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrashRange {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrashMem {
    pub ranges: Vec<CrashRange>,
    pub max_nr_ranges: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct E820Entry {
    pub addr: u64,
    pub size: u64,
    pub typ: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootParamsMemmap {
    pub entries: Vec<E820Entry>,
    pub max_entries: usize,
}

pub const fn resource_size(range: CrashRange) -> u64 {
    if range.end < range.start {
        0
    } else {
        range.end - range.start + 1
    }
}

pub fn exclude_mem_range(mem: &mut CrashMem, start: u64, end: u64) -> Result<(), i32> {
    let mut out = Vec::new();
    for range in &mem.ranges {
        if end < range.start || start > range.end {
            out.push(*range);
            continue;
        }
        if start > range.start {
            out.push(CrashRange {
                start: range.start,
                end: start - 1,
            });
        }
        if end < range.end {
            out.push(CrashRange {
                start: end + 1,
                end: range.end,
            });
        }
    }
    if out.len() > mem.max_nr_ranges {
        return Err(ENOMEM);
    }
    mem.ranges = out;
    Ok(())
}

pub fn elf_header_exclude_ranges(
    mem: &mut CrashMem,
    crash_kernel: Option<CrashRange>,
    crash_kernel_low: Option<CrashRange>,
    cma_ranges: &[CrashRange],
) -> Result<(), i32> {
    exclude_mem_range(mem, 0, 0x0f_ffff)?;
    if let Some(range) = crash_kernel {
        exclude_mem_range(mem, range.start, range.end)?;
    }
    if let Some(range) = crash_kernel_low {
        exclude_mem_range(mem, range.start, range.end)?;
    }
    for range in cma_ranges {
        exclude_mem_range(mem, range.start, range.end)?;
    }
    Ok(())
}

pub fn add_e820_entry(memmap: &mut BootParamsMemmap, entry: E820Entry) -> Result<(), i32> {
    if memmap.entries.len() >= memmap.max_entries {
        return Err(ENOMEM);
    }
    memmap.entries.push(entry);
    Ok(())
}

pub const fn arch_crash_hotplug_support(kexec_flags: u32) -> bool {
    (kexec_flags & (KEXEC_UPDATE_ELFCOREHDR | KEXEC_CRASH_HOTPLUG_SUPPORT)) != 0
}

pub fn crash_smp_send_stop(stopped: &mut bool) -> bool {
    let was_stopped = *stopped;
    *stopped = true;
    !was_stopped
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn exclude_mem_range_splits_inclusive_ranges() {
        let mut mem = CrashMem {
            ranges: vec![CrashRange {
                start: 0x1000,
                end: 0x4fff,
            }],
            max_nr_ranges: 4,
        };
        exclude_mem_range(&mut mem, 0x2000, 0x2fff).unwrap();
        assert_eq!(
            mem.ranges,
            vec![
                CrashRange {
                    start: 0x1000,
                    end: 0x1fff
                },
                CrashRange {
                    start: 0x3000,
                    end: 0x4fff
                }
            ]
        );
    }

    #[test]
    fn exclude_mem_range_reports_capacity_overflow() {
        let mut mem = CrashMem {
            ranges: vec![CrashRange {
                start: 0x1000,
                end: 0x4fff,
            }],
            max_nr_ranges: 1,
        };
        assert_eq!(exclude_mem_range(&mut mem, 0x2000, 0x2fff), Err(ENOMEM));
    }

    #[test]
    fn elf_header_exclusions_drop_low_meg_and_crash_windows() {
        let mut mem = CrashMem {
            ranges: vec![CrashRange {
                start: 0,
                end: 0x3f_ffff,
            }],
            max_nr_ranges: 8,
        };
        elf_header_exclude_ranges(
            &mut mem,
            Some(CrashRange {
                start: 0x20_0000,
                end: 0x2f_ffff,
            }),
            None,
            &[],
        )
        .unwrap();
        assert_eq!(
            mem.ranges,
            vec![
                CrashRange {
                    start: 0x10_0000,
                    end: 0x1f_ffff
                },
                CrashRange {
                    start: 0x30_0000,
                    end: 0x3f_ffff
                }
            ]
        );
    }

    #[test]
    fn e820_entry_append_respects_capacity() {
        let mut map = BootParamsMemmap {
            entries: Vec::new(),
            max_entries: 1,
        };
        assert_eq!(
            add_e820_entry(
                &mut map,
                E820Entry {
                    addr: 0,
                    size: 4096,
                    typ: 1
                }
            ),
            Ok(())
        );
        assert_eq!(
            add_e820_entry(
                &mut map,
                E820Entry {
                    addr: 4096,
                    size: 4096,
                    typ: 2
                }
            ),
            Err(ENOMEM)
        );
    }

    #[test]
    fn crash_hotplug_flags_match_linux_uapi() {
        assert!(arch_crash_hotplug_support(KEXEC_UPDATE_ELFCOREHDR));
        assert!(arch_crash_hotplug_support(KEXEC_CRASH_HOTPLUG_SUPPORT));
        assert!(!arch_crash_hotplug_support(0));
    }
}
