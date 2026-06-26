//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/xfs_trace.c
//! test-origin: linux:vendor/linux/fs/xfs/xfs_trace.c
//! XFS tracepoint compile-unit shape.

pub const TRACE_SYSTEM: &str = "xfs";
pub const CREATE_TRACE_POINTS: bool = true;
pub const XFS_TRACE_LAST_INCLUDE: &str = "xfs_trace.h";
pub const XFS_TRACE_REQUIRED_INCLUDES: &[&str] = &[
    "xfs_platform.h",
    "xfs_fs.h",
    "xfs_shared.h",
    "xfs_bit.h",
    "xfs_format.h",
    "xfs_log_format.h",
    "xfs_trans_resv.h",
    "xfs_mount.h",
    "xfs_inode.h",
    "xfs_btree.h",
    "xfs_attr.h",
    "xfs_trans.h",
    "xfs_log.h",
    "xfs_error.h",
    "xfs_health.h",
    "xfs_file.h",
    "linux/fserror.h",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_trace_compile_unit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/xfs_trace.c"
        ));
        for include in XFS_TRACE_REQUIRED_INCLUDES {
            assert!(source.contains(include));
        }
        assert!(source.contains("/*"));
        assert!(source.contains("We include this last"));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("#include \"xfs_trace.h\""));
        assert_eq!(TRACE_SYSTEM, "xfs");
        assert!(CREATE_TRACE_POINTS);
        assert_eq!(XFS_TRACE_LAST_INCLUDE, "xfs_trace.h");
    }
}
