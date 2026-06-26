//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/scrub/fscounters_repair.c
//! test-origin: linux:vendor/linux/fs/xfs/scrub/fscounters_repair.c
//! XFS online repair filesystem counter reset plan.

use crate::include::uapi::errno::EUCLEAN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsFsCounters {
    pub frozen: bool,
    pub icount: i64,
    pub ifree: i64,
    pub fdblocks: i64,
    pub frextents: i64,
    pub frextents_delayed: i64,
    pub has_zoned: bool,
    pub has_rtgroups: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsFsCounterRepair {
    pub icount: i64,
    pub ifree: i64,
    pub fdblocks: i64,
    pub frextents_counter: Option<i64>,
    pub sb_frextents: Option<i64>,
}

pub const fn xrep_fscounters(counters: XfsFsCounters) -> Result<XfsFsCounterRepair, i32> {
    if !counters.frozen {
        return Err(-EUCLEAN);
    }

    let (frextents_counter, sb_frextents) = if counters.has_zoned {
        (None, None)
    } else if counters.has_rtgroups {
        (Some(counters.frextents - counters.frextents_delayed), None)
    } else {
        (
            Some(counters.frextents - counters.frextents_delayed),
            Some(counters.frextents),
        )
    };

    Ok(XfsFsCounterRepair {
        icount: counters.icount,
        ifree: counters.ifree,
        fdblocks: counters.fdblocks,
        frextents_counter,
        sb_frextents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_fscounters_repair_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/scrub/fscounters_repair.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"xfs_fs.h\""));
        assert!(source.contains("#include \"xfs_mount.h\""));
        assert!(source.contains("#include \"xfs_sb.h\""));
        assert!(source.contains("#include \"scrub/fscounters.h\""));
        assert!(source.contains("int"));
        assert!(source.contains("xrep_fscounters"));
        assert!(source.contains("struct xfs_mount\t*mp = sc->mp;"));
        assert!(source.contains("struct xchk_fscounters\t*fsc = sc->buf;"));
        assert!(source.contains("if (!fsc->frozen)"));
        assert!(source.contains("return -EFSCORRUPTED;"));
        assert!(source.contains("trace_xrep_reset_counters(mp, fsc);"));
        assert!(source.contains("percpu_counter_set(&mp->m_icount, fsc->icount);"));
        assert!(source.contains("percpu_counter_set(&mp->m_ifree, fsc->ifree);"));
        assert!(source.contains("xfs_set_freecounter(mp, XC_FREE_BLOCKS, fsc->fdblocks);"));
        assert!(source.contains("if (!xfs_has_zoned(mp))"));
        assert!(source.contains("XC_FREE_RTEXTENTS"));
        assert!(source.contains("fsc->frextents - fsc->frextents_delayed"));
        assert!(source.contains("if (!xfs_has_rtgroups(mp))"));
        assert!(source.contains("mp->m_sb.sb_frextents = fsc->frextents;"));

        assert_eq!(
            xrep_fscounters(XfsFsCounters {
                frozen: false,
                icount: 0,
                ifree: 0,
                fdblocks: 0,
                frextents: 0,
                frextents_delayed: 0,
                has_zoned: false,
                has_rtgroups: false,
            }),
            Err(-EUCLEAN)
        );
        assert_eq!(
            xrep_fscounters(XfsFsCounters {
                frozen: true,
                icount: 10,
                ifree: 2,
                fdblocks: 99,
                frextents: 50,
                frextents_delayed: 7,
                has_zoned: false,
                has_rtgroups: false,
            }),
            Ok(XfsFsCounterRepair {
                icount: 10,
                ifree: 2,
                fdblocks: 99,
                frextents_counter: Some(43),
                sb_frextents: Some(50),
            })
        );
        assert_eq!(
            xrep_fscounters(XfsFsCounters {
                frozen: true,
                icount: 1,
                ifree: 1,
                fdblocks: 1,
                frextents: 8,
                frextents_delayed: 3,
                has_zoned: false,
                has_rtgroups: true,
            })
            .unwrap()
            .sb_frextents,
            None
        );
        assert_eq!(
            xrep_fscounters(XfsFsCounters {
                frozen: true,
                icount: 1,
                ifree: 1,
                fdblocks: 1,
                frextents: 8,
                frextents_delayed: 3,
                has_zoned: true,
                has_rtgroups: false,
            })
            .unwrap()
            .frextents_counter,
            None
        );
    }
}
