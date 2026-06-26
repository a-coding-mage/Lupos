//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry
//! test-origin: linux:vendor/linux/arch/x86/entry
//! x86_64 IA32 compatibility syscall conversion helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/sys_ia32.c

#![allow(dead_code)]

use crate::include::uapi::errno::{EFAULT, EINVAL};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MmapArgStruct32 {
    pub addr: u32,
    pub len: u32,
    pub prot: u32,
    pub flags: u32,
    pub fd: u32,
    pub offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stat64Compat {
    pub st_dev: u64,
    pub __st_ino: u32,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: u32,
    pub st_blocks: u64,
    pub st_atime: u32,
    pub st_atime_nsec: u32,
    pub st_mtime: u32,
    pub st_mtime_nsec: u32,
    pub st_ctime: u32,
    pub st_ctime_nsec: u32,
    pub st_ino: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KstatCompatInput {
    pub dev: u64,
    pub ino: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u64,
    pub size: i64,
    pub blksize: u32,
    pub blocks: u64,
    pub atime_sec: u64,
    pub atime_nsec: u32,
    pub mtime_sec: u64,
    pub mtime_nsec: u32,
    pub ctime_sec: u64,
    pub ctime_nsec: u32,
}

pub const fn join_u32_low_high(low: u32, high: u32) -> u64 {
    low as u64 | ((high as u64) << 32)
}

pub const fn ia32_mmap_pgoff(arg: MmapArgStruct32) -> Result<(u64, u64, u32, u32, i32, u64), i32> {
    const PAGE_MASK: u32 = !0xfff;
    if arg.offset & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }
    Ok((
        arg.addr as u64,
        arg.len as u64,
        arg.prot,
        arg.flags,
        arg.fd as i32,
        (arg.offset >> 12) as u64,
    ))
}

pub fn cp_stat64(stat: KstatCompatInput) -> Stat64Compat {
    Stat64Compat {
        st_dev: stat.dev,
        __st_ino: stat.ino as u32,
        st_mode: stat.mode,
        st_nlink: stat.nlink,
        st_uid: stat.uid,
        st_gid: stat.gid,
        st_rdev: stat.rdev,
        st_size: stat.size,
        st_blksize: stat.blksize,
        st_blocks: stat.blocks,
        st_atime: stat.atime_sec as u32,
        st_atime_nsec: stat.atime_nsec,
        st_mtime: stat.mtime_sec as u32,
        st_mtime_nsec: stat.mtime_nsec,
        st_ctime: stat.ctime_sec as u32,
        st_ctime_nsec: stat.ctime_nsec,
        st_ino: stat.ino,
    }
}

pub unsafe fn compat_ia32_mmap(
    arg: *const MmapArgStruct32,
) -> Result<(u64, u64, u32, u32, i32, u64), i32> {
    if arg.is_null() {
        return Err(EFAULT);
    }
    ia32_mmap_pgoff(unsafe { *arg })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ia32_mmap_converts_offset_to_pages() {
        let arg = MmapArgStruct32 {
            addr: 0x1000,
            len: 0x2000,
            prot: 3,
            flags: 0x22,
            fd: u32::MAX,
            offset: 0x3000,
        };
        assert_eq!(ia32_mmap_pgoff(arg), Ok((0x1000, 0x2000, 3, 0x22, -1, 3)));
        assert_eq!(
            ia32_mmap_pgoff(MmapArgStruct32 { offset: 7, ..arg }),
            Err(EINVAL)
        );
    }

    #[test]
    fn cp_stat64_preserves_64bit_inode_and_truncates_legacy_times() {
        let out = cp_stat64(KstatCompatInput {
            ino: 0x1_0000_0002,
            atime_sec: 0x1_0000_0003,
            ..Default::default()
        });
        assert_eq!(out.__st_ino, 2);
        assert_eq!(out.st_ino, 0x1_0000_0002);
        assert_eq!(out.st_atime, 3);
    }
}
