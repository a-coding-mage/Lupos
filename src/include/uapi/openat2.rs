//! linux-parity: complete
//! linux-source: vendor/linux/include/uapi
//! linux-source: vendor/linux/include/uapi/linux/openat2.h
//! `openat2(2)` UAPI — `open_how` and `RESOLVE_*` flags.
//!
//! Verified complete against `openat2.h`: `struct open_how` (flags/mode/resolve)
//! and all six RESOLVE_* flags (NO_XDEV 0x01, NO_MAGICLINKS 0x02, NO_SYMLINKS
//! 0x04, BENEATH 0x08, IN_ROOT 0x10, CACHED 0x20). `RESOLVE_VALID_MASK` mirrors
//! the kernel's accepted-flags mask.
//!
//! Ref: `vendor/linux/include/uapi/linux/openat2.h`

#![allow(dead_code)]

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenHow {
    pub flags: u64,
    pub mode: u64,
    pub resolve: u64,
}

pub const RESOLVE_NO_XDEV: u64 = 0x01;
pub const RESOLVE_NO_MAGICLINKS: u64 = 0x02;
pub const RESOLVE_NO_SYMLINKS: u64 = 0x04;
pub const RESOLVE_BENEATH: u64 = 0x08;
pub const RESOLVE_IN_ROOT: u64 = 0x10;
pub const RESOLVE_CACHED: u64 = 0x20;

pub const RESOLVE_VALID_MASK: u64 = RESOLVE_NO_XDEV
    | RESOLVE_NO_MAGICLINKS
    | RESOLVE_NO_SYMLINKS
    | RESOLVE_BENEATH
    | RESOLVE_IN_ROOT
    | RESOLVE_CACHED;
