//! linux-parity: complete
//! linux-source: vendor/linux/fs/romfs/mmap-nommu.c
//! test-origin: linux:vendor/linux/fs/romfs/mmap-nommu.c
//! NOMMU mmap checks for ROMFS-on-MTD files.

use crate::include::uapi::errno::{EINVAL, ENOSYS, EOPNOTSUPP};

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const NOMMU_MAP_COPY: u32 = 0x0000_0001;

pub const fn romfs_mmap_prepare(shared_nommu_mapping: bool) -> i32 {
    if shared_nommu_mapping { 0 } else { -ENOSYS }
}

pub const fn romfs_mmap_capabilities(has_mtd: bool, mtd_capabilities: u32) -> u32 {
    if has_mtd {
        mtd_capabilities
    } else {
        NOMMU_MAP_COPY
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RomfsUnmappedAreaInput {
    pub has_mtd: bool,
    pub addr: u64,
    pub len: u64,
    pub pgoff: u64,
    pub inode_size: u64,
    pub mtd_size: u64,
    pub data_offset: u64,
    pub mtd_result: i64,
}

pub const fn romfs_get_unmapped_area(input: RomfsUnmappedAreaInput) -> i64 {
    if !input.has_mtd {
        return -(ENOSYS as i64);
    }

    let lpages = input.len.saturating_add(PAGE_SIZE - 1) >> PAGE_SHIFT;
    let maxpages = input.inode_size.saturating_add(PAGE_SIZE - 1) >> PAGE_SHIFT;
    if input.pgoff >= maxpages || maxpages - input.pgoff < lpages {
        return -(EINVAL as i64);
    }
    if input.addr != 0 {
        return -(EINVAL as i64);
    }
    if input.len > input.mtd_size || input.pgoff >= (input.mtd_size >> PAGE_SHIFT) {
        return -(EINVAL as i64);
    }
    let Some(offset) = (input.pgoff << PAGE_SHIFT).checked_add(input.data_offset) else {
        return -(EINVAL as i64);
    };
    if offset >= input.mtd_size {
        return -(EINVAL as i64);
    }

    if input.mtd_result == -(EOPNOTSUPP as i64) {
        -(ENOSYS as i64)
    } else {
        input.mtd_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romfs_nommu_mmap_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/romfs/mmap-nommu.c"
        ));
        assert!(source.contains("#include <linux/mm.h>"));
        assert!(source.contains("#include <linux/mtd/super.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("static unsigned long romfs_get_unmapped_area"));
        assert!(source.contains("if (!mtd)"));
        assert!(source.contains("return (unsigned long) -ENOSYS;"));
        assert!(source.contains("if ((pgoff >= maxpages) || (maxpages - pgoff < lpages))"));
        assert!(source.contains("if (addr != 0)"));
        assert!(source.contains("offset += ROMFS_I(inode)->i_dataoffset;"));
        assert!(source.contains("mtd_get_unmapped_area(mtd, len, offset, flags);"));
        assert!(source.contains("if (ret == -EOPNOTSUPP)"));
        assert!(source.contains("static int romfs_mmap_prepare"));
        assert!(source.contains("is_nommu_shared_vma_flags"));
        assert!(source.contains("const struct file_operations romfs_ro_fops"));

        assert_eq!(romfs_mmap_prepare(true), 0);
        assert_eq!(romfs_mmap_prepare(false), -ENOSYS);
        assert_eq!(romfs_mmap_capabilities(false, 0), NOMMU_MAP_COPY);
        assert_eq!(romfs_mmap_capabilities(true, 7), 7);
        assert_eq!(
            romfs_get_unmapped_area(RomfsUnmappedAreaInput {
                has_mtd: false,
                addr: 0,
                len: 4096,
                pgoff: 0,
                inode_size: 4096,
                mtd_size: 8192,
                data_offset: 0,
                mtd_result: 0x1000,
            }),
            -38
        );
        assert_eq!(
            romfs_get_unmapped_area(RomfsUnmappedAreaInput {
                has_mtd: true,
                addr: 0,
                len: 4096,
                pgoff: 0,
                inode_size: 4096,
                mtd_size: 8192,
                data_offset: 0,
                mtd_result: 0x2000,
            }),
            0x2000
        );
        assert_eq!(
            romfs_get_unmapped_area(RomfsUnmappedAreaInput {
                has_mtd: true,
                addr: 1,
                len: 4096,
                pgoff: 0,
                inode_size: 4096,
                mtd_size: 8192,
                data_offset: 0,
                mtd_result: 0,
            }),
            -22
        );
        assert_eq!(
            romfs_get_unmapped_area(RomfsUnmappedAreaInput {
                has_mtd: true,
                addr: 0,
                len: 4096,
                pgoff: 0,
                inode_size: 4096,
                mtd_size: 8192,
                data_offset: 0,
                mtd_result: -95,
            }),
            -38
        );
    }
}
