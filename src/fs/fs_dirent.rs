//! linux-parity: complete
//! linux-source: vendor/linux/fs/fs_dirent.c
//! test-origin: linux:vendor/linux/fs/fs_dirent.c
//! Generic fs on-disk file type and dirent type conversions.

use crate::include::uapi::stat::{
    S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK,
};

pub const S_DT_SHIFT: u32 = 12;
pub const S_DT_MASK: u32 = S_IFMT >> S_DT_SHIFT;

pub const DT_UNKNOWN: u8 = 0;
pub const DT_FIFO: u8 = 1;
pub const DT_CHR: u8 = 2;
pub const DT_DIR: u8 = 4;
pub const DT_BLK: u8 = 6;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;
pub const DT_SOCK: u8 = 12;
pub const DT_WHT: u8 = 14;
pub const DT_MAX: usize = (S_DT_MASK as usize) + 1;

pub const FT_UNKNOWN: u8 = 0;
pub const FT_REG_FILE: u8 = 1;
pub const FT_DIR: u8 = 2;
pub const FT_CHRDEV: u8 = 3;
pub const FT_BLKDEV: u8 = 4;
pub const FT_FIFO: u8 = 5;
pub const FT_SOCK: u8 = 6;
pub const FT_SYMLINK: u8 = 7;
pub const FT_MAX: usize = 8;

pub const FS_DTYPE_BY_FTYPE: [u8; FT_MAX] = [
    DT_UNKNOWN, DT_REG, DT_DIR, DT_CHR, DT_BLK, DT_FIFO, DT_SOCK, DT_LNK,
];

pub const FS_FTYPE_BY_DTYPE: [u8; DT_MAX] = [
    FT_UNKNOWN,
    FT_FIFO,
    FT_CHRDEV,
    FT_UNKNOWN,
    FT_DIR,
    FT_UNKNOWN,
    FT_BLKDEV,
    FT_UNKNOWN,
    FT_REG_FILE,
    FT_UNKNOWN,
    FT_SYMLINK,
    FT_UNKNOWN,
    FT_SOCK,
    FT_UNKNOWN,
    FT_UNKNOWN,
    FT_UNKNOWN,
];

pub const fn fs_ftype_to_dtype(filetype: u32) -> u8 {
    if filetype as usize >= FT_MAX {
        DT_UNKNOWN
    } else {
        FS_DTYPE_BY_FTYPE[filetype as usize]
    }
}

pub const fn s_dt(mode: u32) -> usize {
    ((mode & S_IFMT) >> S_DT_SHIFT) as usize
}

pub const fn fs_umode_to_ftype(mode: u32) -> u8 {
    FS_FTYPE_BY_DTYPE[s_dt(mode)]
}

pub const fn fs_umode_to_dtype(mode: u32) -> u8 {
    fs_ftype_to_dtype(fs_umode_to_ftype(mode) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_dirent_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/fs_dirent.c"
        ));
        assert!(source.contains("#include <linux/fs_dirent.h>"));
        assert!(source.contains("#include <linux/export.h>"));
        assert!(source.contains("static const unsigned char fs_dtype_by_ftype[FT_MAX]"));
        assert!(source.contains("[FT_UNKNOWN]\t= DT_UNKNOWN"));
        assert!(source.contains("[FT_REG_FILE]\t= DT_REG"));
        assert!(source.contains("[FT_DIR]\t= DT_DIR"));
        assert!(source.contains("[FT_CHRDEV]\t= DT_CHR"));
        assert!(source.contains("[FT_BLKDEV]\t= DT_BLK"));
        assert!(source.contains("[FT_FIFO]\t= DT_FIFO"));
        assert!(source.contains("[FT_SOCK]\t= DT_SOCK"));
        assert!(source.contains("[FT_SYMLINK]\t= DT_LNK"));
        assert!(source.contains("unsigned char fs_ftype_to_dtype(unsigned int filetype)"));
        assert!(source.contains("if (filetype >= FT_MAX)"));
        assert!(source.contains("return DT_UNKNOWN;"));
        assert!(source.contains("static const unsigned char fs_ftype_by_dtype[DT_MAX]"));
        assert!(source.contains("[DT_REG]\t= FT_REG_FILE"));
        assert!(source.contains("return fs_ftype_by_dtype[S_DT(mode)];"));
        assert!(source.contains("return fs_ftype_to_dtype(fs_umode_to_ftype(mode));"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fs_ftype_to_dtype);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fs_umode_to_dtype);"));

        assert_eq!(fs_ftype_to_dtype(FT_UNKNOWN as u32), DT_UNKNOWN);
        assert_eq!(fs_ftype_to_dtype(FT_REG_FILE as u32), DT_REG);
        assert_eq!(fs_ftype_to_dtype(FT_DIR as u32), DT_DIR);
        assert_eq!(fs_ftype_to_dtype(FT_CHRDEV as u32), DT_CHR);
        assert_eq!(fs_ftype_to_dtype(FT_BLKDEV as u32), DT_BLK);
        assert_eq!(fs_ftype_to_dtype(FT_FIFO as u32), DT_FIFO);
        assert_eq!(fs_ftype_to_dtype(FT_SOCK as u32), DT_SOCK);
        assert_eq!(fs_ftype_to_dtype(FT_SYMLINK as u32), DT_LNK);
        assert_eq!(fs_ftype_to_dtype(FT_MAX as u32), DT_UNKNOWN);
        assert_eq!(fs_umode_to_ftype(S_IFREG), FT_REG_FILE);
        assert_eq!(fs_umode_to_ftype(S_IFDIR), FT_DIR);
        assert_eq!(fs_umode_to_ftype(S_IFLNK), FT_SYMLINK);
        assert_eq!(fs_umode_to_ftype(S_IFCHR), FT_CHRDEV);
        assert_eq!(fs_umode_to_ftype(S_IFBLK), FT_BLKDEV);
        assert_eq!(fs_umode_to_ftype(S_IFIFO), FT_FIFO);
        assert_eq!(fs_umode_to_ftype(S_IFSOCK), FT_SOCK);
        assert_eq!(fs_umode_to_dtype(S_IFLNK), DT_LNK);
        assert_eq!(fs_umode_to_dtype(0), DT_UNKNOWN);
    }
}
