//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/libxfs/xfs_zones.c
//! test-origin: linux:vendor/linux/fs/xfs/libxfs/xfs_zones.c
//! XFS zoned-device validation helpers.

pub const BLK_ZONE_TYPE_CONVENTIONAL: u32 = 0x1;
pub const BLK_ZONE_TYPE_SEQWRITE_REQ: u32 = 0x2;

pub const BLK_ZONE_COND_NOT_WP: u32 = 0x0;
pub const BLK_ZONE_COND_EMPTY: u32 = 0x1;
pub const BLK_ZONE_COND_IMP_OPEN: u32 = 0x2;
pub const BLK_ZONE_COND_EXP_OPEN: u32 = 0x3;
pub const BLK_ZONE_COND_CLOSED: u32 = 0x4;
pub const BLK_ZONE_COND_READONLY: u32 = 0x0d;
pub const BLK_ZONE_COND_FULL: u32 = 0x0e;
pub const BLK_ZONE_COND_OFFLINE: u32 = 0x0f;
pub const BLK_ZONE_COND_ACTIVE: u32 = 0xff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsBlkZone {
    pub zone_type: u32,
    pub cond: u32,
    pub start: u64,
    pub len: u64,
    pub capacity: u64,
    pub wp: u64,
}

pub const fn xfs_bb_to_fsb(bb: u64, blkbb_log: u8) -> u64 {
    (bb + ((1u64 << blkbb_log) - 1)) >> blkbb_log
}

pub fn xfs_validate_blk_zone(
    zone: XfsBlkZone,
    zone_no: u32,
    expected_size: u32,
    expected_capacity: u32,
    blkbb_log: u8,
) -> Result<Option<u32>, u32> {
    let _ = zone_no;
    if xfs_bb_to_fsb(zone.capacity, blkbb_log) != expected_capacity as u64 {
        return Err(zone.cond);
    }
    if xfs_bb_to_fsb(zone.len, blkbb_log) != expected_size as u64 {
        return Err(zone.cond);
    }

    match zone.zone_type {
        BLK_ZONE_TYPE_CONVENTIONAL => xfs_validate_blk_zone_conv(zone),
        BLK_ZONE_TYPE_SEQWRITE_REQ => xfs_validate_blk_zone_seq(zone, blkbb_log),
        _ => Err(zone.cond),
    }
}

fn xfs_validate_blk_zone_conv(zone: XfsBlkZone) -> Result<Option<u32>, u32> {
    match zone.cond {
        BLK_ZONE_COND_NOT_WP => Ok(None),
        _ => Err(zone.cond),
    }
}

fn xfs_validate_blk_zone_seq(zone: XfsBlkZone, blkbb_log: u8) -> Result<Option<u32>, u32> {
    match zone.cond {
        BLK_ZONE_COND_EMPTY => Ok(Some(0)),
        BLK_ZONE_COND_IMP_OPEN
        | BLK_ZONE_COND_EXP_OPEN
        | BLK_ZONE_COND_CLOSED
        | BLK_ZONE_COND_ACTIVE => {
            if zone.wp < zone.start || zone.wp >= zone.start + zone.capacity {
                return Err(zone.cond);
            }
            Ok(Some(xfs_bb_to_fsb(zone.wp - zone.start, blkbb_log) as u32))
        }
        BLK_ZONE_COND_FULL => Ok(Some(xfs_bb_to_fsb(zone.capacity, blkbb_log) as u32)),
        BLK_ZONE_COND_NOT_WP | BLK_ZONE_COND_OFFLINE | BLK_ZONE_COND_READONLY => Err(zone.cond),
        _ => Err(zone.cond),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_zones_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/libxfs/xfs_zones.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"xfs_fs.h\""));
        assert!(source.contains("#include \"xfs_shared.h\""));
        assert!(source.contains("#include \"xfs_format.h\""));
        assert!(source.contains("#include \"xfs_log_format.h\""));
        assert!(source.contains("#include \"xfs_trans_resv.h\""));
        assert!(source.contains("#include \"xfs_mount.h\""));
        assert!(source.contains("#include \"xfs_inode.h\""));
        assert!(source.contains("#include \"xfs_rtgroup.h\""));
        assert!(source.contains("#include \"xfs_zones.h\""));
        assert!(source.contains("static bool"));
        assert!(source.contains("xfs_validate_blk_zone_seq("));
        assert!(source.contains("case BLK_ZONE_COND_EMPTY:"));
        assert!(source.contains("*write_pointer = 0;"));
        assert!(source.contains("case BLK_ZONE_COND_IMP_OPEN:"));
        assert!(source.contains("case BLK_ZONE_COND_ACTIVE:"));
        assert!(source.contains("zone->wp < zone->start ||"));
        assert!(source.contains("zone->wp >= zone->start + zone->capacity"));
        assert!(source.contains("*write_pointer = XFS_BB_TO_FSB(mp, zone->wp - zone->start);"));
        assert!(source.contains("case BLK_ZONE_COND_FULL:"));
        assert!(source.contains("*write_pointer = XFS_BB_TO_FSB(mp, zone->capacity);"));
        assert!(source.contains("xfs_validate_blk_zone_conv("));
        assert!(source.contains("case BLK_ZONE_COND_NOT_WP:"));
        assert!(source.contains("bool"));
        assert!(source.contains("xfs_validate_blk_zone("));
        assert!(source.contains("XFS_BB_TO_FSB(mp, zone->capacity) != expected_capacity"));
        assert!(source.contains("XFS_BB_TO_FSB(mp, zone->len) != expected_size"));
        assert!(source.contains("case BLK_ZONE_TYPE_CONVENTIONAL:"));
        assert!(source.contains("case BLK_ZONE_TYPE_SEQWRITE_REQ:"));

        let seq_empty = XfsBlkZone {
            zone_type: BLK_ZONE_TYPE_SEQWRITE_REQ,
            cond: BLK_ZONE_COND_EMPTY,
            start: 100,
            len: 16,
            capacity: 16,
            wp: 100,
        };
        assert_eq!(xfs_validate_blk_zone(seq_empty, 0, 4, 4, 2), Ok(Some(0)));
        assert_eq!(
            xfs_validate_blk_zone(
                XfsBlkZone {
                    cond: BLK_ZONE_COND_EXP_OPEN,
                    wp: 108,
                    ..seq_empty
                },
                0,
                4,
                4,
                2,
            ),
            Ok(Some(2))
        );
        assert_eq!(
            xfs_validate_blk_zone(
                XfsBlkZone {
                    cond: BLK_ZONE_COND_FULL,
                    wp: 116,
                    ..seq_empty
                },
                0,
                4,
                4,
                2,
            ),
            Ok(Some(4))
        );
        assert!(
            xfs_validate_blk_zone(
                XfsBlkZone {
                    wp: 116,
                    cond: BLK_ZONE_COND_CLOSED,
                    ..seq_empty
                },
                0,
                4,
                4,
                2
            )
            .is_err()
        );
        assert_eq!(
            xfs_validate_blk_zone(
                XfsBlkZone {
                    zone_type: BLK_ZONE_TYPE_CONVENTIONAL,
                    cond: BLK_ZONE_COND_NOT_WP,
                    ..seq_empty
                },
                0,
                4,
                4,
                2,
            ),
            Ok(None)
        );
        assert!(xfs_validate_blk_zone(seq_empty, 0, 5, 4, 2).is_err());
    }
}
