//! linux-parity: complete
//! linux-source: vendor/linux/fs/freevxfs/vxfs_immed.c
//! test-origin: linux:vendor/linux/fs/freevxfs/vxfs_immed.c
//! FreeVxFS immediate-inode folio read plan.

pub const PAGE_SIZE: usize = 4096;
pub const VXFS_IMMED_AOPS_SYMBOL: &str = "vxfs_immed_aops";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VxfsImmedReadPlan {
    pub source_offset: usize,
    pub pages: usize,
    pub bytes_copied: usize,
    pub mark_uptodate: bool,
    pub unlock: bool,
    pub result: i32,
}

pub const fn vxfs_immed_read_folio_plan(
    folio_pos: usize,
    folio_nr_pages: usize,
) -> VxfsImmedReadPlan {
    VxfsImmedReadPlan {
        source_offset: folio_pos,
        pages: folio_nr_pages,
        bytes_copied: folio_nr_pages * PAGE_SIZE,
        mark_uptodate: true,
        unlock: true,
        result: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vxfs_immed_read_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/freevxfs/vxfs_immed.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("#include \"vxfs.h\""));
        assert!(source.contains("#include \"vxfs_extern.h\""));
        assert!(source.contains("#include \"vxfs_inode.h\""));
        assert!(source.contains("static int vxfs_immed_read_folio"));
        assert!(source.contains("vip->vii_immed.vi_immed + folio_pos(folio)"));
        assert!(source.contains("for (i = 0; i < folio_nr_pages(folio); i++)"));
        assert!(source.contains("memcpy_to_page(folio_page(folio, i), 0, src, PAGE_SIZE);"));
        assert!(source.contains("src += PAGE_SIZE;"));
        assert!(source.contains("folio_mark_uptodate(folio);"));
        assert!(source.contains("folio_unlock(folio);"));
        assert!(source.contains(VXFS_IMMED_AOPS_SYMBOL));

        assert_eq!(
            vxfs_immed_read_folio_plan(8192, 2),
            VxfsImmedReadPlan {
                source_offset: 8192,
                pages: 2,
                bytes_copied: 8192,
                mark_uptodate: true,
                unlock: true,
                result: 0,
            }
        );
    }
}
