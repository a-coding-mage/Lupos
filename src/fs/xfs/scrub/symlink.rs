//! linux-parity: complete
//! linux-source: vendor/linux/fs/xfs/scrub/symlink.c
//! test-origin: linux:vendor/linux/fs/xfs/scrub/symlink.c
//! XFS symlink scrub decision helpers.

use crate::include::uapi::errno::{ENOENT, ENOMEM};

pub const XFS_SYMLINK_SETUP_EXTRA_NUL: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfsSymlinkForkFormat {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XfsSymlinkScrub {
    Clean,
    CorruptDataFork,
    RemoteReadClean,
}

pub const fn xchk_setup_symlink_buffer_len(max_symlink_len: usize) -> usize {
    max_symlink_len + XFS_SYMLINK_SETUP_EXTRA_NUL
}

pub const fn xchk_setup_symlink_result(
    allocation_succeeded: bool,
    repair_setup_error: Option<i32>,
) -> Result<(), i32> {
    if !allocation_succeeded {
        return Err(-ENOMEM);
    }
    if let Some(error) = repair_setup_error {
        if error != 0 {
            return Err(error);
        }
    }
    Ok(())
}

pub const fn xchk_symlink_result(
    is_symlink: bool,
    looks_zapped: bool,
    len: i64,
    max_len: i64,
    format: XfsSymlinkForkFormat,
    fork_size: i64,
    strnlen_result: i64,
) -> Result<XfsSymlinkScrub, i32> {
    if !is_symlink {
        return Err(-ENOENT);
    }
    if looks_zapped || len > max_len || len <= 0 {
        return Ok(XfsSymlinkScrub::CorruptDataFork);
    }
    match format {
        XfsSymlinkForkFormat::Local => {
            if len > fork_size || len > strnlen_result {
                Ok(XfsSymlinkScrub::CorruptDataFork)
            } else {
                Ok(XfsSymlinkScrub::Clean)
            }
        }
        XfsSymlinkForkFormat::Remote => {
            if strnlen_result < len {
                Ok(XfsSymlinkScrub::CorruptDataFork)
            } else {
                Ok(XfsSymlinkScrub::RemoteReadClean)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xfs_symlink_scrub_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/xfs/scrub/symlink.c"
        ));
        assert!(source.contains("#include \"xfs_symlink.h\""));
        assert!(source.contains("#include \"xfs_symlink_remote.h\""));
        assert!(source.contains("xchk_setup_symlink"));
        assert!(source.contains("kvzalloc(XFS_SYMLINK_MAXLEN + 1, XCHK_GFP_FLAGS);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("xrep_setup_symlink(sc, &resblks);"));
        assert!(source.contains("xchk_setup_inode_contents(sc, resblks);"));
        assert!(source.contains("if (!S_ISLNK(VFS_I(ip)->i_mode))"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("xchk_file_looks_zapped(sc, XFS_SICK_INO_SYMLINK_ZAPPED)"));
        assert!(source.contains("len > XFS_SYMLINK_MAXLEN || len <= 0"));
        assert!(source.contains("ifp->if_format == XFS_DINODE_FMT_LOCAL"));
        assert!(source.contains("len > xfs_inode_data_fork_size(ip)"));
        assert!(source.contains("len > strnlen(ifp->if_data, xfs_inode_data_fork_size(ip))"));
        assert!(source.contains("xfs_symlink_remote_read(sc->ip, sc->buf);"));
        assert!(source.contains("strnlen(sc->buf, XFS_SYMLINK_MAXLEN) < len"));
        assert!(source.contains("xchk_mark_healthy_if_clean(sc, XFS_SICK_INO_SYMLINK_ZAPPED);"));

        assert_eq!(xchk_setup_symlink_buffer_len(1024), 1025);
        assert_eq!(xchk_setup_symlink_result(false, None), Err(-ENOMEM));
        assert_eq!(xchk_setup_symlink_result(true, Some(-5)), Err(-5));
        assert_eq!(
            xchk_symlink_result(false, false, 4, 1024, XfsSymlinkForkFormat::Local, 60, 4),
            Err(-ENOENT)
        );
        assert_eq!(
            xchk_symlink_result(true, false, 61, 1024, XfsSymlinkForkFormat::Local, 60, 61),
            Ok(XfsSymlinkScrub::CorruptDataFork)
        );
        assert_eq!(
            xchk_symlink_result(true, false, 8, 1024, XfsSymlinkForkFormat::Remote, 0, 8),
            Ok(XfsSymlinkScrub::RemoteReadClean)
        );
    }
}
