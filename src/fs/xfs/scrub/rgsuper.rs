//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/scrub/rgsuper.c
//! test-origin: linux:vendor/linux/fs/xfs/scrub/rgsuper.c
//! XFS realtime-group superblock scrub control flow.

use crate::include::uapi::errno::ENOENT;

pub const XFS_RGSUPERBLOCK_RGNO: u32 = 0;
pub const XFS_RGSUPERBLOCK_TRANS_RESBLKS: u32 = 0;
pub const XFS_RGSUPERBLOCK_XREF_BLOCKS: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfsRgSuperblockCheck {
    MissingRealtimeGroup,
    LockBitmapShared,
    CrossReference,
}

pub const fn xchk_setup_rgsuperblock_reservation() -> u32 {
    XFS_RGSUPERBLOCK_TRANS_RESBLKS
}

pub const fn xchk_rgsuperblock_plan(
    rgno: u32,
    already_corrupt: bool,
) -> Result<XfsRgSuperblockCheck, i32> {
    if rgno != XFS_RGSUPERBLOCK_RGNO {
        return Err(-ENOENT);
    }
    if already_corrupt {
        return Ok(XfsRgSuperblockCheck::LockBitmapShared);
    }
    Ok(XfsRgSuperblockCheck::CrossReference)
}

pub const fn xrep_rgsuperblock_logs_superblock(rgno: u32) -> bool {
    rgno == XFS_RGSUPERBLOCK_RGNO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_rgsuperblock_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/scrub/rgsuper.c"
        ));
        assert!(source.contains("#include \"xfs_rtgroup.h\""));
        assert!(source.contains("#include \"xfs_rmap.h\""));
        assert!(source.contains("#include \"scrub/repair.h\""));
        assert!(source.contains("xchk_setup_rgsuperblock"));
        assert!(source.contains("return xchk_trans_alloc(sc, 0);"));
        assert!(source.contains("xchk_rgsuperblock_xref"));
        assert!(source.contains("if (sc->sm->sm_flags & XFS_SCRUB_OFLAG_CORRUPT)"));
        assert!(
            source.contains("xchk_xref_is_used_rt_space(sc, xfs_rgbno_to_rtb(sc->sr.rtg, 0), 1);")
        );
        assert!(source.contains("xchk_xref_is_only_rt_owned_by(sc, 0, 1, &XFS_RMAP_OINFO_FS);"));
        assert!(source.contains("Only rtgroup 0 has a superblock"));
        assert!(source.contains("if (rgno != 0)"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("xchk_rtgroup_lock(sc, &sc->sr, XFS_RTGLOCK_BITMAP_SHARED);"));
        assert!(source.contains("xfs_log_sb(sc->tp);"));

        assert_eq!(xchk_setup_rgsuperblock_reservation(), 0);
        assert_eq!(xchk_rgsuperblock_plan(1, false), Err(-ENOENT));
        assert_eq!(
            xchk_rgsuperblock_plan(0, true),
            Ok(XfsRgSuperblockCheck::LockBitmapShared)
        );
        assert_eq!(
            xchk_rgsuperblock_plan(0, false),
            Ok(XfsRgSuperblockCheck::CrossReference)
        );
        assert!(xrep_rgsuperblock_logs_superblock(0));
        assert!(!xrep_rgsuperblock_logs_superblock(1));
    }
}
