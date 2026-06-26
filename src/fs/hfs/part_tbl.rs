//! linux-parity: complete
//! linux-source: vendor/linux/fs/hfs/part_tbl.c
//! test-origin: linux:vendor/linux/fs/hfs/part_tbl.c
//! HFS old and new Macintosh partition map scanning.

use crate::include::uapi::errno::{EINVAL, ENOENT};

pub const HFS_PMAP_BLK: u64 = 1;
pub const HFS_OLD_PMAP_MAGIC: u16 = 0x5453;
pub const HFS_NEW_PMAP_MAGIC: u16 = 0x504d;
pub const HFS_OLD_PMAP_TFS1: u32 = 0x5446_5331;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsPart {
    pub start: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsOldPmapEntry {
    pub start: u32,
    pub size: u32,
    pub fsid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsNewPmapEntry<'a> {
    pub sig: u16,
    pub map_blocks: u32,
    pub part_start: u32,
    pub part_size: u32,
    pub part_type: &'a [u8],
}

pub fn hfs_part_find_old(
    part_start: u64,
    requested_part: i32,
    entries: &[HfsOldPmapEntry],
) -> Result<HfsPart, i32> {
    let mut found = None;
    for (i, entry) in entries.iter().take(42).enumerate() {
        if entry.start != 0
            && entry.size != 0
            && entry.fsid == HFS_OLD_PMAP_TFS1
            && (requested_part < 0 || requested_part == i as i32)
        {
            found = Some(HfsPart {
                start: part_start + entry.start as u64,
                size: entry.size as u64,
            });
        }
    }
    found.ok_or(-ENOENT)
}

pub fn hfs_part_find_new(
    part_start: u64,
    requested_part: i32,
    entries: &[HfsNewPmapEntry<'_>],
) -> Result<HfsPart, i32> {
    let Some(first) = entries.first() else {
        return Err(-EINVAL);
    };
    let size = first.map_blocks as usize;
    for i in 0..size {
        let Some(entry) = entries.get(i) else {
            return Err(-EINVAL);
        };
        if i != 0 && entry.sig != HFS_NEW_PMAP_MAGIC {
            break;
        }
        if entry.part_type.len() >= 9
            && &entry.part_type[..9] == b"Apple_HFS"
            && (requested_part < 0 || requested_part == i as i32)
        {
            return Ok(HfsPart {
                start: part_start + entry.part_start as u64,
                size: entry.part_size as u64,
            });
        }
    }
    Err(-ENOENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hfs_part_tbl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hfs/part_tbl.c"
        ));
        assert!(source.contains("#include \"hfs_fs.h\""));
        assert!(source.contains("struct new_pmap"));
        assert!(source.contains("__be16\tpmSig;"));
        assert!(source.contains("__be32\tpmMapBlkCnt;"));
        assert!(source.contains("u8\tpmPartType[32];"));
        assert!(source.contains("struct old_pmap"));
        assert!(source.contains("struct \told_pmap_entry"));
        assert!(source.contains("int hfs_part_find(struct super_block *sb,"));
        assert!(source.contains("res = -ENOENT;"));
        assert!(source.contains("bh = sb_bread512(sb, *part_start + HFS_PMAP_BLK, data);"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains("case HFS_OLD_PMAP_MAGIC:"));
        assert!(source.contains("size = 42;"));
        assert!(source.contains("p->pdFSID == cpu_to_be32(0x54465331)"));
        assert!(source.contains("*part_start += be32_to_cpu(p->pdStart);"));
        assert!(source.contains("*part_size = be32_to_cpu(p->pdSize);"));
        assert!(source.contains("case HFS_NEW_PMAP_MAGIC:"));
        assert!(source.contains("size = be32_to_cpu(pm->pmMapBlkCnt);"));
        assert!(source.contains("!memcmp(pm->pmPartType,\"Apple_HFS\", 9)"));
        assert!(source.contains("bh = sb_bread512(sb, *part_start + HFS_PMAP_BLK + ++i, pm);"));
        assert!(source.contains("if (pm->pmSig != cpu_to_be16(HFS_NEW_PMAP_MAGIC))"));
        assert!(source.contains("brelse(bh);"));

        let old = [
            HfsOldPmapEntry {
                start: 2,
                size: 10,
                fsid: HFS_OLD_PMAP_TFS1,
            },
            HfsOldPmapEntry {
                start: 20,
                size: 5,
                fsid: HFS_OLD_PMAP_TFS1,
            },
        ];
        assert_eq!(
            hfs_part_find_old(100, -1, &old),
            Ok(HfsPart {
                start: 120,
                size: 5
            })
        );
        assert_eq!(
            hfs_part_find_old(100, 0, &old),
            Ok(HfsPart {
                start: 102,
                size: 10
            })
        );
        assert_eq!(hfs_part_find_old(100, 7, &old), Err(-ENOENT));

        let new = [
            HfsNewPmapEntry {
                sig: HFS_NEW_PMAP_MAGIC,
                map_blocks: 2,
                part_start: 0,
                part_size: 0,
                part_type: b"Apple_Free",
            },
            HfsNewPmapEntry {
                sig: HFS_NEW_PMAP_MAGIC,
                map_blocks: 2,
                part_start: 4,
                part_size: 44,
                part_type: b"Apple_HFS\0",
            },
        ];
        assert_eq!(
            hfs_part_find_new(100, -1, &new),
            Ok(HfsPart {
                start: 104,
                size: 44
            })
        );
        assert_eq!(hfs_part_find_new(100, 0, &new), Err(-ENOENT));
    }
}
