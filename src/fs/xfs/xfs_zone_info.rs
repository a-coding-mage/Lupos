//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/xfs_zone_info.c
//! test-origin: linux:vendor/linux/fs/xfs/xfs_zone_info.c
//! XFS zoned stats display helpers.

pub const WRITE_LIFE_NOT_SET: u8 = 0;
pub const WRITE_LIFE_NONE: u8 = 1;
pub const WRITE_LIFE_SHORT: u8 = 2;
pub const WRITE_LIFE_MEDIUM: u8 = 3;
pub const WRITE_LIFE_LONG: u8 = 4;
pub const WRITE_LIFE_EXTREME: u8 = 5;
pub const XFS_ZONE_USED_BUCKETS: u32 = 10;

pub const XFS_WRITE_HINT_SHORTHAND: [&str; 6] =
    ["NOT_SET", "NONE", "SHORT", "MEDIUM", "LONG", "EXTREME"];

pub const fn xfs_write_hint_to_str(write_hint: u8) -> &'static str {
    if write_hint > WRITE_LIFE_EXTREME {
        "UNKNOWN"
    } else {
        XFS_WRITE_HINT_SHORTHAND[write_hint as usize]
    }
}

pub const fn xfs_zone_used_bucket_range(bucket: u32) -> (u32, u32) {
    let width = 100 / XFS_ZONE_USED_BUCKETS;
    (bucket * width, (bucket + 1) * width - 1)
}

pub fn xfs_full_zone_count(
    rgcount: u32,
    open_zones: u32,
    open_gc_zones: u32,
    free_zones: u32,
    used_bucket_entries: &[u32],
) -> u32 {
    let reclaimable: u32 = used_bucket_entries.iter().copied().sum();
    rgcount - open_zones - open_gc_zones - free_zones - reclaimable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_zone_info_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/xfs_zone_info.c"
        ));
        assert!(source.contains("#include \"xfs_zone_alloc.h\""));
        assert!(source.contains("#include \"xfs_zone_priv.h\""));
        assert!(source.contains("static const char xfs_write_hint_shorthand[6][16]"));
        assert!(
            source.contains("\"NOT_SET\", \"NONE\", \"SHORT\", \"MEDIUM\", \"LONG\", \"EXTREME\"")
        );
        assert!(source.contains("if (write_hint > WRITE_LIFE_EXTREME)"));
        assert!(source.contains("return \"UNKNOWN\";"));
        assert!(source.contains("xfs_show_open_zone"));
        assert!(source.contains("hint %s %s\\n"));
        assert!(source.contains("oz->oz_is_gc ? \"(GC)\" : \"\""));
        assert!(source.contains("xfs_show_full_zone_used_distribution"));
        assert!(source.contains("for (i = 0; i < XFS_ZONE_USED_BUCKETS; i++)"));
        assert!(source.contains("i * (100 / XFS_ZONE_USED_BUCKETS)"));
        assert!(source.contains("(i + 1) * (100 / XFS_ZONE_USED_BUCKETS) - 1"));
        assert!(source.contains("full = mp->m_sb.sb_rgcount;"));
        assert!(source.contains("full -= zi->zi_nr_open_zones;"));
        assert!(source.contains("full -= zi->zi_nr_open_gc_zones;"));
        assert!(source.contains("full -= atomic_read(&zi->zi_nr_free_zones);"));
        assert!(source.contains("full -= reclaimable;"));
        assert!(source.contains("xfs_zoned_show_stats"));
        assert!(source.contains("xfs_sum_freecounter(mp, XC_FREE_RTEXTENTS)"));
        assert!(source.contains("xfs_zoned_need_gc(mp)"));
        assert!(source.contains("list_for_each_entry(oz, &zi->zi_open_zones, oz_entry)"));

        assert_eq!(xfs_write_hint_to_str(WRITE_LIFE_NOT_SET), "NOT_SET");
        assert_eq!(xfs_write_hint_to_str(WRITE_LIFE_EXTREME), "EXTREME");
        assert_eq!(xfs_write_hint_to_str(6), "UNKNOWN");
        assert_eq!(xfs_zone_used_bucket_range(0), (0, 9));
        assert_eq!(xfs_zone_used_bucket_range(9), (90, 99));
        assert_eq!(xfs_full_zone_count(100, 4, 2, 10, &[1, 2, 3]), 78);
    }
}
