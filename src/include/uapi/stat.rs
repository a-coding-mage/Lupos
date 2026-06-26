//! linux-parity: complete
//! linux-source: vendor/linux/include/uapi
//! linux-source: vendor/linux/include/uapi/linux/stat.h
//! `stat.h` UAPI — file mode bits, `S_IS*` type tests, and `STATX_*` flags.
//!
//! The `struct statx`/`struct statx_timestamp` layouts live with the `statx(2)`
//! implementation (`LinuxStatx`/`LinuxStatxTimestamp` in `fs/syscalls.rs`); this
//! file is the canonical home for the mode bits and the `STATX_*` request/attr
//! masks.
//!
//! Ref: `vendor/linux/include/uapi/linux/stat.h`

#![allow(dead_code)]

pub const S_IFMT: u32 = 0o170000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFIFO: u32 = 0o010000;

pub const S_ISUID: u32 = 0o004000;
pub const S_ISGID: u32 = 0o002000;
pub const S_ISVTX: u32 = 0o001000;

pub const S_IRWXU: u32 = 0o0700;
pub const S_IRUSR: u32 = 0o0400;
pub const S_IWUSR: u32 = 0o0200;
pub const S_IXUSR: u32 = 0o0100;

pub const S_IRWXG: u32 = 0o0070;
pub const S_IRGRP: u32 = 0o0040;
pub const S_IWGRP: u32 = 0o0020;
pub const S_IXGRP: u32 = 0o0010;

pub const S_IRWXO: u32 = 0o0007;
pub const S_IROTH: u32 = 0o0004;
pub const S_IWOTH: u32 = 0o0002;
pub const S_IXOTH: u32 = 0o0001;

#[inline]
pub const fn is_reg(m: u32) -> bool {
    (m & S_IFMT) == S_IFREG
}
#[inline]
pub const fn is_dir(m: u32) -> bool {
    (m & S_IFMT) == S_IFDIR
}
#[inline]
pub const fn is_lnk(m: u32) -> bool {
    (m & S_IFMT) == S_IFLNK
}
#[inline]
pub const fn is_chr(m: u32) -> bool {
    (m & S_IFMT) == S_IFCHR
}
#[inline]
pub const fn is_blk(m: u32) -> bool {
    (m & S_IFMT) == S_IFBLK
}
#[inline]
pub const fn is_fifo(m: u32) -> bool {
    (m & S_IFMT) == S_IFIFO
}
#[inline]
pub const fn is_sock(m: u32) -> bool {
    (m & S_IFMT) == S_IFSOCK
}

// ── statx(2) request masks (`stx_mask`) ──────────────────────────────────────
pub const STATX_TYPE: u32 = 0x0000_0001;
pub const STATX_MODE: u32 = 0x0000_0002;
pub const STATX_NLINK: u32 = 0x0000_0004;
pub const STATX_UID: u32 = 0x0000_0008;
pub const STATX_GID: u32 = 0x0000_0010;
pub const STATX_ATIME: u32 = 0x0000_0020;
pub const STATX_MTIME: u32 = 0x0000_0040;
pub const STATX_CTIME: u32 = 0x0000_0080;
pub const STATX_INO: u32 = 0x0000_0100;
pub const STATX_SIZE: u32 = 0x0000_0200;
pub const STATX_BLOCKS: u32 = 0x0000_0400;
pub const STATX_BASIC_STATS: u32 = 0x0000_07ff;
pub const STATX_BTIME: u32 = 0x0000_0800;
pub const STATX_MNT_ID: u32 = 0x0000_1000;
pub const STATX_DIOALIGN: u32 = 0x0000_2000;
pub const STATX_MNT_ID_UNIQUE: u32 = 0x0000_4000;
pub const STATX_SUBVOL: u32 = 0x0000_8000;
pub const STATX_WRITE_ATOMIC: u32 = 0x0001_0000;
pub const STATX_DIO_READ_ALIGN: u32 = 0x0002_0000;
pub const STATX_RESERVED: u32 = 0x8000_0000;
pub const STATX_ALL: u32 = 0x0000_0fff;

// ── statx attribute flags (`stx_attributes`) ─────────────────────────────────
pub const STATX_ATTR_COMPRESSED: u64 = 0x0000_0004;
pub const STATX_ATTR_IMMUTABLE: u64 = 0x0000_0010;
pub const STATX_ATTR_APPEND: u64 = 0x0000_0020;
pub const STATX_ATTR_NODUMP: u64 = 0x0000_0040;
pub const STATX_ATTR_ENCRYPTED: u64 = 0x0000_0800;
pub const STATX_ATTR_AUTOMOUNT: u64 = 0x0000_1000;
pub const STATX_ATTR_MOUNT_ROOT: u64 = 0x0000_2000;
pub const STATX_ATTR_VERITY: u64 = 0x0010_0000;
pub const STATX_ATTR_DAX: u64 = 0x0020_0000;
pub const STATX_ATTR_WRITE_ATOMIC: u64 = 0x0040_0000;
