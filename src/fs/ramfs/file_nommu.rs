//! linux-parity: complete
//! linux-source: vendor/linux/fs/ramfs/file-nommu.c
//! test-origin: linux:vendor/linux/fs/ramfs/file-nommu.c
//! ramfs NOMMU file helpers and operation tables.
//!
//! Ref: `vendor/linux/fs/ramfs/file-nommu.c`

use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EFBIG, ENOMEM, ENOSYS};

pub use crate::fs::attr::{ATTR_CTIME, ATTR_MTIME, ATTR_SIZE};

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = crate::mm::frame::PAGE_SIZE;
pub const MAX_PAGE_ORDER: usize = crate::mm::zone::MAX_PAGE_ORDER;

pub const VMA_MAYREAD_BIT: u32 = 4;
pub const VMA_MAYWRITE_BIT: u32 = 5;
pub const VMA_MAYEXEC_BIT: u32 = 6;
pub const VMA_MAYSHARE_BIT: u32 = 7;
pub const VMA_MAYOVERLAY_BIT: u32 = 9;

pub const VM_MAYREAD: u64 = 1u64 << VMA_MAYREAD_BIT;
pub const VM_MAYWRITE: u64 = 1u64 << VMA_MAYWRITE_BIT;
pub const VM_MAYEXEC: u64 = 1u64 << VMA_MAYEXEC_BIT;
pub const VM_MAYSHARE: u64 = 1u64 << VMA_MAYSHARE_BIT;
pub const VM_MAYOVERLAY: u64 = 1u64 << VMA_MAYOVERLAY_BIT;

pub const NOMMU_MAP_COPY: u64 = 0x0000_0001;
pub const NOMMU_MAP_DIRECT: u64 = 0x0000_0008;
pub const NOMMU_MAP_READ: u64 = VM_MAYREAD;
pub const NOMMU_MAP_WRITE: u64 = VM_MAYWRITE;
pub const NOMMU_MAP_EXEC: u64 = VM_MAYEXEC;
pub const RAMFS_NOMMU_MMAP_CAPABILITIES: u64 =
    NOMMU_MAP_DIRECT | NOMMU_MAP_COPY | NOMMU_MAP_READ | NOMMU_MAP_WRITE | NOMMU_MAP_EXEC;

pub const RAMFS_FILE_OPERATIONS_SYMBOL: &str = "ramfs_file_operations";
pub const RAMFS_FILE_OPERATIONS: &[(&str, &str)] = &[
    ("mmap_capabilities", "ramfs_mmap_capabilities"),
    ("mmap_prepare", "ramfs_nommu_mmap_prepare"),
    ("get_unmapped_area", "ramfs_nommu_get_unmapped_area"),
    ("read_iter", "generic_file_read_iter"),
    ("write_iter", "generic_file_write_iter"),
    ("fsync", "noop_fsync"),
    ("splice_read", "filemap_splice_read"),
    ("splice_write", "iter_file_splice_write"),
    ("llseek", "generic_file_llseek"),
];
pub const RAMFS_FILE_INODE_OPERATIONS_SYMBOL: &str = "ramfs_file_inode_operations";
pub const RAMFS_FILE_INODE_OPERATIONS: &[(&str, &str)] = &[
    ("setattr", "ramfs_nommu_setattr"),
    ("getattr", "simple_getattr"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamfsNommuKernelResponses {
    pub setattr_prepare: i32,
    pub inode_newsize: i32,
    pub alloc_pages: bool,
    pub shrink_inode_mappings: i32,
    pub add_to_page_cache_error_at: Option<(u64, i32)>,
}

impl RamfsNommuKernelResponses {
    pub const fn success() -> Self {
        Self {
            setattr_prepare: 0,
            inode_newsize: 0,
            alloc_pages: true,
            shrink_inode_mappings: 0,
            add_to_page_cache_error_at: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamfsNommuExpandPlan {
    pub order: usize,
    pub xpages: u64,
    pub npages: u64,
    pub trimmed_pages: u64,
    pub zero_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RamfsNommuResizePlan {
    Expand(RamfsNommuExpandPlan),
    ShrinkAndTruncate { old_size: u64, new_size: u64 },
    Truncate { old_size: u64, new_size: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamfsNommuSetattrPlan {
    pub ret: i32,
    pub original_ia_valid: u32,
    pub effective_ia_valid: u32,
    pub restored_ia_valid: u32,
    pub resize: Option<RamfsNommuResizePlan>,
    pub copy_attrs: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamfsFolio {
    pub index: u64,
    pub pfn: u64,
    pub pages: u64,
    pub address: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamfsNommuMmapPreparePlan {
    pub file_accessed: bool,
    pub vm_ops: &'static str,
}

pub fn read(file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    crate::fs::libfs::ram_file_read(file, buf, pos)
}

pub fn write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    crate::fs::libfs::ram_file_write(file, buf, pos)
}

pub const fn supports_mmap() -> bool {
    true
}

pub const fn ramfs_mmap_capabilities() -> u64 {
    RAMFS_NOMMU_MMAP_CAPABILITIES
}

pub const fn linux_get_order(size: u64) -> usize {
    if size == 0 {
        return u64::BITS as usize - PAGE_SHIFT;
    }
    if size < PAGE_SIZE as u64 {
        return 0;
    }
    (u64::BITS as usize - (size - 1).leading_zeros() as usize) - PAGE_SHIFT
}

pub const fn ramfs_nommu_pages_for_len(len: u64) -> u64 {
    if len == 0 {
        0
    } else {
        ((len - 1) >> PAGE_SHIFT) + 1
    }
}

pub fn ramfs_nommu_expand_for_mapping_plan(
    newsize: u64,
    responses: RamfsNommuKernelResponses,
) -> Result<RamfsNommuExpandPlan, i32> {
    let order = linux_get_order(newsize);
    if order > MAX_PAGE_ORDER {
        return Err(-EFBIG);
    }

    if responses.inode_newsize != 0 {
        return Err(responses.inode_newsize);
    }

    if !responses.alloc_pages {
        return Err(-ENOMEM);
    }

    let xpages = 1u64 << order;
    let npages = ramfs_nommu_pages_for_len(newsize);

    if let Some((page, ret)) = responses.add_to_page_cache_error_at {
        if page < npages && ret != 0 {
            return Err(ret);
        }
    }

    Ok(RamfsNommuExpandPlan {
        order,
        xpages,
        npages,
        trimmed_pages: xpages.saturating_sub(npages),
        zero_bytes: npages.saturating_mul(PAGE_SIZE as u64),
    })
}

pub fn ramfs_nommu_resize_plan(
    old_size: u64,
    new_size: u64,
    responses: RamfsNommuKernelResponses,
) -> Result<RamfsNommuResizePlan, i32> {
    if old_size == 0 {
        if new_size >> 32 != 0 {
            return Err(-EFBIG);
        }
        return ramfs_nommu_expand_for_mapping_plan(new_size, responses)
            .map(RamfsNommuResizePlan::Expand);
    }

    if new_size < old_size {
        if responses.shrink_inode_mappings < 0 {
            return Err(responses.shrink_inode_mappings);
        }
        return Ok(RamfsNommuResizePlan::ShrinkAndTruncate { old_size, new_size });
    }

    Ok(RamfsNommuResizePlan::Truncate { old_size, new_size })
}

pub fn ramfs_nommu_setattr_plan(
    current_size: u64,
    ia_size: u64,
    ia_valid: u32,
    responses: RamfsNommuKernelResponses,
) -> RamfsNommuSetattrPlan {
    let old_ia_valid = ia_valid;
    if responses.setattr_prepare != 0 {
        return RamfsNommuSetattrPlan {
            ret: responses.setattr_prepare,
            original_ia_valid: old_ia_valid,
            effective_ia_valid: ia_valid,
            restored_ia_valid: old_ia_valid,
            resize: None,
            copy_attrs: false,
        };
    }

    let mut effective_ia_valid = ia_valid;
    let mut resize = None;
    let mut copy_attrs = true;

    if effective_ia_valid & ATTR_SIZE != 0 {
        if ia_size != current_size {
            match ramfs_nommu_resize_plan(current_size, ia_size, responses) {
                Ok(plan) => resize = Some(plan),
                Err(ret) => {
                    return RamfsNommuSetattrPlan {
                        ret,
                        original_ia_valid: old_ia_valid,
                        effective_ia_valid,
                        restored_ia_valid: old_ia_valid,
                        resize: None,
                        copy_attrs: false,
                    };
                }
            }

            if effective_ia_valid == ATTR_SIZE {
                copy_attrs = false;
            }
        } else {
            effective_ia_valid |= ATTR_MTIME | ATTR_CTIME;
        }
    }

    RamfsNommuSetattrPlan {
        ret: 0,
        original_ia_valid: old_ia_valid,
        effective_ia_valid,
        restored_ia_valid: old_ia_valid,
        resize,
        copy_attrs,
    }
}

pub const fn is_nommu_shared_vma_flags(flags: u64) -> bool {
    flags & (VM_MAYSHARE | VM_MAYOVERLAY) != 0
}

pub fn ramfs_nommu_mmap_prepare(vma_flags: u64) -> Result<RamfsNommuMmapPreparePlan, i32> {
    if !is_nommu_shared_vma_flags(vma_flags) {
        return Err(-ENOSYS);
    }

    Ok(RamfsNommuMmapPreparePlan {
        file_accessed: true,
        vm_ops: "generic_file_vm_ops",
    })
}

pub fn ramfs_nommu_get_unmapped_area_plan(
    isize: u64,
    _addr: u64,
    len: u64,
    pgoff: u64,
    _flags: u64,
    folios: &[RamfsFolio],
) -> Result<u64, i32> {
    let lpages = ramfs_nommu_pages_for_len(len);
    let maxpages = ramfs_nommu_pages_for_len(isize);

    if pgoff >= maxpages {
        return Err(-ENOSYS);
    }
    if maxpages - pgoff < lpages {
        return Err(-ENOSYS);
    }

    let mut ret = None;
    let mut nr_pages = 0;
    let mut expected_index = pgoff;
    let mut expected_pfn = 0;

    for folio in folios {
        if folio.index != expected_index || folio.pages == 0 {
            return Err(-ENOSYS);
        }
        if ret.is_none() {
            ret = Some(folio.address);
            expected_pfn = folio.pfn;
        }
        if expected_pfn + nr_pages != folio.pfn {
            return Err(-ENOSYS);
        }

        nr_pages += folio.pages;
        expected_index += folio.pages;
        if nr_pages >= lpages {
            return Ok(ret.unwrap_or(folio.address));
        }
    }

    Err(-ENOSYS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::attr::ATTR_MODE;
    use crate::include::uapi::errno::{EINVAL, EIO};

    fn success() -> RamfsNommuKernelResponses {
        RamfsNommuKernelResponses::success()
    }

    #[test]
    fn ramfs_nommu_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ramfs/file-nommu.c"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ramfs/internal.h"
        ));
        let fs_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/fs.h"
        ));
        let mm_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/mm.h"
        ));
        let getorder_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/getorder.h"
        ));

        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/mm.h>"));
        assert!(source.contains("#include <linux/ramfs.h>"));
        assert!(source.contains("#include <linux/folio_batch.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("static unsigned ramfs_mmap_capabilities"));
        assert!(source.contains("return NOMMU_MAP_DIRECT | NOMMU_MAP_COPY | NOMMU_MAP_READ |"));
        assert!(source.contains("ramfs_nommu_expand_for_mapping"));
        assert!(source.contains("order = get_order(newsize);"));
        assert!(source.contains("if (unlikely(order > MAX_PAGE_ORDER))"));
        assert!(source.contains("ret = inode_newsize_ok(inode, newsize);"));
        assert!(source.contains("pages = alloc_pages(gfp, order);"));
        assert!(source.contains("ret = add_to_page_cache_lru(page, inode->i_mapping, loop,"));
        assert!(source.contains("if (unlikely(newsize >> 32))"));
        assert!(source.contains("nommu_shrink_inode_mappings(inode, size, newsize);"));
        assert!(source.contains("ia->ia_valid |= ATTR_MTIME|ATTR_CTIME;"));
        assert!(source.contains("filemap_get_folios_contig(inode->i_mapping, &pgoff,"));
        assert!(source.contains("if (!is_nommu_shared_vma_flags(&desc->vma_flags))"));
        assert!(source.contains("desc->vm_ops = &generic_file_vm_ops;"));
        assert!(
            internal.contains("extern const struct inode_operations ramfs_file_inode_operations;")
        );
        assert!(fs_h.contains("#define ATTR_SIZE\t(1 << 3)"));
        assert!(fs_h.contains("#define ATTR_MTIME\t(1 << 5)"));
        assert!(fs_h.contains("#define ATTR_CTIME\t(1 << 6)"));
        assert!(fs_h.contains("#define NOMMU_MAP_DIRECT\t0x00000008"));
        assert!(mm_h.contains("DECLARE_VMA_BIT(MAYREAD, 4)"));
        assert!(mm_h.contains("DECLARE_VMA_BIT(MAYSHARE, 7)"));
        assert!(mm_h.contains("DECLARE_VMA_BIT(MAYOVERLAY, 9)"));
        assert!(getorder_h.contains("return ilog2((size) - 1) - PAGE_SHIFT + 1;"));
        assert!(source.contains(RAMFS_FILE_OPERATIONS_SYMBOL));
        assert!(source.contains(RAMFS_FILE_INODE_OPERATIONS_SYMBOL));

        for (slot, target) in RAMFS_FILE_OPERATIONS
            .iter()
            .chain(RAMFS_FILE_INODE_OPERATIONS.iter())
        {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }

    #[test]
    fn mmap_capabilities_and_prepare_match_nommu_flags() {
        assert!(supports_mmap());
        assert_eq!(NOMMU_MAP_COPY, 0x1);
        assert_eq!(NOMMU_MAP_DIRECT, 0x8);
        assert_eq!(NOMMU_MAP_READ, 1 << 4);
        assert_eq!(NOMMU_MAP_WRITE, 1 << 5);
        assert_eq!(NOMMU_MAP_EXEC, 1 << 6);
        assert_eq!(ramfs_mmap_capabilities(), 0x79);

        assert_eq!(ramfs_nommu_mmap_prepare(0), Err(-ENOSYS));
        let shared = ramfs_nommu_mmap_prepare(VM_MAYSHARE).unwrap();
        assert!(shared.file_accessed);
        assert_eq!(shared.vm_ops, "generic_file_vm_ops");
        assert!(ramfs_nommu_mmap_prepare(VM_MAYOVERLAY).is_ok());
    }

    #[test]
    fn expand_for_mapping_matches_get_order_and_error_checks() {
        assert_eq!(linux_get_order(1), 0);
        assert_eq!(linux_get_order(PAGE_SIZE as u64), 0);
        assert_eq!(linux_get_order(PAGE_SIZE as u64 + 1), 1);
        assert_eq!(linux_get_order(0), u64::BITS as usize - PAGE_SHIFT);

        let plan = ramfs_nommu_expand_for_mapping_plan(PAGE_SIZE as u64 + 1, success()).unwrap();
        assert_eq!(
            plan,
            RamfsNommuExpandPlan {
                order: 1,
                xpages: 2,
                npages: 2,
                trimmed_pages: 0,
                zero_bytes: 2 * PAGE_SIZE as u64,
            }
        );

        let single = ramfs_nommu_expand_for_mapping_plan(1, success()).unwrap();
        assert_eq!(single.order, 0);
        assert_eq!(single.xpages, 1);
        assert_eq!(single.npages, 1);
        assert_eq!(single.zero_bytes, PAGE_SIZE as u64);

        let oversized = ((PAGE_SIZE as u64) << MAX_PAGE_ORDER) + 1;
        assert_eq!(
            ramfs_nommu_expand_for_mapping_plan(oversized, success()),
            Err(-EFBIG)
        );

        let mut responses = success();
        responses.inode_newsize = -EINVAL;
        assert_eq!(
            ramfs_nommu_expand_for_mapping_plan(PAGE_SIZE as u64, responses),
            Err(-EINVAL)
        );

        responses = success();
        responses.alloc_pages = false;
        assert_eq!(
            ramfs_nommu_expand_for_mapping_plan(PAGE_SIZE as u64, responses),
            Err(-ENOMEM)
        );

        responses = success();
        responses.add_to_page_cache_error_at = Some((0, -EIO));
        assert_eq!(
            ramfs_nommu_expand_for_mapping_plan(PAGE_SIZE as u64, responses),
            Err(-EIO)
        );
    }

    #[test]
    fn resize_plan_matches_zero_growth_shrink_and_truncate_paths() {
        assert_eq!(
            ramfs_nommu_resize_plan(0, 1u64 << 32, success()),
            Err(-EFBIG)
        );

        let grow = ramfs_nommu_resize_plan(0, PAGE_SIZE as u64, success()).unwrap();
        assert_eq!(
            grow,
            RamfsNommuResizePlan::Expand(RamfsNommuExpandPlan {
                order: 0,
                xpages: 1,
                npages: 1,
                trimmed_pages: 0,
                zero_bytes: PAGE_SIZE as u64,
            })
        );

        let mut responses = success();
        responses.shrink_inode_mappings = -EIO;
        assert_eq!(ramfs_nommu_resize_plan(8192, 4096, responses), Err(-EIO));
        assert_eq!(
            ramfs_nommu_resize_plan(8192, 4096, success()),
            Ok(RamfsNommuResizePlan::ShrinkAndTruncate {
                old_size: 8192,
                new_size: 4096,
            })
        );
        assert_eq!(
            ramfs_nommu_resize_plan(8192, 12288, success()),
            Ok(RamfsNommuResizePlan::Truncate {
                old_size: 8192,
                new_size: 12288,
            })
        );
    }

    #[test]
    fn setattr_plan_restores_ia_valid_and_copies_attrs_like_linux() {
        let same_size = ramfs_nommu_setattr_plan(4096, 4096, ATTR_SIZE, success());
        assert_eq!(same_size.ret, 0);
        assert_eq!(same_size.original_ia_valid, ATTR_SIZE);
        assert_eq!(same_size.restored_ia_valid, ATTR_SIZE);
        assert_eq!(
            same_size.effective_ia_valid,
            ATTR_SIZE | ATTR_MTIME | ATTR_CTIME
        );
        assert!(same_size.copy_attrs);
        assert_eq!(same_size.resize, None);

        let size_only = ramfs_nommu_setattr_plan(4096, 8192, ATTR_SIZE, success());
        assert_eq!(size_only.ret, 0);
        assert!(!size_only.copy_attrs);
        assert_eq!(
            size_only.resize,
            Some(RamfsNommuResizePlan::Truncate {
                old_size: 4096,
                new_size: 8192,
            })
        );

        let mixed = ramfs_nommu_setattr_plan(4096, 8192, ATTR_SIZE | ATTR_MODE, success());
        assert_eq!(mixed.ret, 0);
        assert!(mixed.copy_attrs);
        assert_eq!(mixed.effective_ia_valid, ATTR_SIZE | ATTR_MODE);

        let mut responses = success();
        responses.shrink_inode_mappings = -EIO;
        let shrink_error = ramfs_nommu_setattr_plan(8192, 4096, ATTR_SIZE | ATTR_MODE, responses);
        assert_eq!(shrink_error.ret, -EIO);
        assert_eq!(shrink_error.restored_ia_valid, ATTR_SIZE | ATTR_MODE);
        assert!(!shrink_error.copy_attrs);

        responses = success();
        responses.setattr_prepare = -EINVAL;
        let prepare_error = ramfs_nommu_setattr_plan(4096, 4096, ATTR_SIZE, responses);
        assert_eq!(prepare_error.ret, -EINVAL);
        assert!(!prepare_error.copy_attrs);
    }

    #[test]
    fn get_unmapped_area_requires_eof_and_physically_contiguous_folios() {
        let folios = [
            RamfsFolio {
                index: 1,
                pfn: 100,
                pages: 1,
                address: 0x1000_0000,
            },
            RamfsFolio {
                index: 2,
                pfn: 101,
                pages: 1,
                address: 0x1000_1000,
            },
        ];
        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                3 * PAGE_SIZE as u64,
                0,
                2 * PAGE_SIZE as u64,
                1,
                0,
                &folios
            ),
            Ok(0x1000_0000)
        );

        let huge_folio = [RamfsFolio {
            index: 0,
            pfn: 200,
            pages: 2,
            address: 0x2000_0000,
        }];
        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                2 * PAGE_SIZE as u64,
                0,
                2 * PAGE_SIZE as u64,
                0,
                0,
                &huge_folio
            ),
            Ok(0x2000_0000)
        );

        let nonadjacent = [
            RamfsFolio {
                index: 0,
                pfn: 300,
                pages: 1,
                address: 0x3000_0000,
            },
            RamfsFolio {
                index: 1,
                pfn: 302,
                pages: 1,
                address: 0x3000_1000,
            },
        ];
        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                2 * PAGE_SIZE as u64,
                0,
                2 * PAGE_SIZE as u64,
                0,
                0,
                &nonadjacent
            ),
            Err(-ENOSYS)
        );

        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                PAGE_SIZE as u64,
                0,
                PAGE_SIZE as u64,
                1,
                0,
                &folios
            ),
            Err(-ENOSYS)
        );
        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                PAGE_SIZE as u64,
                0,
                2 * PAGE_SIZE as u64,
                0,
                0,
                &folios
            ),
            Err(-ENOSYS)
        );
        assert_eq!(
            ramfs_nommu_get_unmapped_area_plan(
                3 * PAGE_SIZE as u64,
                0,
                PAGE_SIZE as u64,
                0,
                0,
                &folios
            ),
            Err(-ENOSYS)
        );
    }
}
