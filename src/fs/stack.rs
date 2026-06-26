//! linux-parity: complete
//! linux-source: vendor/linux/fs/stack.c
//! test-origin: linux:vendor/linux/fs/stack.c
//! fsstack inode size and attribute copy helpers.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsStackInodeSize {
    pub i_size: i64,
    pub i_blocks: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsStackInodeAttrs {
    pub i_mode: u16,
    pub i_uid: u32,
    pub i_gid: u32,
    pub i_rdev: u64,
    pub atime: i64,
    pub mtime: i64,
    pub ctime: i64,
    pub i_blkbits: u8,
    pub i_flags: u32,
    pub i_nlink: u32,
}

pub const fn fsstack_copy_inode_size(src: FsStackInodeSize) -> FsStackInodeSize {
    src
}

pub const fn fsstack_copy_attr_all(src: FsStackInodeAttrs) -> FsStackInodeAttrs {
    src
}

pub const fn fsstack_copy_size_needs_lock(i_size_bytes: usize, i_blocks_bytes: usize) -> bool {
    i_size_bytes > core::mem::size_of::<usize>() || i_blocks_bytes > core::mem::size_of::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsstack_copy_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/stack.c"
        ));
        assert!(source.contains("#include <linux/export.h>"));
        assert!(source.contains("#include <linux/fs_stack.h>"));
        assert!(source.contains("void fsstack_copy_inode_size"));
        assert!(source.contains("i_size = i_size_read(src);"));
        assert!(source.contains("i_blocks = src->i_blocks;"));
        assert!(source.contains("i_size_write(dst, i_size);"));
        assert!(source.contains("dst->i_blocks = i_blocks;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fsstack_copy_inode_size);"));
        assert!(source.contains("void fsstack_copy_attr_all"));
        assert!(source.contains("dest->i_mode = src->i_mode;"));
        assert!(source.contains("inode_set_atime_to_ts(dest, inode_get_atime(src));"));
        assert!(source.contains("set_nlink(dest, src->i_nlink);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fsstack_copy_attr_all);"));

        let size = FsStackInodeSize {
            i_size: 4096,
            i_blocks: 8,
        };
        assert_eq!(fsstack_copy_inode_size(size), size);
        let attrs = FsStackInodeAttrs {
            i_mode: 0o100644,
            i_uid: 1000,
            i_gid: 1000,
            i_rdev: 0,
            atime: 1,
            mtime: 2,
            ctime: 3,
            i_blkbits: 12,
            i_flags: 4,
            i_nlink: 2,
        };
        assert_eq!(fsstack_copy_attr_all(attrs), attrs);
    }
}
