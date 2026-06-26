//! linux-parity: complete
//! linux-source: vendor/linux/mm/fadvise.c
//! test-origin: linux:vendor/linux/mm/fadvise.c
//! Generic `fadvise` validation and page-range effects.

use crate::include::uapi::errno::{EBADF, EINVAL, ESPIPE};

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);

pub const POSIX_FADV_NORMAL: i32 = 0;
pub const POSIX_FADV_RANDOM: i32 = 1;
pub const POSIX_FADV_SEQUENTIAL: i32 = 2;
pub const POSIX_FADV_WILLNEED: i32 = 3;
pub const POSIX_FADV_DONTNEED: i32 = 4;
pub const POSIX_FADV_NOREUSE: i32 = 5;

pub const FMODE_RANDOM: u32 = 1 << 12;
pub const FMODE_NOREUSE: u32 = 1 << 23;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FadviseFile {
    pub is_fifo: bool,
    pub has_mapping: bool,
    pub is_dax: bool,
    pub noop_bdi: bool,
    pub bdi_ra_pages: u64,
    pub file_ra_pages: u64,
    pub f_mode: u32,
    pub inode_size: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FadviseAction {
    None,
    Readahead {
        start_index: u64,
        nrpages: u64,
    },
    DontNeed {
        flush_offset: u64,
        flush_endbyte: u64,
        start_index: u64,
        end_index: u64,
        invalidate: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FadviseOutcome {
    pub file: FadviseFile,
    pub endbyte: u64,
    pub action: FadviseAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FadviseFd {
    pub open: bool,
    pub file: FadviseFile,
}

pub fn generic_fadvise(
    mut file: FadviseFile,
    offset: i64,
    len: i64,
    advice: i32,
) -> Result<FadviseOutcome, i32> {
    if file.is_fifo {
        return Err(-ESPIPE);
    }
    if !file.has_mapping || len < 0 || offset < 0 {
        return Err(-EINVAL);
    }
    if file.is_dax || file.noop_bdi {
        return if valid_advice(advice) {
            Ok(FadviseOutcome {
                file,
                endbyte: 0,
                action: FadviseAction::None,
            })
        } else {
            Err(-EINVAL)
        };
    }

    let endbyte = inclusive_endbyte(offset as u64, len as u64);
    let action = match advice {
        POSIX_FADV_NORMAL => {
            file.file_ra_pages = file.bdi_ra_pages;
            file.f_mode &= !(FMODE_RANDOM | FMODE_NOREUSE);
            FadviseAction::None
        }
        POSIX_FADV_RANDOM => {
            file.f_mode |= FMODE_RANDOM;
            FadviseAction::None
        }
        POSIX_FADV_SEQUENTIAL => {
            file.file_ra_pages = file.bdi_ra_pages.saturating_mul(2);
            file.f_mode &= !FMODE_RANDOM;
            FadviseAction::None
        }
        POSIX_FADV_WILLNEED => {
            let start_index = offset as u64 >> PAGE_SHIFT;
            let end_index = endbyte >> PAGE_SHIFT;
            let mut nrpages = end_index.wrapping_sub(start_index).wrapping_add(1);
            if nrpages == 0 {
                nrpages = u64::MAX;
            }
            FadviseAction::Readahead {
                start_index,
                nrpages,
            }
        }
        POSIX_FADV_NOREUSE => {
            file.f_mode |= FMODE_NOREUSE;
            FadviseAction::None
        }
        POSIX_FADV_DONTNEED => dontneed_action(offset as u64, endbyte, file.inode_size),
        _ => return Err(-EINVAL),
    };

    Ok(FadviseOutcome {
        file,
        endbyte,
        action,
    })
}

pub fn vfs_fadvise(
    file: FadviseFile,
    offset: i64,
    len: i64,
    advice: i32,
    file_op_result: Option<Result<FadviseOutcome, i32>>,
) -> Result<FadviseOutcome, i32> {
    if let Some(result) = file_op_result {
        return result;
    }
    generic_fadvise(file, offset, len, advice)
}

pub fn ksys_fadvise64_64(
    fd: Option<FadviseFd>,
    offset: i64,
    len: i64,
    advice: i32,
) -> Result<FadviseOutcome, i32> {
    let Some(fd) = fd else {
        return Err(-EBADF);
    };
    if !fd.open {
        return Err(-EBADF);
    }
    vfs_fadvise(fd.file, offset, len, advice, None)
}

pub fn sys_fadvise64_64(
    fd: Option<FadviseFd>,
    offset: i64,
    len: i64,
    advice: i32,
) -> Result<FadviseOutcome, i32> {
    ksys_fadvise64_64(fd, offset, len, advice)
}

pub fn sys_fadvise64(
    fd: Option<FadviseFd>,
    offset: i64,
    len: usize,
    advice: i32,
) -> Result<FadviseOutcome, i32> {
    ksys_fadvise64_64(fd, offset, len as i64, advice)
}

pub const fn compat_arg_u64_glue(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

pub fn compat_sys_fadvise64_64(
    fd: Option<FadviseFd>,
    offset_hi: u32,
    offset_lo: u32,
    len_hi: u32,
    len_lo: u32,
    advice: i32,
) -> Result<FadviseOutcome, i32> {
    ksys_fadvise64_64(
        fd,
        compat_arg_u64_glue(offset_hi, offset_lo) as i64,
        compat_arg_u64_glue(len_hi, len_lo) as i64,
        advice,
    )
}

pub const fn valid_advice(advice: i32) -> bool {
    matches!(
        advice,
        POSIX_FADV_NORMAL
            | POSIX_FADV_RANDOM
            | POSIX_FADV_SEQUENTIAL
            | POSIX_FADV_WILLNEED
            | POSIX_FADV_NOREUSE
            | POSIX_FADV_DONTNEED
    )
}

pub const fn inclusive_endbyte(offset: u64, len: u64) -> u64 {
    let endbyte = offset.wrapping_add(len);
    if len == 0 || endbyte < len {
        i64::MAX as u64
    } else {
        endbyte - 1
    }
}

pub const fn dontneed_action(offset: u64, endbyte: u64, inode_size: u64) -> FadviseAction {
    let start_index = (offset + (PAGE_SIZE - 1)) >> PAGE_SHIFT;
    let mut end_index = endbyte >> PAGE_SHIFT;

    if (endbyte & !PAGE_MASK) != !PAGE_MASK && endbyte != inode_size.wrapping_sub(1) {
        if end_index == 0 {
            return FadviseAction::DontNeed {
                flush_offset: offset,
                flush_endbyte: endbyte,
                start_index,
                end_index,
                invalidate: false,
            };
        }
        end_index -= 1;
    }

    FadviseAction::DontNeed {
        flush_offset: offset,
        flush_endbyte: endbyte,
        start_index,
        end_index,
        invalidate: end_index >= start_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> FadviseFile {
        FadviseFile {
            is_fifo: false,
            has_mapping: true,
            is_dax: false,
            noop_bdi: false,
            bdi_ra_pages: 128,
            file_ra_pages: 0,
            f_mode: FMODE_RANDOM | FMODE_NOREUSE,
            inode_size: 0x10_000,
        }
    }

    #[test]
    fn fadvise_branches_match_linux_source_and_uapi() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/fadvise.c"
        ));
        let uapi = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/fadvise.h"
        ));
        let fs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/fs.h"
        ));

        assert!(source.contains("if (S_ISFIFO(inode->i_mode))"));
        assert!(source.contains("if (!mapping || len < 0 || offset < 0)"));
        assert!(source.contains("if (IS_DAX(inode) || (bdi == &noop_backing_dev_info))"));
        assert!(source.contains("endbyte = (u64)offset + (u64)len;"));
        assert!(source.contains("if (!len || endbyte < len)"));
        assert!(source.contains("file->f_ra.ra_pages = bdi->ra_pages * 2;"));
        assert!(
            source.contains("force_page_cache_readahead(mapping, file, start_index, nrpages);")
        );
        assert!(source.contains("mapping_try_invalidate(mapping, start_index, end_index"));
        assert!(source.contains("int vfs_fadvise(struct file *file, loff_t offset"));
        assert!(source.contains("if (file->f_op->fadvise)"));
        assert!(source.contains("return file->f_op->fadvise(file, offset, len, advice);"));
        assert!(source.contains("return generic_fadvise(file, offset, len, advice);"));
        assert!(source.contains("int ksys_fadvise64_64(int fd, loff_t offset"));
        assert!(source.contains("if (fd_empty(f))"));
        assert!(source.contains("return -EBADF;"));
        assert!(source.contains("SYSCALL_DEFINE4(fadvise64_64"));
        assert!(source.contains("SYSCALL_DEFINE4(fadvise64"));
        assert!(source.contains("COMPAT_SYSCALL_DEFINE6(fadvise64_64"));
        assert!(source.contains("compat_arg_u64_glue(offset)"));
        assert!(uapi.contains("#define POSIX_FADV_DONTNEED\t4"));
        assert!(uapi.contains("#define POSIX_FADV_NOREUSE\t5"));
        assert!(fs.contains("#define FMODE_RANDOM"));
        assert!(fs.contains("#define\tFMODE_NOREUSE"));

        assert_eq!(POSIX_FADV_DONTNEED, 4);
        assert_eq!(POSIX_FADV_NOREUSE, 5);
        assert_eq!(FMODE_RANDOM, 1 << 12);
        assert_eq!(FMODE_NOREUSE, 1 << 23);
    }

    #[test]
    fn generic_fadvise_updates_file_state_and_ranges() {
        let normal = generic_fadvise(file(), 0, 4096, POSIX_FADV_NORMAL).unwrap();
        assert_eq!(normal.file.file_ra_pages, 128);
        assert_eq!(normal.file.f_mode & (FMODE_RANDOM | FMODE_NOREUSE), 0);

        let random = generic_fadvise(file(), 0, 4096, POSIX_FADV_RANDOM).unwrap();
        assert_ne!(random.file.f_mode & FMODE_RANDOM, 0);

        let sequential = generic_fadvise(file(), 0, 4096, POSIX_FADV_SEQUENTIAL).unwrap();
        assert_eq!(sequential.file.file_ra_pages, 256);
        assert_eq!(sequential.file.f_mode & FMODE_RANDOM, 0);

        let willneed = generic_fadvise(file(), 0x1000, 0x3000, POSIX_FADV_WILLNEED).unwrap();
        assert_eq!(
            willneed.action,
            FadviseAction::Readahead {
                start_index: 1,
                nrpages: 3,
            }
        );

        let dontneed =
            generic_fadvise(file(), 1, PAGE_SIZE as i64 * 3, POSIX_FADV_DONTNEED).unwrap();
        assert_eq!(
            dontneed.action,
            FadviseAction::DontNeed {
                flush_offset: 1,
                flush_endbyte: PAGE_SIZE * 3,
                start_index: 1,
                end_index: 2,
                invalidate: true,
            }
        );

        assert_eq!(
            generic_fadvise(
                FadviseFile {
                    is_fifo: true,
                    ..file()
                },
                0,
                1,
                POSIX_FADV_NORMAL,
            ),
            Err(-ESPIPE)
        );
        assert_eq!(
            generic_fadvise(file(), -1, 1, POSIX_FADV_NORMAL),
            Err(-EINVAL)
        );
        assert_eq!(generic_fadvise(file(), 0, 1, 99), Err(-EINVAL));
    }

    #[test]
    fn vfs_and_syscall_wrappers_follow_linux_dispatch() {
        let fd = FadviseFd {
            open: true,
            file: file(),
        };
        let direct = vfs_fadvise(file(), 0, 4096, POSIX_FADV_RANDOM, None).unwrap();
        assert_ne!(direct.file.f_mode & FMODE_RANDOM, 0);

        let override_outcome = FadviseOutcome {
            file: file(),
            endbyte: 123,
            action: FadviseAction::None,
        };
        assert_eq!(
            vfs_fadvise(
                file(),
                0,
                4096,
                POSIX_FADV_RANDOM,
                Some(Ok(override_outcome))
            ),
            Ok(override_outcome)
        );
        assert_eq!(
            vfs_fadvise(file(), 0, 4096, POSIX_FADV_RANDOM, Some(Err(-EINVAL))),
            Err(-EINVAL)
        );

        assert_eq!(
            ksys_fadvise64_64(None, 0, 1, POSIX_FADV_NORMAL),
            Err(-EBADF)
        );
        assert_eq!(
            ksys_fadvise64_64(
                Some(FadviseFd {
                    open: false,
                    file: file(),
                }),
                0,
                1,
                POSIX_FADV_NORMAL,
            ),
            Err(-EBADF)
        );
        assert!(sys_fadvise64_64(Some(fd), 0, 1, POSIX_FADV_NORMAL).is_ok());
        assert!(sys_fadvise64(Some(fd), 0, 1, POSIX_FADV_NORMAL).is_ok());
        assert_eq!(compat_arg_u64_glue(0x1234, 0x5678), 0x1234_0000_5678);
        let compat =
            compat_sys_fadvise64_64(Some(fd), 0, 0x1000, 0, 0x1000, POSIX_FADV_WILLNEED).unwrap();
        assert_eq!(
            compat.action,
            FadviseAction::Readahead {
                start_index: 1,
                nrpages: 1,
            }
        );
    }
}
