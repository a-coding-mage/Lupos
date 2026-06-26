//! linux-parity: complete
//! linux-source: vendor/linux/fs/ocfs2/symlink.c
//! test-origin: linux:vendor/linux/fs/ocfs2/symlink.c
//! OCFS2 fast symlink address-space and inode operations.

pub const OCFS2_FAST_SYMLINK_READ_FOLIO: &str = "ocfs2_fast_symlink_read_folio";
pub const OCFS2_FAST_SYMLINK_AOPS_SYMBOL: &str = "ocfs2_fast_symlink_aops";
pub const OCFS2_SYMLINK_INODE_OPS_SYMBOL: &str = "ocfs2_symlink_inode_operations";

pub const OCFS2_SYMLINK_INODE_OPS: &[&str] = &[
    "page_get_link",
    "ocfs2_getattr",
    "ocfs2_setattr",
    "ocfs2_listxattr",
    "ocfs2_fiemap",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ocfs2FastSymlinkRead<'a> {
    pub read_inode_status: i32,
    pub link: &'a [u8],
    pub fast_symlink_chars: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ocfs2FastSymlinkReadReport {
    pub read_inode_block_called: bool,
    pub logged_errno: Option<i32>,
    pub copied_to_folio: bool,
    pub copy_offset: usize,
    pub copy_len: usize,
    pub folio_end_read_success: bool,
    pub brelse_called: bool,
    pub returned_status: i32,
}

pub fn ocfs2_fast_symlink_copy_len(link: &[u8], max_chars: usize) -> usize {
    let limit = core::cmp::min(link.len(), max_chars);
    let len = link[..limit]
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(limit);
    len + 1
}

pub fn ocfs2_fast_symlink_read_folio_report(
    read: Ocfs2FastSymlinkRead<'_>,
) -> Ocfs2FastSymlinkReadReport {
    if read.read_inode_status < 0 {
        return Ocfs2FastSymlinkReadReport {
            read_inode_block_called: true,
            logged_errno: Some(read.read_inode_status),
            copied_to_folio: false,
            copy_offset: 0,
            copy_len: 0,
            folio_end_read_success: false,
            brelse_called: true,
            returned_status: read.read_inode_status,
        };
    }

    Ocfs2FastSymlinkReadReport {
        read_inode_block_called: true,
        logged_errno: None,
        copied_to_folio: true,
        copy_offset: 0,
        copy_len: ocfs2_fast_symlink_copy_len(read.link, read.fast_symlink_chars),
        folio_end_read_success: read.read_inode_status == 0,
        brelse_called: true,
        returned_status: read.read_inode_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocfs2_symlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ocfs2/symlink.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("#include \"ocfs2.h\""));
        assert!(source.contains("#include \"symlink.h\""));
        assert!(source.contains(OCFS2_FAST_SYMLINK_READ_FOLIO));
        assert!(source.contains("status = ocfs2_read_inode_block(inode, &bh);"));
        assert!(source.contains("if (status < 0)"));
        assert!(source.contains("mlog_errno(status);"));
        assert!(source.contains("goto out;"));
        assert!(source.contains("fe = (struct ocfs2_dinode *) bh->b_data;"));
        assert!(source.contains("link = (char *) fe->id2.i_symlink;"));
        assert!(source.contains("/* will be less than a page size */"));
        assert!(source.contains("len = strnlen(link, ocfs2_fast_symlink_chars(inode->i_sb));"));
        assert!(source.contains("memcpy_to_folio(folio, 0, link, len + 1);"));
        assert!(source.contains("out:"));
        assert!(source.contains("folio_end_read(folio, status == 0);"));
        assert!(source.contains("brelse(bh);"));
        assert!(source.contains("return status;"));
        assert!(source.contains(OCFS2_FAST_SYMLINK_AOPS_SYMBOL));
        assert!(source.contains(".read_folio\t\t= ocfs2_fast_symlink_read_folio"));
        assert!(source.contains(OCFS2_SYMLINK_INODE_OPS_SYMBOL));
        for op in OCFS2_SYMLINK_INODE_OPS {
            assert!(source.contains(op));
        }

        assert_eq!(ocfs2_fast_symlink_copy_len(b"target\0ignored", 32), 7);
        assert_eq!(ocfs2_fast_symlink_copy_len(b"target", 3), 4);
    }

    #[test]
    fn read_folio_report_matches_success_path() {
        assert_eq!(
            ocfs2_fast_symlink_read_folio_report(Ocfs2FastSymlinkRead {
                read_inode_status: 0,
                link: b"target\0ignored",
                fast_symlink_chars: 32,
            }),
            Ocfs2FastSymlinkReadReport {
                read_inode_block_called: true,
                logged_errno: None,
                copied_to_folio: true,
                copy_offset: 0,
                copy_len: 7,
                folio_end_read_success: true,
                brelse_called: true,
                returned_status: 0,
            }
        );
    }

    #[test]
    fn read_folio_report_matches_error_path() {
        assert_eq!(
            ocfs2_fast_symlink_read_folio_report(Ocfs2FastSymlinkRead {
                read_inode_status: -5,
                link: b"target\0ignored",
                fast_symlink_chars: 32,
            }),
            Ocfs2FastSymlinkReadReport {
                read_inode_block_called: true,
                logged_errno: Some(-5),
                copied_to_folio: false,
                copy_offset: 0,
                copy_len: 0,
                folio_end_read_success: false,
                brelse_called: true,
                returned_status: -5,
            }
        );
    }

    #[test]
    fn read_folio_report_copies_but_marks_positive_status_unsuccessful() {
        assert_eq!(
            ocfs2_fast_symlink_read_folio_report(Ocfs2FastSymlinkRead {
                read_inode_status: 1,
                link: b"abc\0ignored",
                fast_symlink_chars: 32,
            }),
            Ocfs2FastSymlinkReadReport {
                read_inode_block_called: true,
                logged_errno: None,
                copied_to_folio: true,
                copy_offset: 0,
                copy_len: 4,
                folio_end_read_success: false,
                brelse_called: true,
                returned_status: 1,
            }
        );
    }
}
