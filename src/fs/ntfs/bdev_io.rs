//! linux-parity: complete
//! linux-source: vendor/linux/fs/ntfs/bdev-io.c
//! test-origin: linux:vendor/linux/fs/ntfs/bdev-io.c
//! NTFS direct block-device I/O planning.

use crate::include::uapi::errno::EINVAL;

pub const SECTOR_SHIFT: u32 = 9;
pub const SECTOR_SIZE: u64 = 1 << SECTOR_SHIFT;
pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;

pub const REQ_OP_READ: u32 = 0;
pub const REQ_META: u32 = 1 << 0;
pub const REQ_SYNC: u32 = 1 << 1;
pub const NTFS_BDEV_READ_OP: u32 = REQ_OP_READ | REQ_META | REQ_SYNC;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NtfsBdevReadPath {
    BdevRwVirt,
    VmallocBio { max_segments: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NtfsBdevReadPlan {
    pub sector: u64,
    pub op: u32,
    pub path: NtfsBdevReadPath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NtfsBdevWritePlan {
    pub first_index: u64,
    pub end_index_exclusive: u64,
    pub first_from: u32,
}

pub fn ntfs_bdev_read_plan(
    start: u64,
    size: usize,
    is_vmalloc: bool,
) -> Result<NtfsBdevReadPlan, i32> {
    if (start & (SECTOR_SIZE - 1)) != 0 {
        return Err(-EINVAL);
    }
    let sector = start >> SECTOR_SHIFT;
    let path = if is_vmalloc {
        NtfsBdevReadPath::VmallocBio {
            max_segments: div_round_up(size as u64, PAGE_SIZE),
        }
    } else {
        NtfsBdevReadPath::BdevRwVirt
    };
    Ok(NtfsBdevReadPlan {
        sector,
        op: NTFS_BDEV_READ_OP,
        path,
    })
}

pub fn ntfs_bdev_write_plan(start: u64, size: usize) -> NtfsBdevWritePlan {
    let end = start + size as u64;
    let idx = start >> PAGE_SHIFT;
    let mut idx_end = end >> PAGE_SHIFT;
    if idx == idx_end {
        idx_end += 1;
    }
    NtfsBdevWritePlan {
        first_index: idx,
        end_index_exclusive: idx_end,
        first_from: (start & (PAGE_SIZE - 1)) as u32,
    }
}

pub const fn ntfs_bdev_write_page_to(end: u64, page_index: u64) -> u32 {
    let offset = page_index << PAGE_SHIFT;
    let remaining = end - offset;
    if remaining < PAGE_SIZE {
        remaining as u32
    } else {
        PAGE_SIZE as u32
    }
}

const fn div_round_up(value: u64, divisor: u64) -> u64 {
    if value == 0 {
        0
    } else {
        ((value - 1) / divisor) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntfs_bdev_io_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ntfs/bdev-io.c"
        ));
        assert!(source.contains("#include <linux/blkdev.h>"));
        assert!(source.contains("#include \"ntfs.h\""));
        assert!(source.contains(
            "int ntfs_bdev_read(struct block_device *bdev, char *data, loff_t start, size_t size)"
        ));
        assert!(source.contains("sector_t sector = start >> SECTOR_SHIFT;"));
        assert!(source.contains("if (start & (SECTOR_SIZE - 1))"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("op = REQ_OP_READ | REQ_META | REQ_SYNC;"));
        assert!(source.contains("if (!is_vmalloc_addr(data))"));
        assert!(source.contains("return bdev_rw_virt(bdev, sector, data, size, op);"));
        assert!(source.contains("bio = bio_alloc(bdev,"));
        assert!(source.contains("bio_max_segs(DIV_ROUND_UP(size, PAGE_SIZE))"));
        assert!(source.contains("bio->bi_iter.bi_sector = sector;"));
        assert!(source.contains("added = bio_add_vmalloc_chunk(bio, data + done, size - done);"));
        assert!(source.contains("if (!added)"));
        assert!(source.contains("bio_chain(prev, bio);"));
        assert!(source.contains("submit_bio(prev);"));
        assert!(source.contains("error = submit_bio_wait(bio);"));
        assert!(source.contains("bio_put(bio);"));
        assert!(source.contains("if (op == REQ_OP_READ)"));
        assert!(source.contains("invalidate_kernel_vmap_range(data, size);"));
        assert!(source.contains(
            "int ntfs_bdev_write(struct super_block *sb, void *buf, loff_t start, size_t size)"
        ));
        assert!(source.contains("idx = start >> PAGE_SHIFT;"));
        assert!(source.contains("idx_end = end >> PAGE_SHIFT;"));
        assert!(source.contains("from = start & ~PAGE_MASK;"));
        assert!(source.contains("if (idx == idx_end)"));
        assert!(source.contains("idx_end++;"));
        assert!(source.contains("folio = read_mapping_folio(sb->s_bdev->bd_mapping, idx, NULL);"));
        assert!(source.contains("offset = (loff_t)idx << PAGE_SHIFT;"));
        assert!(source.contains("to = min_t(u32, end - offset, PAGE_SIZE);"));
        assert!(source.contains("memcpy_to_folio(folio, from, buf + buf_off, len);"));
        assert!(source.contains("folio_mark_uptodate(folio);"));
        assert!(source.contains("folio_mark_dirty(folio);"));

        assert_eq!(ntfs_bdev_read_plan(1, 4096, false), Err(-EINVAL));
        assert_eq!(
            ntfs_bdev_read_plan(1024, 8193, true),
            Ok(NtfsBdevReadPlan {
                sector: 2,
                op: NTFS_BDEV_READ_OP,
                path: NtfsBdevReadPath::VmallocBio { max_segments: 3 }
            })
        );
        assert_eq!(
            ntfs_bdev_read_plan(512, 4096, false).unwrap().path,
            NtfsBdevReadPath::BdevRwVirt
        );
        assert_eq!(
            ntfs_bdev_write_plan(4096 + 128, 100),
            NtfsBdevWritePlan {
                first_index: 1,
                end_index_exclusive: 2,
                first_from: 128
            }
        );
        assert_eq!(ntfs_bdev_write_page_to(8192 + 7, 2), 7);
    }
}
