//! linux-parity: complete
//! linux-source: vendor/linux/fs/iomap/fiemap.c
//! test-origin: linux:vendor/linux/fs/iomap/fiemap.c
//! iomap to FIEMAP extent conversion.

pub const IOMAP_HOLE: u16 = 0;
pub const IOMAP_DELALLOC: u16 = 1;
pub const IOMAP_MAPPED: u16 = 2;
pub const IOMAP_UNWRITTEN: u16 = 3;
pub const IOMAP_INLINE: u16 = 4;
pub const IOMAP_NULL_ADDR: u64 = u64::MAX;

pub const IOMAP_F_SHARED: u16 = 1 << 2;
pub const IOMAP_F_MERGED: u16 = 1 << 3;
pub const IOMAP_REPORT: u32 = 1 << 2;

pub const FIEMAP_EXTENT_LAST: u32 = 0x0000_0001;
pub const FIEMAP_EXTENT_UNKNOWN: u32 = 0x0000_0002;
pub const FIEMAP_EXTENT_DELALLOC: u32 = 0x0000_0004;
pub const FIEMAP_EXTENT_DATA_INLINE: u32 = 0x0000_0200;
pub const FIEMAP_EXTENT_UNWRITTEN: u32 = 0x0000_0800;
pub const FIEMAP_EXTENT_MERGED: u32 = 0x0000_1000;
pub const FIEMAP_EXTENT_SHARED: u32 = 0x0000_2000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IomapFiemap {
    pub iomap_type: u16,
    pub flags: u16,
    pub offset: u64,
    pub addr: u64,
    pub length: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiemapExtent {
    pub logical: u64,
    pub physical: u64,
    pub length: u64,
    pub flags: u32,
}

pub fn iomap_to_fiemap(iomap: IomapFiemap, mut flags: u32) -> Option<FiemapExtent> {
    match iomap.iomap_type {
        IOMAP_HOLE => return None,
        IOMAP_DELALLOC => flags |= FIEMAP_EXTENT_DELALLOC | FIEMAP_EXTENT_UNKNOWN,
        IOMAP_MAPPED => {}
        IOMAP_UNWRITTEN => flags |= FIEMAP_EXTENT_UNWRITTEN,
        IOMAP_INLINE => flags |= FIEMAP_EXTENT_DATA_INLINE,
        _ => {}
    }

    if (iomap.flags & IOMAP_F_MERGED) != 0 {
        flags |= FIEMAP_EXTENT_MERGED;
    }
    if (iomap.flags & IOMAP_F_SHARED) != 0 {
        flags |= FIEMAP_EXTENT_SHARED;
    }

    Some(FiemapExtent {
        logical: iomap.offset,
        physical: if iomap.addr != IOMAP_NULL_ADDR {
            iomap.addr
        } else {
            0
        },
        length: iomap.length,
        flags,
    })
}

pub fn iomap_bmap_result(iomap: IomapFiemap, pos: u64, i_blkbits: u32) -> u64 {
    if iomap.iomap_type != IOMAP_MAPPED {
        return 0;
    }
    let blkshift = i_blkbits - 9;
    ((iomap.addr + pos - iomap.offset) >> 9) >> blkshift
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iomap_fiemap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/iomap/fiemap.c"
        ));
        assert!(source.contains("#include <linux/iomap.h>"));
        assert!(source.contains("#include <linux/fiemap.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("static int iomap_to_fiemap(struct fiemap_extent_info *fi,"));
        assert!(source.contains("case IOMAP_HOLE:"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("case IOMAP_DELALLOC:"));
        assert!(source.contains("flags |= FIEMAP_EXTENT_DELALLOC | FIEMAP_EXTENT_UNKNOWN;"));
        assert!(source.contains("case IOMAP_MAPPED:"));
        assert!(source.contains("case IOMAP_UNWRITTEN:"));
        assert!(source.contains("flags |= FIEMAP_EXTENT_UNWRITTEN;"));
        assert!(source.contains("case IOMAP_INLINE:"));
        assert!(source.contains("flags |= FIEMAP_EXTENT_DATA_INLINE;"));
        assert!(source.contains("if (iomap->flags & IOMAP_F_MERGED)"));
        assert!(source.contains("flags |= FIEMAP_EXTENT_MERGED;"));
        assert!(source.contains("if (iomap->flags & IOMAP_F_SHARED)"));
        assert!(source.contains("flags |= FIEMAP_EXTENT_SHARED;"));
        assert!(source.contains("iomap->addr != IOMAP_NULL_ADDR ? iomap->addr : 0"));
        assert!(source.contains("static int iomap_fiemap_iter(struct iomap_iter *iter,"));
        assert!(source.contains("if (iter->iomap.type == IOMAP_HOLE)"));
        assert!(source.contains("ret = iomap_to_fiemap(fi, prev, 0);"));
        assert!(source.contains("if (ret == 1)\t/* extent array full */"));
        assert!(source.contains("return iomap_iter_advance_full(iter);"));
        assert!(
            source.contains("int iomap_fiemap(struct inode *inode, struct fiemap_extent_info *fi,")
        );
        assert!(source.contains(".flags\t\t= IOMAP_REPORT"));
        assert!(source.contains(".type\t\t= IOMAP_HOLE"));
        assert!(source.contains("FIEMAP_EXTENT_LAST"));
        assert!(source.contains("ret < 0 && ret != -ENOENT"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(iomap_fiemap);"));
        assert!(source.contains("sector_t"));
        assert!(source.contains("iomap_bmap(struct address_space *mapping, sector_t bno,"));
        assert!(source.contains("if (filemap_write_and_wait(mapping))"));
        assert!(source.contains("if (iter.iomap.type == IOMAP_MAPPED)"));
        assert!(source.contains("bno = iomap_sector(&iter.iomap, iter.pos) >> blkshift;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(iomap_bmap);"));

        assert_eq!(
            iomap_to_fiemap(
                IomapFiemap {
                    iomap_type: IOMAP_HOLE,
                    flags: 0,
                    offset: 0,
                    addr: 0,
                    length: 1
                },
                0,
            ),
            None
        );
        assert_eq!(
            iomap_to_fiemap(
                IomapFiemap {
                    iomap_type: IOMAP_DELALLOC,
                    flags: IOMAP_F_SHARED | IOMAP_F_MERGED,
                    offset: 10,
                    addr: IOMAP_NULL_ADDR,
                    length: 5
                },
                0,
            )
            .unwrap(),
            FiemapExtent {
                logical: 10,
                physical: 0,
                length: 5,
                flags: FIEMAP_EXTENT_DELALLOC
                    | FIEMAP_EXTENT_UNKNOWN
                    | FIEMAP_EXTENT_SHARED
                    | FIEMAP_EXTENT_MERGED
            }
        );
        assert_eq!(
            iomap_to_fiemap(
                IomapFiemap {
                    iomap_type: IOMAP_UNWRITTEN,
                    flags: 0,
                    offset: 0,
                    addr: 4096,
                    length: 4096
                },
                FIEMAP_EXTENT_LAST,
            )
            .unwrap()
            .flags,
            FIEMAP_EXTENT_LAST | FIEMAP_EXTENT_UNWRITTEN
        );
        assert_eq!(
            iomap_bmap_result(
                IomapFiemap {
                    iomap_type: IOMAP_MAPPED,
                    flags: 0,
                    offset: 0,
                    addr: 8192,
                    length: 4096
                },
                0,
                12,
            ),
            2
        );
    }
}
