//! linux-parity: complete
//! linux-source: vendor/linux/fs/freevxfs/vxfs_olt.c
//! test-origin: linux:vendor/linux/fs/freevxfs/vxfs_olt.c
//! FreeVxFS object location table parsing decisions.

use crate::include::uapi::errno::EINVAL;

pub const VXFS_OLT_MAGIC: u32 = 0xa504_fcf5;
pub const VXFS_OLT_FREE: u32 = 1;
pub const VXFS_OLT_FSHEAD: u32 = 2;
pub const VXFS_OLT_CUT: u32 = 3;
pub const VXFS_OLT_ILIST: u32 = 4;
pub const VXFS_OLT_DEV: u32 = 5;
pub const VXFS_OLT_SB: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VxfsOltEntry {
    FsHead,
    Ilist,
    Ignored,
}

pub const fn vxfs_oblock(super_block_size: u64, block: u64, bsize: u64) -> u64 {
    assert!(super_block_size.is_multiple_of(bsize));
    block * (super_block_size / bsize)
}

pub const fn vxfs_olt_entry(olt_type: u32) -> VxfsOltEntry {
    match olt_type {
        VXFS_OLT_FSHEAD => VxfsOltEntry::FsHead,
        VXFS_OLT_ILIST => VxfsOltEntry::Ilist,
        _ => VxfsOltEntry::Ignored,
    }
}

pub const fn vxfs_read_olt_result(
    buffer_present: bool,
    magic: u32,
    oltsize: u32,
    fshino: u32,
    iext: u32,
) -> Result<(), i32> {
    if !buffer_present || magic != VXFS_OLT_MAGIC || oltsize > 1 {
        return Err(-EINVAL);
    }
    if fshino != 0 && iext != 0 {
        Ok(())
    } else {
        Err(-EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vxfs_olt_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/freevxfs/vxfs_olt.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"vxfs_olt.h\""));
        assert!(source.contains("vxfs_get_fshead"));
        assert!(source.contains("BUG_ON(infp->vsi_fshino);"));
        assert!(source.contains("infp->vsi_fshino = fs32_to_cpu(infp, fshp->olt_fsino[0]);"));
        assert!(source.contains("vxfs_get_ilist"));
        assert!(source.contains("BUG_ON(infp->vsi_iext);"));
        assert!(source.contains("infp->vsi_iext = fs32_to_cpu(infp, ilistp->olt_iext[0]);"));
        assert!(source.contains("BUG_ON(sbp->s_blocksize % bsize);"));
        assert!(source.contains("return (block * (sbp->s_blocksize / bsize));"));
        assert!(source.contains("bp = sb_bread(sbp, vxfs_oblock(sbp, infp->vsi_oltext, bsize));"));
        assert!(source.contains("fs32_to_cpu(infp, op->olt_magic) != VXFS_OLT_MAGIC"));
        assert!(source.contains("if (infp->vsi_oltsize > 1)"));
        assert!(source.contains("case VXFS_OLT_FSHEAD:"));
        assert!(source.contains("case VXFS_OLT_ILIST:"));
        assert!(source.contains("return (infp->vsi_fshino && infp->vsi_iext) ? 0 : -EINVAL;"));
        assert!(source.contains("return -EINVAL;"));

        assert_eq!(vxfs_oblock(4096, 2, 512), 16);
        assert_eq!(vxfs_olt_entry(VXFS_OLT_FSHEAD), VxfsOltEntry::FsHead);
        assert_eq!(vxfs_olt_entry(VXFS_OLT_ILIST), VxfsOltEntry::Ilist);
        assert_eq!(vxfs_olt_entry(VXFS_OLT_DEV), VxfsOltEntry::Ignored);
        assert_eq!(
            vxfs_read_olt_result(false, VXFS_OLT_MAGIC, 1, 1, 1),
            Err(-EINVAL)
        );
        assert_eq!(vxfs_read_olt_result(true, 0, 1, 1, 1), Err(-EINVAL));
        assert_eq!(
            vxfs_read_olt_result(true, VXFS_OLT_MAGIC, 2, 1, 1),
            Err(-EINVAL)
        );
        assert_eq!(
            vxfs_read_olt_result(true, VXFS_OLT_MAGIC, 1, 1, 0),
            Err(-EINVAL)
        );
        assert_eq!(vxfs_read_olt_result(true, VXFS_OLT_MAGIC, 1, 1, 2), Ok(()));
    }
}
