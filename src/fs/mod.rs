//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! Virtual File System — M38–M42 (Phase 6).
//!
//! Mirrors `vendor/linux/fs/`.  M38 lands the core:
//!   * `types`     — `SuperBlock`, `Inode`, `Dentry`, `File`, `Path`
//!   * `ops`       — vtables (`SuperOps`, `InodeOps`, `DentryOps`, `FileOps`)
//!   * `dcache`    — dentry cache, `d_alloc` / `d_lookup` / `dput`
//!   * `inode`     — inode cache helpers, `iget` / `iput`
//!   * `super_block` — `register_filesystem`, `mount_fs`, `kill_super`
//!   * `file`      — file table, `alloc_file`, `fput`
//!   * `read_write` — `vfs_read`, `vfs_write`, `vfs_lseek`
//!   * `libfs`     — generic helpers ported from `vendor/linux/fs/libfs.c`
//!   * `ramfs`     — reference in-memory filesystem (M38 acceptance)
//!
//! Subsequent milestones add `mount`, `namei`, `openat`, `fdtable`, `fs_struct`,
//! `proc`, `sysfs`, `kernfs`, `debugfs`, and overlay mounts.

pub mod adfs;
pub mod affs;
pub mod anon_inode;
pub mod attr;
pub mod autofs;
pub mod befs;
pub mod binfmt_elf; // M24a
pub mod binfmt_script; // M24a
pub mod btrfs;
pub mod ceph;
pub mod coda;
pub mod cramfs;
pub mod crypto;
pub mod dcache;
pub mod dlm;
pub mod drop_caches;
pub mod ecryptfs;
pub mod efs;
pub mod file;
pub mod file_table;
pub mod filesystems;
pub mod freevxfs;
pub mod fs_dirent;
pub mod fs_pin;
pub mod fuse;
pub mod gfs2;
pub mod hfs;
pub mod hfsplus;
pub mod hostfs;
pub mod hpfs;
pub mod hugetlbfs;
pub mod inode;
pub mod iomap;
pub mod jffs2;
pub mod jfs;
pub mod libfs;
pub mod lockd;
pub mod minix;
pub mod mqueue;
pub mod netfs;
pub mod nfs;
pub mod nfs_common;
pub mod nfsd;
pub mod nls;
pub mod ntfs;
pub mod ntfs3;
pub mod nullfs;
pub mod ocfs2;
pub mod ops;
pub mod proc_namespace;
pub mod pstore;
pub mod qnx4;
pub mod qnx6;
pub mod quota;
pub mod ramfs;
pub mod read_write;
pub mod romfs;
pub mod select;
pub mod squashfs;
pub mod stack;
pub mod stat;
pub mod super_block;
pub mod syscalls;
pub mod sysctls;
pub mod tests;
pub mod types;
pub mod ubifs;
pub mod udf;
pub mod ufs;
pub mod v9fs;
pub mod verity;
pub mod xattr;
pub mod xfs;

// M39
pub mod fcntl;
pub mod fdtable;
pub mod fs_struct;
pub mod ioctl;
pub mod mount;
pub mod namei;
pub mod openat;
pub mod orangefs;
pub mod permission;
pub mod pipe;

// M40
pub mod kernfs;
pub mod proc;

// M41
pub mod sysfs;

// M42
pub mod cachefiles;
pub mod debugfs;
pub mod namespace;
pub mod nsfs;
pub mod overlayfs;
pub mod smb;

// M45 / M46 — on-disk filesystems
pub mod ext2;
pub mod ext4;
pub mod fat;
pub mod isofs;
pub mod jbd2;

// M60 — event/notification fds.  Data structures + syscall entry points;
// real VFS-fd integration deferred (FileOps lacks a poll slot).
pub mod eventfd;
pub mod eventpoll;
pub mod fanotify;
pub mod inotify;
pub mod pidfd;
pub mod signalfd;
pub mod timerfd;

pub use ops::{FileOps, InodeOps, SuperOps};
pub use super_block::{FileSystemType, MountFn, lookup_filesystem, register_filesystem};
pub use types::{
    Dentry, DentryRef, File, FileRef, Inode, InodeKind, InodeRef, SuperBlock, SuperBlockRef,
};

/// Initialise the VFS subsystem.  Registers built-in filesystem types.
///
/// Idempotent — safe to call multiple times under feature gating.
pub fn init() {
    super_block::init_registry();
    ramfs::register();
    hugetlbfs::register();
    mqueue::register();
    proc::register();
    sysfs::register();
    crate::mm::shmem::register();
    debugfs::register();
    crate::kernel::cgroup::register();
    overlayfs::register();
    crate::security::inode::register();
    ext4::register();
    fat::register();
    isofs::register();
    binfmt_elf::register();
    binfmt_script::register();
}
