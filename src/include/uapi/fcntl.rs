//! linux-parity: complete
//! linux-source: vendor/linux/include/uapi
//! linux-source: vendor/linux/include/uapi/asm-generic/fcntl.h
//! linux-source: vendor/linux/include/uapi/linux/fcntl.h
//! `fcntl.h` UAPI — open() flag bits, F_* commands, lock/lease/notify, AT_*.
//!
//! Ref: `vendor/linux/include/uapi/asm-generic/fcntl.h`
//!      `vendor/linux/include/uapi/linux/fcntl.h`

#![allow(dead_code)]

// open() / openat() flag bits.
pub const O_ACCMODE: u32 = 0o0000003;
pub const O_RDONLY: u32 = 0o0000000;
pub const O_WRONLY: u32 = 0o0000001;
pub const O_RDWR: u32 = 0o0000002;
pub const O_CREAT: u32 = 0o0000100;
pub const O_EXCL: u32 = 0o0000200;
pub const O_NOCTTY: u32 = 0o0000400;
pub const O_TRUNC: u32 = 0o0001000;
pub const O_APPEND: u32 = 0o0002000;
pub const O_NONBLOCK: u32 = 0o0004000;
pub const O_DSYNC: u32 = 0o0010000;
pub const O_DIRECT: u32 = 0o0040000;
pub const O_LARGEFILE: u32 = 0o0100000;
pub const O_DIRECTORY: u32 = 0o0200000;
pub const O_NOFOLLOW: u32 = 0o0400000;
pub const O_NOATIME: u32 = 0o1000000;
pub const O_CLOEXEC: u32 = 0o2000000;
pub const O_SYNC: u32 = 0o4010000;
pub const O_PATH: u32 = 0o10000000;
pub const O_TMPFILE: u32 = 0o20200000;
/// `FASYNC` — fcntl, for BSD compatibility (same bit as `O_FASYNC`).
pub const FASYNC: u32 = 0o0020000;
/// `O_NDELAY` — alias of `O_NONBLOCK`.
pub const O_NDELAY: u32 = O_NONBLOCK;

// fcntl() commands.
pub const F_DUPFD: i32 = 0;
pub const F_GETFD: i32 = 1;
pub const F_SETFD: i32 = 2;
pub const F_GETFL: i32 = 3;
pub const F_SETFL: i32 = 4;
pub const F_DUPFD_CLOEXEC: i32 = 1030;
pub const F_ADD_SEALS: i32 = 1033;
pub const F_GET_SEALS: i32 = 1034;

// memfd seals.
pub const F_SEAL_SEAL: u32 = 0x0001;
pub const F_SEAL_SHRINK: u32 = 0x0002;
pub const F_SEAL_GROW: u32 = 0x0004;
pub const F_SEAL_WRITE: u32 = 0x0008;
pub const F_SEAL_FUTURE_WRITE: u32 = 0x0010;
pub const F_SEAL_EXEC: u32 = 0x0020;

// fd flags (F_GETFD/F_SETFD).
pub const FD_CLOEXEC: u32 = 1;

// AT_* flags for *at() syscalls.
pub const AT_FDCWD: i32 = -100;
pub const AT_SYMLINK_NOFOLLOW: u32 = 0x100;
pub const AT_REMOVEDIR: u32 = 0x200;
pub const AT_SYMLINK_FOLLOW: u32 = 0x400;
pub const AT_NO_AUTOMOUNT: u32 = 0x800;
pub const AT_EMPTY_PATH: u32 = 0x1000;
pub const AT_RECURSIVE: u32 = 0x8000;

// Record-locking and socket-owner fcntl() commands (asm-generic/fcntl.h).
pub const F_GETLK: i32 = 5;
pub const F_SETLK: i32 = 6;
pub const F_SETLKW: i32 = 7;
pub const F_SETOWN: i32 = 8;
pub const F_GETOWN: i32 = 9;
pub const F_SETSIG: i32 = 10;
pub const F_GETSIG: i32 = 11;
pub const F_GETLK64: i32 = 12;
pub const F_SETLK64: i32 = 13;
pub const F_SETLKW64: i32 = 14;
pub const F_SETOWN_EX: i32 = 15;
pub const F_GETOWN_EX: i32 = 16;
pub const F_GETOWNER_UIDS: i32 = 17;
pub const F_OFD_GETLK: i32 = 36;
pub const F_OFD_SETLK: i32 = 37;
pub const F_OFD_SETLKW: i32 = 38;

// `f_owner_ex.type` values.
pub const F_OWNER_TID: i32 = 0;
pub const F_OWNER_PID: i32 = 1;
pub const F_OWNER_PGRP: i32 = 2;

// Lock types (`flock.l_type`).
pub const F_RDLCK: i16 = 0;
pub const F_WRLCK: i16 = 1;
pub const F_UNLCK: i16 = 2;
pub const F_EXLCK: i16 = 4;
pub const F_SHLCK: i16 = 8;

// flock(2) operations.
pub const LOCK_SH: i32 = 1;
pub const LOCK_EX: i32 = 2;
pub const LOCK_NB: i32 = 4;
pub const LOCK_UN: i32 = 8;
pub const LOCK_MAND: i32 = 32;
pub const LOCK_READ: i32 = 64;
pub const LOCK_WRITE: i32 = 128;
pub const LOCK_RW: i32 = 192;

// Linux-specific fcntl() commands (uapi/linux/fcntl.h).
pub const F_LINUX_SPECIFIC_BASE: i32 = 1024;
pub const F_SETLEASE: i32 = F_LINUX_SPECIFIC_BASE;
pub const F_GETLEASE: i32 = F_LINUX_SPECIFIC_BASE + 1;
pub const F_NOTIFY: i32 = F_LINUX_SPECIFIC_BASE + 2;
pub const F_DUPFD_QUERY: i32 = F_LINUX_SPECIFIC_BASE + 3;
pub const F_CREATED_QUERY: i32 = F_LINUX_SPECIFIC_BASE + 4;
pub const F_CANCELLK: i32 = F_LINUX_SPECIFIC_BASE + 5;
pub const F_SETPIPE_SZ: i32 = F_LINUX_SPECIFIC_BASE + 7;
pub const F_GETPIPE_SZ: i32 = F_LINUX_SPECIFIC_BASE + 8;
pub const F_GET_RW_HINT: i32 = F_LINUX_SPECIFIC_BASE + 11;
pub const F_SET_RW_HINT: i32 = F_LINUX_SPECIFIC_BASE + 12;
pub const F_GET_FILE_RW_HINT: i32 = F_LINUX_SPECIFIC_BASE + 13;
pub const F_SET_FILE_RW_HINT: i32 = F_LINUX_SPECIFIC_BASE + 14;
pub const F_GETDELEG: i32 = F_LINUX_SPECIFIC_BASE + 15;
pub const F_SETDELEG: i32 = F_LINUX_SPECIFIC_BASE + 16;

// Write-lifetime hints (F_GET/SET_RW_HINT).
pub const RWH_WRITE_LIFE_NOT_SET: u64 = 0;
pub const RWH_WRITE_LIFE_NONE: u64 = 1;
pub const RWH_WRITE_LIFE_SHORT: u64 = 2;
pub const RWH_WRITE_LIFE_MEDIUM: u64 = 3;
pub const RWH_WRITE_LIFE_LONG: u64 = 4;
pub const RWH_WRITE_LIFE_EXTREME: u64 = 5;
pub const RWF_WRITE_LIFE_NOT_SET: u64 = RWH_WRITE_LIFE_NOT_SET;

// Directory-change notification (F_NOTIFY) events.
pub const DN_ACCESS: u32 = 0x00000001;
pub const DN_MODIFY: u32 = 0x00000002;
pub const DN_CREATE: u32 = 0x00000004;
pub const DN_DELETE: u32 = 0x00000008;
pub const DN_RENAME: u32 = 0x00000010;
pub const DN_ATTRIB: u32 = 0x00000020;
pub const DN_MULTISHOT: u32 = 0x80000000;

// Additional AT_* flags.
pub const AT_EACCESS: u32 = 0x200; // shares the AT_REMOVEDIR bit (context-dependent)
pub const AT_STATX_SYNC_TYPE: u32 = 0x6000;
pub const AT_STATX_SYNC_AS_STAT: u32 = 0x0000;
pub const AT_STATX_FORCE_SYNC: u32 = 0x2000;
pub const AT_STATX_DONT_SYNC: u32 = 0x4000;
pub const AT_RENAME_NOREPLACE: u32 = 0x0001;
pub const AT_RENAME_EXCHANGE: u32 = 0x0002;
pub const AT_RENAME_WHITEOUT: u32 = 0x0004;
pub const AT_HANDLE_FID: u32 = 0x200;
pub const AT_HANDLE_MNT_ID_UNIQUE: u32 = 0x001;
pub const AT_HANDLE_CONNECTABLE: u32 = 0x002;
pub const AT_EXECVE_CHECK: u32 = 0x10000;

/// `struct flock` (x86-64 layout: `short, short, off_t, off_t, pid_t`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Flock {
    pub l_type: i16,
    pub l_whence: i16,
    pub l_start: i64,
    pub l_len: i64,
    pub l_pid: i32,
}

/// `struct f_owner_ex` — operand of `F_SETOWN_EX`/`F_GETOWN_EX`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FOwnerEx {
    /// `F_OWNER_TID` / `F_OWNER_PID` / `F_OWNER_PGRP`.
    pub type_: i32,
    pub pid: i32,
}
