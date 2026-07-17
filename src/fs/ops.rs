//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! VFS vtables — `super_operations`, `inode_operations`, `file_operations`,
//! `dentry_operations`.
//!
//! Mirrors `vendor/linux/include/linux/fs.h`.  Function-pointer slots that
//! a filesystem doesn't implement get a no-op default so the dispatch path
//! never has to NULL-check before calling.

use super::types::{Dentry, DentryRef, File, FileRef, Inode, InodeRef, SuperBlock, SuperBlockRef};
use crate::mm::mm_types::VmAreaStruct;

pub type PollFn = fn(&FileRef, Option<&mut crate::fs::select::PollTable>) -> u32;
pub type IoctlFn = fn(&FileRef, cmd: u32, arg: u64) -> Result<i64, i32>;
/// Rust-native equivalent of Linux `file_operations::mmap`.
///
/// The MM core initializes the VMA before invoking the callback. As in Linux,
/// the callback runs once while the mapping is created and may update VMA
/// flags, page protection, operations, and private data.
pub type MmapFn = fn(&FileRef, &mut VmAreaStruct) -> Result<(), i32>;
pub type RenameFn =
    fn(old_dir: &InodeRef, old_name: &str, new_dir: &InodeRef, new_name: &str) -> Result<(), i32>;
pub type SymlinkFn =
    fn(dir: &InodeRef, name: &str, target: &str, mode: u32) -> Result<InodeRef, i32>;

/// `struct super_operations`.
#[repr(C)]
pub struct SuperOps {
    pub name: &'static str,
    pub statfs: Option<fn(&SuperBlock) -> i32>,
    pub put_super: Option<fn(&SuperBlockRef)>,
    pub sync_fs: Option<fn(&SuperBlock, wait: bool) -> i32>,
    pub alloc_inode: Option<fn(&SuperBlockRef) -> InodeRef>,
    pub destroy_inode: Option<fn(InodeRef)>,
}

pub const NOOP_SUPER_OPS: SuperOps = SuperOps {
    name: "noop",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

/// `struct inode_operations`.  Returns `Result<_, errno>`.
#[repr(C)]
pub struct InodeOps {
    pub name: &'static str,
    /// Look up `name` in this directory inode; return the child inode or
    /// `Err(-ENOENT)` for a negative result.
    pub lookup: Option<fn(dir: &InodeRef, name: &str) -> Result<InodeRef, i32>>,
    /// Create a regular file `name` under `dir` with `mode`.
    pub create: Option<fn(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32>>,
    /// Create a directory `name` under `dir` with `mode`.
    pub mkdir: Option<fn(dir: &InodeRef, name: &str, mode: u32) -> Result<InodeRef, i32>>,
    /// Remove a regular-file entry. Linux passes the resolved target dentry
    /// into ->unlink(); keeping the target inode here is essential for
    /// coherent in-memory link counts on disk-backed filesystems.
    pub unlink: Option<fn(dir: &InodeRef, name: &str, target: &InodeRef) -> Result<(), i32>>,
    /// Remove an empty directory.
    pub rmdir: Option<fn(dir: &InodeRef, name: &str) -> Result<(), i32>>,
    /// Rename an existing directory entry, replacing the destination when the
    /// VFS has already allowed it.
    pub rename: Option<RenameFn>,
    /// Create a symbolic link `name` under `dir` pointing at `target`.
    pub symlink: Option<SymlinkFn>,
    /// Read symlink target into `buf` (returns bytes written).
    pub readlink: Option<fn(inode: &InodeRef, buf: &mut [u8]) -> Result<usize, i32>>,
}

pub const NOOP_INODE_OPS: InodeOps = InodeOps {
    name: "noop",
    lookup: None,
    create: None,
    mkdir: None,
    unlink: None,
    rmdir: None,
    rename: None,
    symlink: None,
    readlink: None,
};

/// `struct file_operations`.  M38 lands the read/write/llseek/fsync/release
/// slots — readdir/poll/mmap follow in M39+.
#[repr(C)]
pub struct FileOps {
    pub name: &'static str,
    pub read: Option<fn(&FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32>>,
    pub write: Option<fn(&FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32>>,
    pub llseek: Option<fn(&FileRef, off: i64, whence: i32) -> Result<u64, i32>>,
    pub fsync: Option<fn(&FileRef) -> Result<(), i32>>,
    pub poll: Option<PollFn>,
    pub ioctl: Option<IoctlFn>,
    pub mmap: Option<MmapFn>,
    pub release: Option<fn(FileRef)>,
    /// Iterate one directory entry — returns `Ok(Some((name, ino, kind)))` per
    /// call, `Ok(None)` at end-of-stream. The directory cursor lives in
    /// `file.pos`, matching Linux `file->f_pos` and lseek(2).
    pub readdir: Option<
        fn(&FileRef) -> Result<Option<(alloc::string::String, u64, super::types::InodeKind)>, i32>,
    >,
}

pub const NOOP_FILE_OPS: FileOps = FileOps {
    name: "noop",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

/// Empty file operations for Linux `O_PATH` handles. The file still pins a
/// dentry/inode for metadata and dirfd use, but normal I/O dispatch rejects it.
pub const PATH_FILE_OPS: FileOps = FileOps {
    name: "path",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

/// `struct dentry_operations`.  Linux 6.x has many slots; M38 only needs the
/// release hook for fs-specific cleanup.
#[repr(C)]
pub struct DentryOps {
    pub name: &'static str,
    pub d_release: Option<fn(&Dentry)>,
}

pub const NOOP_DENTRY_OPS: DentryOps = DentryOps {
    name: "noop",
    d_release: None,
};
