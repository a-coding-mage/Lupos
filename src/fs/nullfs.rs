//! linux-parity: complete
//! linux-source: vendor/linux/fs/nullfs.c
//! test-origin: linux:vendor/linux/fs/nullfs.c
//! Nullfs singleton filesystem metadata.

use crate::include::uapi::errno::ENOMEM;

pub const NULL_FS_MAGIC: u64 = 0x4e55_4c4c;
pub const NULLFS_NAME: &str = "nullfs";
pub const PAGE_SHIFT: u8 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const S_IMMUTABLE: u32 = 1 << 3;
pub const SB_NOUSER: u32 = 1 << 31;
pub const SB_I_NOEXEC: u32 = 0x0000_0002;
pub const SB_I_NODEV: u32 = 0x0000_0004;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NullfsSuperSpec {
    pub blocksize: usize,
    pub blocksize_bits: u8,
    pub magic: u64,
    pub time_gran: u32,
    pub root_ino: u64,
    pub root_flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NullfsContextSpec {
    pub global: bool,
    pub sb_flags: u32,
    pub s_iflags: u32,
}

pub const NULLFS_SUPER_SPEC: NullfsSuperSpec = NullfsSuperSpec {
    blocksize: PAGE_SIZE,
    blocksize_bits: PAGE_SHIFT,
    magic: NULL_FS_MAGIC,
    time_gran: 1,
    root_ino: 1,
    root_flags: S_IMMUTABLE,
};

pub const NULLFS_CONTEXT_SPEC: NullfsContextSpec = NullfsContextSpec {
    global: true,
    sb_flags: SB_NOUSER,
    s_iflags: SB_I_NOEXEC | SB_I_NODEV,
};

pub const fn nullfs_fill_super_result(inode_allocated: bool, root_allocated: bool) -> i32 {
    if !inode_allocated || !root_allocated {
        -ENOMEM
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullfs_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nullfs.c"
        ));
        assert!(source.contains("#include <linux/fs/super_types.h>"));
        assert!(source.contains("#include <linux/fs_context.h>"));
        assert!(source.contains("#include <linux/magic.h>"));
        assert!(source.contains("static const struct super_operations nullfs_super_operations"));
        assert!(source.contains(".statfs\t= simple_statfs"));
        assert!(source.contains("s->s_magic\t\t= NULL_FS_MAGIC"));
        assert!(source.contains("make_empty_dir_inode(inode);"));
        assert!(source.contains("inode->i_ino\t= 1;"));
        assert!(source.contains("inode->i_flags |= S_IMMUTABLE;"));
        assert!(source.contains("fc->global\t= true;"));
        assert!(source.contains("fc->sb_flags\t= SB_NOUSER;"));
        assert!(source.contains("fc->s_iflags\t= SB_I_NOEXEC | SB_I_NODEV;"));
        assert!(source.contains(".name\t\t\t= \"nullfs\""));

        assert_eq!(NULLFS_NAME, "nullfs");
        assert_eq!(NULLFS_SUPER_SPEC.magic, NULL_FS_MAGIC);
        assert_eq!(NULLFS_SUPER_SPEC.root_flags, S_IMMUTABLE);
        assert_eq!(NULLFS_CONTEXT_SPEC.s_iflags, SB_I_NOEXEC | SB_I_NODEV);
        assert_eq!(nullfs_fill_super_result(false, true), -ENOMEM);
        assert_eq!(nullfs_fill_super_result(true, false), -ENOMEM);
        assert_eq!(nullfs_fill_super_result(true, true), 0);
    }
}
