//! linux-parity: complete
//! linux-source: vendor/linux/fs/minix/file.c
//! test-origin: linux:vendor/linux/fs/minix/file.c
//! Minix regular file operation tables and setattr flow.

pub const MINIX_FSYNC_HELPER: &str = "mmb_fsync";
pub const MINIX_FILE_OPERATIONS_SYMBOL: &str = "minix_file_operations";
pub const MINIX_FILE_OPERATIONS: &[(&str, &str)] = &[
    ("llseek", "generic_file_llseek"),
    ("read_iter", "generic_file_read_iter"),
    ("write_iter", "generic_file_write_iter"),
    ("mmap_prepare", "generic_file_mmap_prepare"),
    ("fsync", "minix_fsync"),
    ("splice_read", "filemap_splice_read"),
];
pub const MINIX_INODE_OPERATIONS_SYMBOL: &str = "minix_file_inode_operations";
pub const MINIX_INODE_OPERATIONS: &[(&str, &str)] =
    &[("setattr", "minix_setattr"), ("getattr", "minix_getattr")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MinixSetattrOutcome {
    pub result: i32,
    pub newsize_checked: bool,
    pub truncated: bool,
    pub attrs_copied: bool,
    pub inode_marked_dirty: bool,
}

pub const fn minix_setattr_outcome(
    setattr_prepare_result: i32,
    requested_size: Option<u64>,
    current_size: u64,
    inode_newsize_ok_result: i32,
) -> MinixSetattrOutcome {
    if setattr_prepare_result != 0 {
        return MinixSetattrOutcome {
            result: setattr_prepare_result,
            newsize_checked: false,
            truncated: false,
            attrs_copied: false,
            inode_marked_dirty: false,
        };
    }

    let mut newsize_checked = false;
    let mut truncated = false;
    if let Some(size) = requested_size {
        if size != current_size {
            newsize_checked = true;
            if inode_newsize_ok_result != 0 {
                return MinixSetattrOutcome {
                    result: inode_newsize_ok_result,
                    newsize_checked,
                    truncated: false,
                    attrs_copied: false,
                    inode_marked_dirty: false,
                };
            }
            truncated = true;
        }
    }

    MinixSetattrOutcome {
        result: 0,
        newsize_checked,
        truncated,
        attrs_copied: true,
        inode_marked_dirty: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minix_file_operations_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/minix/file.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"minix.h\""));
        assert!(source.contains("int minix_fsync"));
        assert!(source.contains("return mmb_fsync(file,"));
        assert!(source.contains("&minix_i(file->f_mapping->host)->i_metadata_bhs"));
        assert!(source.contains(MINIX_FILE_OPERATIONS_SYMBOL));
        assert!(source.contains(MINIX_INODE_OPERATIONS_SYMBOL));
        for (slot, target) in MINIX_FILE_OPERATIONS
            .iter()
            .chain(MINIX_INODE_OPERATIONS.iter())
        {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
        assert!(source.contains("static int minix_setattr"));
        assert!(source.contains("setattr_prepare(&nop_mnt_idmap, dentry, attr);"));
        assert!(source.contains("inode_newsize_ok(inode, attr->ia_size);"));
        assert!(source.contains("truncate_setsize(inode, attr->ia_size);"));
        assert!(source.contains("minix_truncate(inode);"));
        assert!(source.contains("setattr_copy(&nop_mnt_idmap, inode, attr);"));
        assert!(source.contains("mark_inode_dirty(inode);"));

        assert_eq!(minix_setattr_outcome(-7, Some(4), 3, 0).result, -7);
        let unchanged = minix_setattr_outcome(0, Some(3), 3, 0);
        assert!(!unchanged.newsize_checked);
        assert!(!unchanged.truncated);
        assert!(unchanged.inode_marked_dirty);
        assert_eq!(minix_setattr_outcome(0, Some(4), 3, -9).result, -9);
        let changed = minix_setattr_outcome(0, Some(4), 3, 0);
        assert!(changed.newsize_checked);
        assert!(changed.truncated);
    }
}
