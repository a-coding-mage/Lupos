//! linux-parity: partial
//! linux-source: vendor/linux/fs/9p/vfs_file.c
//! test-origin: linux:vendor/linux/fs/9p/vfs_file.c
//! 9P file open, cache path, lock status, fsync, and mmap decisions.

use crate::include::uapi::errno::{EAGAIN, EINVAL, ENOLCK, EOPNOTSUPP};
use crate::include::uapi::fcntl::{O_APPEND, O_WRONLY};

use super::types::*;
use super::vfs_inode::v9fs_uflags2omode;
use super::vfs_inode_dotl::v9fs_open_to_dotl_flags;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileOpenPlan {
    pub omode: u32,
    pub writeback_retry_omode: Option<u32>,
    pub append_bit: u32,
    pub seek_end_for_legacy_append: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachedIoPath {
    Cached,
    Unbuffered,
    CopySplice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MmapPrepare {
    ReadOnly,
    WritebackWithVmOps,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LockRange {
    pub start: u64,
    pub length: u64,
}

pub const OFFSET_MAX: u64 = i64::MAX as u64;
pub const VM_SHARED: u32 = 0x0000_0008;
pub const PAGE_SIZE: u64 = 4096;

pub fn v9fs_file_open_plan(session_flags: u32, cache: u32, file_flags: u32) -> FileOpenPlan {
    let (omode, append_bit) = if proto_dotl(session_flags) {
        (v9fs_open_to_dotl_flags(file_flags), P9_DOTL_APPEND)
    } else {
        (
            v9fs_uflags2omode(file_flags, proto_dotu(session_flags)),
            P9_OAPPEND,
        )
    };
    let writeback_retry_omode = if cache & CACHE_WRITEBACK != 0 && omode & P9_OWRITE != 0 {
        Some((omode & !(P9_OWRITE | append_bit)) | P9_ORDWR)
    } else {
        None
    };
    FileOpenPlan {
        omode,
        writeback_retry_omode,
        append_bit,
        seek_end_for_legacy_append: file_flags & O_APPEND != 0
            && !proto_dotu(session_flags)
            && !proto_dotl(session_flags),
    }
}

pub const fn read_iter_path(fid_mode: u32) -> CachedIoPath {
    if fid_mode & P9L_DIRECT != 0 {
        CachedIoPath::Unbuffered
    } else {
        CachedIoPath::Cached
    }
}

pub const fn splice_read_path(fid_mode: u32) -> CachedIoPath {
    if fid_mode & P9L_DIRECT != 0 {
        CachedIoPath::CopySplice
    } else {
        CachedIoPath::Cached
    }
}

pub const fn write_iter_path(fid_mode: u32) -> CachedIoPath {
    if fid_mode & (P9L_DIRECT | P9L_NOWRITECACHE) != 0 {
        CachedIoPath::Unbuffered
    } else {
        CachedIoPath::Cached
    }
}

pub const fn dotl_lock_status_to_errno(status: u8) -> i32 {
    match status {
        P9_LOCK_SUCCESS => 0,
        P9_LOCK_BLOCKED => -EAGAIN,
        P9_LOCK_ERROR | P9_LOCK_GRACE => -ENOLCK,
        _ => -ENOLCK,
    }
}

pub const fn lock_range(start: u64, end: u64) -> LockRange {
    LockRange {
        start,
        length: if end == OFFSET_MAX {
            0
        } else {
            end - start + 1
        },
    }
}

pub const fn getlock_update_range(server_type: u8, start: u64, length: u64) -> Option<(u64, u64)> {
    if server_type == P9_LOCK_TYPE_UNLCK {
        None
    } else if length == 0 {
        Some((start, OFFSET_MAX))
    } else {
        Some((start, start + length - 1))
    }
}

pub const fn flock_precheck(flags_has_flock: bool) -> Result<(), i32> {
    if flags_has_flock {
        Ok(())
    } else {
        Err(-ENOLCK)
    }
}

pub const fn mmap_prepare(cache: u32) -> MmapPrepare {
    if cache & CACHE_WRITEBACK == 0 {
        MmapPrepare::ReadOnly
    } else {
        MmapPrepare::WritebackWithVmOps
    }
}

pub const fn mmap_close_flush_range(
    vm_flags: u32,
    vm_pgoff: u64,
    vm_start: u64,
    vm_end: u64,
) -> Option<(u64, u64)> {
    if vm_flags & VM_SHARED == 0 {
        None
    } else {
        let start = vm_pgoff * PAGE_SIZE;
        Some((start, start + (vm_end - vm_start - 1)))
    }
}

pub const fn fsync_result(write_and_wait: i32, remote_fsync: i32) -> i32 {
    if write_and_wait != 0 {
        write_and_wait
    } else {
        remote_fsync
    }
}

pub const fn file_lock_command_valid(
    is_setlk: bool,
    is_setlkw: bool,
    is_getlk: bool,
) -> Result<(), i32> {
    if is_setlk || is_setlkw || is_getlk {
        Ok(())
    } else {
        Err(-EINVAL)
    }
}

pub const fn symlink_mmap_allowed() -> Result<(), i32> {
    Err(-EOPNOTSUPP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/vfs_file.c"
        ));
        assert!(source.contains("int v9fs_file_open(struct inode *inode, struct file *file)"));
        assert!(source.contains("omode = v9fs_open_to_dotl_flags(file->f_flags);"));
        assert!(source.contains("o_append = P9_DOTL_APPEND;"));
        assert!(source.contains("write-only file with writeback enabled"));
        assert!(source.contains("fid->mode |= P9L_DIRECT;"));
        assert!(source.contains("generic_file_llseek(file, 0, SEEK_END);"));
        assert!(source.contains("static int v9fs_file_do_lock"));
        assert!(source.contains("case P9_LOCK_SUCCESS:"));
        assert!(source.contains("res = -EAGAIN;"));
        assert!(source.contains("res = -ENOLCK;"));
        assert!(source.contains("if (fl->fl_end == OFFSET_MAX)"));
        assert!(source.contains("v9fs_file_read_iter(struct kiocb *iocb"));
        assert!(source.contains("return netfs_unbuffered_read_iter"));
        assert!(source.contains("static ssize_t v9fs_file_splice_read"));
        assert!(source.contains("copy_splice_read"));
        assert!(source.contains("v9fs_file_write_iter(struct kiocb *iocb"));
        assert!(source.contains("P9L_DIRECT | P9L_NOWRITECACHE"));
        assert!(source.contains("int v9fs_file_fsync_dotl"));
        assert!(source.contains("v9fs_file_mmap_prepare(struct vm_area_desc *desc)"));
        assert!(source.contains("if (!(v9ses->cache & CACHE_WRITEBACK))"));
        assert!(source.contains("static void v9fs_mmap_vm_close"));
        assert!(source.contains("if (!(vma->vm_flags & VM_SHARED))"));
        assert!(source.contains("const struct file_operations v9fs_file_operations_dotl"));

        let plan = v9fs_file_open_plan(V9FS_PROTO_2000L, CACHE_WRITEBACK, O_WRONLY | O_APPEND);
        assert_eq!(plan.omode, P9_OWRITE | P9_DOTL_APPEND);
        assert_eq!(plan.writeback_retry_omode, Some(P9_ORDWR));
        assert_eq!(plan.append_bit, P9_DOTL_APPEND);
        assert!(!plan.seek_end_for_legacy_append);

        let legacy = v9fs_file_open_plan(0, 0, O_APPEND);
        assert!(legacy.seek_end_for_legacy_append);
        assert_eq!(read_iter_path(P9L_DIRECT), CachedIoPath::Unbuffered);
        assert_eq!(splice_read_path(P9L_DIRECT), CachedIoPath::CopySplice);
        assert_eq!(write_iter_path(P9L_NOWRITECACHE), CachedIoPath::Unbuffered);
        assert_eq!(dotl_lock_status_to_errno(P9_LOCK_SUCCESS), 0);
        assert_eq!(dotl_lock_status_to_errno(P9_LOCK_BLOCKED), -EAGAIN);
        assert_eq!(
            lock_range(10, 19),
            LockRange {
                start: 10,
                length: 10
            }
        );
        assert_eq!(lock_range(10, OFFSET_MAX).length, 0);
        assert_eq!(
            getlock_update_range(P9_LOCK_TYPE_WRLCK, 3, 0),
            Some((3, OFFSET_MAX))
        );
        assert_eq!(getlock_update_range(P9_LOCK_TYPE_UNLCK, 3, 5), None);
        assert_eq!(flock_precheck(false), Err(-ENOLCK));
        assert_eq!(mmap_prepare(0), MmapPrepare::ReadOnly);
        assert_eq!(
            mmap_prepare(CACHE_WRITEBACK),
            MmapPrepare::WritebackWithVmOps
        );
        assert_eq!(
            mmap_close_flush_range(VM_SHARED, 2, 0x1000, 0x2000),
            Some((8192, 12287))
        );
        assert_eq!(fsync_result(-5, 0), -5);
        assert_eq!(fsync_result(0, -9), -9);
        assert_eq!(file_lock_command_valid(false, false, false), Err(-EINVAL));
        assert_eq!(symlink_mmap_allowed(), Err(-EOPNOTSUPP));
    }
}
