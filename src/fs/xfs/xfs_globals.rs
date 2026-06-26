//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/xfs_globals.c
//! test-origin: linux:vendor/linux/fs/xfs/xfs_globals.c
//! XFS global tunables and debug defaults.

pub const XFS_PTAG_IFLUSH: u32 = 1 << 0;
pub const XFS_PTAG_LOGRES: u32 = 1 << 1;
pub const XFS_PTAG_AILDELETE: u32 = 1 << 2;
pub const XFS_PTAG_ERROR_REPORT: u32 = 1 << 3;
pub const XFS_PTAG_SHUTDOWN_CORRUPT: u32 = 1 << 4;
pub const XFS_PTAG_SHUTDOWN_IOERROR: u32 = 1 << 5;
pub const XFS_PTAG_SHUTDOWN_LOGERROR: u32 = 1 << 6;
pub const XFS_PTAG_FSBLOCK_ZERO: u32 = 1 << 7;
pub const XFS_PTAG_VERIFIER_ERROR: u32 = 1 << 8;
pub const XFS_PTAG_MASK: u32 = XFS_PTAG_IFLUSH
    | XFS_PTAG_LOGRES
    | XFS_PTAG_AILDELETE
    | XFS_PTAG_ERROR_REPORT
    | XFS_PTAG_SHUTDOWN_CORRUPT
    | XFS_PTAG_SHUTDOWN_IOERROR
    | XFS_PTAG_SHUTDOWN_LOGERROR
    | XFS_PTAG_FSBLOCK_ZERO
    | XFS_PTAG_VERIFIER_ERROR;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsSysctlVal {
    pub min: i32,
    pub val: i32,
    pub max: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsParams {
    pub panic_mask: XfsSysctlVal,
    pub error_level: XfsSysctlVal,
    pub syncd_timer: XfsSysctlVal,
    pub stats_clear: XfsSysctlVal,
    pub inherit_sync: XfsSysctlVal,
    pub inherit_nodump: XfsSysctlVal,
    pub inherit_noatim: XfsSysctlVal,
    pub inherit_nosym: XfsSysctlVal,
    pub rotorstep: XfsSysctlVal,
    pub inherit_nodfrg: XfsSysctlVal,
    pub fstrm_timer: XfsSysctlVal,
    pub blockgc_timer: XfsSysctlVal,
}

pub const XFS_PARAMS: XfsParams = XfsParams {
    panic_mask: XfsSysctlVal {
        min: 0,
        val: 0,
        max: XFS_PTAG_MASK as i32,
    },
    error_level: XfsSysctlVal {
        min: 0,
        val: 3,
        max: 11,
    },
    syncd_timer: XfsSysctlVal {
        min: 100,
        val: 30 * 100,
        max: 7200 * 100,
    },
    stats_clear: XfsSysctlVal {
        min: 0,
        val: 0,
        max: 1,
    },
    inherit_sync: XfsSysctlVal {
        min: 0,
        val: 1,
        max: 1,
    },
    inherit_nodump: XfsSysctlVal {
        min: 0,
        val: 1,
        max: 1,
    },
    inherit_noatim: XfsSysctlVal {
        min: 0,
        val: 1,
        max: 1,
    },
    inherit_nosym: XfsSysctlVal {
        min: 0,
        val: 0,
        max: 1,
    },
    rotorstep: XfsSysctlVal {
        min: 1,
        val: 1,
        max: 255,
    },
    inherit_nodfrg: XfsSysctlVal {
        min: 0,
        val: 1,
        max: 1,
    },
    fstrm_timer: XfsSysctlVal {
        min: 1,
        val: 30 * 100,
        max: 3600 * 100,
    },
    blockgc_timer: XfsSysctlVal {
        min: 1,
        val: 300,
        max: 3600 * 24,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XfsGlobals {
    pub log_recovery_delay: i32,
    pub mount_delay: i32,
    pub bug_on_assert: bool,
    pub pwork_threads: Option<i32>,
    pub larp: Option<bool>,
    pub bload_leaf_slack: i32,
    pub bload_node_slack: i32,
    pub always_cow: bool,
}

pub const fn xfs_globals_default(assert_fatal: bool, debug: bool) -> XfsGlobals {
    XfsGlobals {
        log_recovery_delay: 0,
        mount_delay: 0,
        bug_on_assert: assert_fatal,
        pwork_threads: if debug { Some(-1) } else { None },
        larp: if debug { Some(false) } else { None },
        bload_leaf_slack: -1,
        bload_node_slack: -1,
        always_cow: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_globals_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/xfs_globals.c"
        ));
        assert!(source.contains("#include \"xfs_platform.h\""));
        assert!(source.contains("#include \"xfs_error.h\""));
        assert!(source.contains("xfs_param_t xfs_params"));
        assert!(source.contains(".panic_mask\t= {\t0,\t\t0,\t\tXFS_PTAG_MASK}"));
        assert!(source.contains(".syncd_timer\t= {\t1*100,\t\t30*100,\t\t7200*100}"));
        assert!(source.contains(".blockgc_timer\t= {\t1,\t\t300,\t\t3600*24}"));
        assert!(source.contains("struct xfs_globals xfs_globals"));
        assert!(source.contains(".log_recovery_delay\t=\t0"));
        assert!(source.contains(".mount_delay\t\t=\t0"));
        assert!(source.contains("#ifdef XFS_ASSERT_FATAL"));
        assert!(source.contains(".bug_on_assert\t\t=\ttrue"));
        assert!(source.contains(".bug_on_assert\t\t=\tfalse"));
        assert!(source.contains("#ifdef DEBUG"));
        assert!(source.contains(".pwork_threads\t\t=\t-1"));
        assert!(source.contains(".larp\t\t\t=\tfalse"));
        assert!(source.contains(".bload_leaf_slack\t=\t-1"));
        assert!(source.contains(".bload_node_slack\t=\t-1"));

        assert_eq!(XFS_PARAMS.panic_mask.max, XFS_PTAG_MASK as i32);
        assert_eq!(XFS_PARAMS.syncd_timer.val, 3000);
        assert_eq!(XFS_PARAMS.blockgc_timer.max, 3600 * 24);
        assert!(!xfs_globals_default(false, false).bug_on_assert);
        let debug = xfs_globals_default(true, true);
        assert!(debug.bug_on_assert);
        assert_eq!(debug.pwork_threads, Some(-1));
        assert_eq!(debug.larp, Some(false));
    }
}
