//! linux-parity: complete
//! linux-source: vendor/linux/kernel/crash_core_test.c
//! test-origin: linux:vendor/linux/kernel/crash_core_test.c
//! KUnit coverage for crash_exclude_mem_range().

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrashMem {
    pub max_nr_ranges: usize,
    pub ranges: Vec<Range>,
}

impl CrashMem {
    pub fn new(max_nr_ranges: usize, ranges: &[Range]) -> Option<Self> {
        (max_nr_ranges >= ranges.len()).then(|| Self {
            max_nr_ranges,
            ranges: ranges.into(),
        })
    }
}

pub fn crash_exclude_mem_range(
    mem: &mut CrashMem,
    exclude_start: u64,
    exclude_end: u64,
) -> Result<(), i32> {
    let original = mem.ranges.clone();
    let mut next = Vec::new();

    for range in &mem.ranges {
        if exclude_end < range.start || exclude_start > range.end {
            next.push(*range);
            continue;
        }
        if exclude_start > range.start {
            next.push(Range {
                start: range.start,
                end: exclude_start - 1,
            });
        }
        if exclude_end < range.end {
            next.push(Range {
                start: exclude_end + 1,
                end: range.end,
            });
        }
    }

    if next.len() > mem.max_nr_ranges {
        mem.ranges = original;
        return Err(-ENOMEM);
    }
    mem.ranges = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_core_test_matches_linux_original_kunit_cases() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/crash_core_test.c"
        ));

        assert!(source.contains("static const struct range single_range_b"));
        assert!(source.contains("exclude_single_range_test_data[]"));
        assert!(source.contains("exclude_range_regression_test_data[]"));
        assert!(source.contains("KUNIT_CASE(exclude_single_range_test)"));
        assert!(source.contains("KUNIT_CASE(exclude_range_regression_test)"));
        assert!(source.contains(".name = \"crash_exclude_mem_range_tests\""));
        assert!(source.contains("MODULE_DESCRIPTION(\"crash dump KUnit test suite\")"));

        let single = [Range {
            start: 100,
            end: 199,
        }];
        let mut mem = CrashMem::new(1, &single).unwrap();
        assert_eq!(crash_exclude_mem_range(&mut mem, 10, 50), Ok(()));
        assert_eq!(mem.ranges, single);

        let mut mem = CrashMem::new(1, &single).unwrap();
        assert_eq!(crash_exclude_mem_range(&mut mem, 50, 149), Ok(()));
        assert_eq!(
            mem.ranges,
            [Range {
                start: 150,
                end: 199
            }]
        );

        let mut mem = CrashMem::new(2, &single).unwrap();
        assert_eq!(crash_exclude_mem_range(&mut mem, 120, 179), Ok(()));
        assert_eq!(
            mem.ranges,
            [
                Range {
                    start: 100,
                    end: 119
                },
                Range {
                    start: 180,
                    end: 199
                },
            ]
        );

        let mut mem = CrashMem::new(1, &single).unwrap();
        assert_eq!(crash_exclude_mem_range(&mut mem, 120, 179), Err(-ENOMEM));
        assert_eq!(mem.ranges, single);
    }
}
