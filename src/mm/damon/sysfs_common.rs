//! linux-parity: complete
//! linux-source: vendor/linux/mm/damon/sysfs-common.c
//! test-origin: linux:vendor/linux/mm/damon/sysfs-common.c
//! Common DAMON sysfs range helpers.

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamonSysfsUlRange {
    pub min: u64,
    pub max: u64,
}

pub const DAMON_SYSFS_UL_RANGE_ATTR_MODE: u16 = 0o600;

pub const fn damon_sysfs_ul_range_alloc(min: u64, max: u64) -> DamonSysfsUlRange {
    DamonSysfsUlRange { min, max }
}

pub fn min_show(range: DamonSysfsUlRange) -> u64 {
    range.min
}

pub fn max_show(range: DamonSysfsUlRange) -> u64 {
    range.max
}

pub fn min_store(range: &mut DamonSysfsUlRange, value: &str) -> Result<usize, i32> {
    let trimmed = value.trim();
    let min = trimmed.parse::<u64>().map_err(|_| EINVAL)?;
    range.min = min;
    Ok(value.len())
}

pub fn max_store(range: &mut DamonSysfsUlRange, value: &str) -> Result<usize, i32> {
    let trimmed = value.trim();
    let max = trimmed.parse::<u64>().map_err(|_| EINVAL)?;
    range.max = max;
    Ok(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damon_sysfs_range_attrs_match_linux_common_helpers() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/damon/sysfs-common.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/damon/sysfs-common.h"
        ));
        assert!(source.contains("DEFINE_MUTEX(damon_sysfs_lock);"));
        assert!(source.contains("struct damon_sysfs_ul_range *damon_sysfs_ul_range_alloc"));
        assert!(source.contains("range->min = min;"));
        assert!(source.contains("range->max = max;"));
        assert!(source.contains("return sysfs_emit(buf, \"%lu\\n\", range->min);"));
        assert!(source.contains("return sysfs_emit(buf, \"%lu\\n\", range->max);"));
        assert!(source.contains("err = kstrtoul(buf, 0, &min);"));
        assert!(source.contains("err = kstrtoul(buf, 0, &max);"));
        assert!(source.contains("__ATTR_RW_MODE(min, 0600)"));
        assert!(source.contains("__ATTR_RW_MODE(max, 0600)"));
        assert!(source.contains("ATTRIBUTE_GROUPS(damon_sysfs_ul_range);"));
        assert!(header.contains("extern struct mutex damon_sysfs_lock;"));
        assert!(header.contains("struct damon_sysfs_ul_range"));
        assert!(header.contains("extern const struct kobj_type damon_sysfs_ul_range_ktype;"));

        let mut range = damon_sysfs_ul_range_alloc(10, 20);
        assert_eq!(min_show(range), 10);
        assert_eq!(max_show(range), 20);
        assert_eq!(min_store(&mut range, "15\n"), Ok(3));
        assert_eq!(max_store(&mut range, "30"), Ok(2));
        assert_eq!(range, DamonSysfsUlRange { min: 15, max: 30 });
        assert_eq!(min_store(&mut range, "not-a-number"), Err(EINVAL));
        assert_eq!(DAMON_SYSFS_UL_RANGE_ATTR_MODE, 0o600);
    }
}
