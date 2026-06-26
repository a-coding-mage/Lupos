//! linux-parity: complete
//! linux-source: vendor/linux/fs/qnx4/dir.c
//! test-origin: linux:vendor/linux/fs/qnx4/dir.c
//! QNX4 directory iteration constants and inode calculation.

extern crate alloc;

use alloc::vec::Vec;

pub const QNX4_DIR_ENTRY_SIZE: u64 = 0x040;
pub const QNX4_DIR_ENTRY_SIZE_BITS: u8 = 6;
pub const QNX4_INODES_PER_BLOCK: u64 = 0x08;
pub const QNX4_FILE_LINK: u8 = 0x08;
pub const QNX4_DIR_OPERATIONS_SYMBOL: &str = "qnx4_dir_operations";
pub const QNX4_DIR_INODE_OPERATIONS_SYMBOL: &str = "qnx4_dir_inode_operations";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qnx4DirectoryEntry<'a> {
    pub status: u8,
    pub name: Option<&'a str>,
    pub link_inode_blk: u32,
    pub link_inode_ndx: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qnx4DirEmit<'a> {
    pub name: &'a str,
    pub ino: i64,
}

pub const fn qnx4_entry_inode(
    blknum: u64,
    ix: u64,
    status: u8,
    link_blk: u32,
    link_ndx: u8,
) -> i64 {
    if status & QNX4_FILE_LINK == 0 {
        (blknum * QNX4_INODES_PER_BLOCK + ix) as i64 - 1
    } else {
        ((link_blk as u64 - 1) * QNX4_INODES_PER_BLOCK + link_ndx as u64) as i64
    }
}

pub fn qnx4_readdir_block<'a>(
    blknum: u64,
    start_ix: usize,
    entries: &'a [Qnx4DirectoryEntry<'a>],
) -> Vec<Qnx4DirEmit<'a>> {
    let mut out = Vec::new();
    for (ix, entry) in entries
        .iter()
        .enumerate()
        .skip(start_ix)
        .take(QNX4_INODES_PER_BLOCK as usize)
    {
        let Some(name) = entry.name else {
            continue;
        };
        out.push(Qnx4DirEmit {
            name,
            ino: qnx4_entry_inode(
                blknum,
                ix as u64,
                entry.status,
                entry.link_inode_blk,
                entry.link_inode_ndx,
            ),
        });
    }
    out
}

pub const fn qnx4_readdir_start_ix(pos: u64) -> usize {
    ((pos >> QNX4_DIR_ENTRY_SIZE_BITS) % QNX4_INODES_PER_BLOCK) as usize
}

pub const fn qnx4_readdir_next_pos(pos: u64) -> u64 {
    pos + QNX4_DIR_ENTRY_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qnx4_dir_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/qnx4/dir.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include <linux/filelock.h>"));
        assert!(source.contains("#include \"qnx4.h\""));
        assert!(source.contains("static int qnx4_readdir"));
        assert!(source.contains("ctx->pos < inode->i_size"));
        assert!(source.contains("qnx4_block_map(inode, ctx->pos >> QNX4_BLOCK_SIZE_BITS)"));
        assert!(source.contains("ix = (ctx->pos >> QNX4_DIR_ENTRY_SIZE_BITS)"));
        assert!(source.contains("get_entry_fname(de, &size);"));
        assert!(source.contains("de->de_status & QNX4_FILE_LINK"));
        assert!(source.contains("dir_emit(ctx, fname, size, ino, DT_UNKNOWN)"));
        assert!(source.contains(QNX4_DIR_OPERATIONS_SYMBOL));
        assert!(source.contains(".iterate_shared\t= qnx4_readdir"));
        assert!(source.contains(QNX4_DIR_INODE_OPERATIONS_SYMBOL));

        assert_eq!(qnx4_readdir_start_ix(0x80), 2);
        assert_eq!(qnx4_readdir_next_pos(0x80), 0xc0);
        assert_eq!(qnx4_entry_inode(10, 3, 0, 0, 0), 82);
        assert_eq!(qnx4_entry_inode(10, 3, QNX4_FILE_LINK, 20, 2), 154);

        let entries = [
            Qnx4DirectoryEntry {
                status: 0,
                name: Some("a"),
                link_inode_blk: 0,
                link_inode_ndx: 0,
            },
            Qnx4DirectoryEntry {
                status: QNX4_FILE_LINK,
                name: Some("b"),
                link_inode_blk: 3,
                link_inode_ndx: 4,
            },
        ];
        let emitted = qnx4_readdir_block(2, 0, &entries);
        assert_eq!(emitted[0].ino, 15);
        assert_eq!(emitted[1].ino, 20);
    }
}
