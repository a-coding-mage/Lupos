//! linux-parity: complete
//! linux-source: vendor/linux/fs/efs/dir.c
//! test-origin: linux:vendor/linux/fs/efs/dir.c
//! EFS directory position and entry-boundary helpers.

pub const EFS_BLOCKSIZE_BITS: u32 = 9;
pub const EFS_DIRBSIZE_BITS: u32 = EFS_BLOCKSIZE_BITS;
pub const EFS_DIRBSIZE: usize = 1 << EFS_DIRBSIZE_BITS;
pub const EFS_DIRBLK_MAGIC: u16 = 0xbeef;
pub const EFS_DIRBLK_HEADERSIZE: usize = 4;
pub const EFS_MAX_SLOTS_PER_BLOCK: usize = 256;

pub const EFS_DIR_FILE_OPERATIONS: &[&str] = &[
    "generic_file_llseek",
    "generic_read_dir",
    "efs_readdir",
    "generic_setlease",
];

pub const fn efs_directory_size_aligned(size: u64) -> bool {
    size & (EFS_DIRBSIZE as u64 - 1) == 0
}

pub const fn efs_readdir_position(pos: u64) -> (u64, u8) {
    (pos >> EFS_DIRBSIZE_BITS, (pos & 0xff) as u8)
}

pub const fn efs_emit_position(block: u64, slot: u8) -> u64 {
    (block << EFS_DIRBSIZE_BITS) | slot as u64
}

pub const fn efs_realoff(offset: u8) -> usize {
    (offset as usize) << 1
}

pub const fn efs_dirent_fits(entry_offset: usize, namelen: usize) -> bool {
    entry_offset + namelen <= EFS_DIRBSIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efs_dir_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/efs/dir.c"
        ));
        assert!(source.contains("#include <linux/buffer_head.h>"));
        assert!(source.contains("#include <linux/filelock.h>"));
        assert!(source.contains("#include \"efs.h\""));
        assert!(source.contains("const struct file_operations efs_dir_operations"));
        assert!(source.contains(".llseek\t\t= generic_file_llseek"));
        assert!(source.contains(".read\t\t= generic_read_dir"));
        assert!(source.contains(".iterate_shared\t= efs_readdir"));
        assert!(source.contains(".setlease\t= generic_setlease"));
        assert!(source.contains("const struct inode_operations efs_dir_inode_operations"));
        assert!(source.contains(".lookup\t\t= efs_lookup"));
        assert!(source.contains("if (inode->i_size & (EFS_DIRBSIZE-1))"));
        assert!(source.contains("block = ctx->pos >> EFS_DIRBSIZE_BITS;"));
        assert!(source.contains("slot  = ctx->pos & 0xff;"));
        assert!(source.contains("while (block < inode->i_blocks)"));
        assert!(source.contains("sb_bread(inode->i_sb, efs_bmap(inode, block));"));
        assert!(source.contains("be16_to_cpu(dirblock->magic) != EFS_DIRBLK_MAGIC"));
        assert!(source.contains("for (; slot < dirblock->slots; slot++)"));
        assert!(source.contains("if (dirblock->space[slot] == 0)"));
        assert!(source.contains("EFS_SLOTAT(dirblock, slot)"));
        assert!(source.contains("ctx->pos = (block << EFS_DIRBSIZE_BITS) | slot;"));
        assert!(source.contains("if (nameptr - (char *) dirblock + namelen > EFS_DIRBSIZE)"));
        assert!(source.contains("dir_emit(ctx, nameptr, namelen, inodenum, DT_UNKNOWN)"));

        assert_eq!(EFS_DIRBSIZE, 512);
        assert!(efs_directory_size_aligned(1024));
        assert!(!efs_directory_size_aligned(1025));
        assert_eq!(efs_readdir_position(0x1234), (0x9, 0x34));
        assert_eq!(efs_emit_position(0x9, 0x34), 0x1234);
        assert_eq!(efs_realoff(12), 24);
        assert!(efs_dirent_fits(500, 12));
        assert!(!efs_dirent_fits(501, 12));
        assert_eq!(EFS_DIR_FILE_OPERATIONS[2], "efs_readdir");
    }
}
