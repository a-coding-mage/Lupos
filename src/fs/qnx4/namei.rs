//! linux-parity: complete
//! linux-source: vendor/linux/fs/qnx4/namei.c
//! test-origin: linux:vendor/linux/fs/qnx4/namei.c
//! QNX4 lookup name and inode-number helpers.

pub const QNX4_BLOCK_SIZE: u64 = 0x200;
pub const QNX4_DIR_ENTRY_SIZE: u64 = 0x040;
pub const QNX4_INODES_PER_BLOCK: u64 = 0x08;
pub const QNX4_FILE_LINK: u8 = 0x08;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qnx4LookupHit {
    pub ino: u64,
    pub linked: bool,
}

pub fn qnx4_match(name: &[u8], entry_name: Option<&[u8]>) -> bool {
    let Some(fname) = entry_name else {
        return false;
    };
    name.len() == fname.len() && name == fname
}

pub const fn qnx4_regular_ino(block: u64, offset_after_entry: u64) -> u64 {
    block * QNX4_INODES_PER_BLOCK + (offset_after_entry / QNX4_DIR_ENTRY_SIZE) - 1
}

pub const fn qnx4_lookup_ino(
    block: u64,
    entry_index: u64,
    status: u8,
    link_inode_blk: u32,
    link_inode_ndx: u8,
) -> Qnx4LookupHit {
    if (status & QNX4_FILE_LINK) == QNX4_FILE_LINK {
        Qnx4LookupHit {
            ino: (link_inode_blk as u64 - 1) * QNX4_INODES_PER_BLOCK + link_inode_ndx as u64,
            linked: true,
        }
    } else {
        Qnx4LookupHit {
            ino: block * QNX4_INODES_PER_BLOCK + entry_index,
            linked: false,
        }
    }
}

pub const fn qnx4_scan_continues(dir_size: u64, blkofs: u64, offset: u64) -> bool {
    blkofs * QNX4_BLOCK_SIZE + offset < dir_size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qnx4_namei_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/qnx4/namei.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include \"qnx4.h\""));
        assert!(source.contains("static int qnx4_match(int len, const char *name,"));
        assert!(source.contains("if (bh == NULL)"));
        assert!(source.contains("*offset += QNX4_DIR_ENTRY_SIZE;"));
        assert!(source.contains("fname = get_entry_fname(de, &fnamelen);"));
        assert!(source.contains("if (!fname || len != fnamelen)"));
        assert!(source.contains("if (strncmp(name, fname, len) == 0)"));
        assert!(source.contains("static struct buffer_head *qnx4_find_entry"));
        assert!(source.contains("while (blkofs * QNX4_BLOCK_SIZE + offset < dir->i_size)"));
        assert!(source.contains("block = qnx4_block_map(dir, blkofs);"));
        assert!(source.contains("*ino = block * QNX4_INODES_PER_BLOCK +"));
        assert!(source.contains("struct dentry * qnx4_lookup"));
        assert!(source.contains("if ((de->di_status & QNX4_FILE_LINK) == QNX4_FILE_LINK)"));
        assert!(source.contains("ino = (le32_to_cpu(lnk->dl_inode_blk) - 1) *"));
        assert!(source.contains("foundinode = qnx4_iget(dir->i_sb, ino);"));
        assert!(source.contains("return d_splice_alias(foundinode, dentry);"));

        assert!(qnx4_match(b"name", Some(b"name")));
        assert!(!qnx4_match(b"name", Some(b"other")));
        assert!(!qnx4_match(b"name", None));
        assert_eq!(qnx4_regular_ino(10, QNX4_DIR_ENTRY_SIZE), 80);
        assert_eq!(
            qnx4_lookup_ino(10, 3, 0, 0, 0),
            Qnx4LookupHit {
                ino: 83,
                linked: false
            }
        );
        assert_eq!(
            qnx4_lookup_ino(10, 3, QNX4_FILE_LINK, 5, 2),
            Qnx4LookupHit {
                ino: 34,
                linked: true
            }
        );
        assert!(qnx4_scan_continues(1024, 1, 0));
        assert!(!qnx4_scan_continues(1024, 2, 0));
    }
}
