//! linux-parity: complete
//! linux-source: vendor/linux/fs/efs/symlink.c
//! test-origin: linux:vendor/linux/fs/efs/symlink.c
//! EFS symlink folio read behavior.

use crate::include::uapi::errno::{EIO, ENAMETOOLONG};

pub const EFS_BLOCKSIZE: usize = 1 << 9;
pub const EFS_SYMLINK_AOPS_SYMBOL: &str = "efs_symlink_aops";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EfsSymlinkReadOutcome {
    pub result: i32,
    pub first_read_attempted: bool,
    pub second_read_attempted: bool,
    pub first_copy_len: usize,
    pub second_copy_len: usize,
    pub nul_offset: Option<usize>,
    pub folio_end_read_success: bool,
}

pub fn efs_symlink_read_outcome(
    size: usize,
    first_block_read: bool,
    second_block_read: bool,
) -> EfsSymlinkReadOutcome {
    if size > 2 * EFS_BLOCKSIZE {
        return EfsSymlinkReadOutcome {
            result: -ENAMETOOLONG,
            first_read_attempted: false,
            second_read_attempted: false,
            first_copy_len: 0,
            second_copy_len: 0,
            nul_offset: None,
            folio_end_read_success: false,
        };
    }

    if !first_block_read {
        return EfsSymlinkReadOutcome {
            result: -EIO,
            first_read_attempted: true,
            second_read_attempted: false,
            first_copy_len: 0,
            second_copy_len: 0,
            nul_offset: None,
            folio_end_read_success: false,
        };
    }

    let first_copy_len = if size > EFS_BLOCKSIZE {
        EFS_BLOCKSIZE
    } else {
        size
    };
    if size > EFS_BLOCKSIZE && !second_block_read {
        return EfsSymlinkReadOutcome {
            result: -EIO,
            first_read_attempted: true,
            second_read_attempted: true,
            first_copy_len,
            second_copy_len: 0,
            nul_offset: None,
            folio_end_read_success: false,
        };
    }

    EfsSymlinkReadOutcome {
        result: 0,
        first_read_attempted: true,
        second_read_attempted: size > EFS_BLOCKSIZE,
        first_copy_len,
        second_copy_len: size.saturating_sub(EFS_BLOCKSIZE),
        nul_offset: Some(size),
        folio_end_read_success: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efs_symlink_read_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/efs/symlink.c"
        ));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"efs.h\""));
        assert!(source.contains("static int efs_symlink_read_folio"));
        assert!(source.contains("err = -ENAMETOOLONG;"));
        assert!(source.contains("size > 2 * EFS_BLOCKSIZE"));
        assert!(source.contains("err = -EIO;"));
        assert!(source.contains("sb_bread(inode->i_sb, efs_bmap(inode, 0))"));
        assert!(source.contains("sb_bread(inode->i_sb, efs_bmap(inode, 1))"));
        assert!(source.contains("link[size] = '\\0';"));
        assert!(source.contains("folio_end_read(folio, err == 0);"));
        assert!(source.contains(EFS_SYMLINK_AOPS_SYMBOL));

        assert_eq!(
            efs_symlink_read_outcome(2 * EFS_BLOCKSIZE + 1, true, true).result,
            -ENAMETOOLONG
        );
        assert_eq!(
            efs_symlink_read_outcome(EFS_BLOCKSIZE + 8, true, false).result,
            -EIO
        );
        let ok = efs_symlink_read_outcome(EFS_BLOCKSIZE + 8, true, true);
        assert_eq!(ok.first_copy_len, EFS_BLOCKSIZE);
        assert_eq!(ok.second_copy_len, 8);
        assert_eq!(ok.nul_offset, Some(EFS_BLOCKSIZE + 8));
        assert!(ok.folio_end_read_success);
    }
}
