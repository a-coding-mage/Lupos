//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/page.c
//! `/proc` page accounting files.
//!
//! Ref: `vendor/linux/fs/proc/page.c`

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use crate::fs::anon_inode::alloc_anon_file_with_kind;
use crate::fs::kernfs::KernfsNode;
use crate::fs::ops::FileOps;
use crate::fs::types::{FileRef, InodeKind};
use crate::include::uapi::errno::{EACCES, EINVAL};
use crate::include::uapi::fcntl::{O_ACCMODE, O_RDONLY};

static PROC_KPAGEFLAGS_FILE_OPS: FileOps = FileOps {
    name: "proc-kpageflags",
    read: Some(kpageflags_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

pub fn pagetypeinfo_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "Page block order: 0\nPages per block:  1\n")
}

pub fn kpageflags_file_from_proc_path(
    path: &str,
    flags: u32,
    mode: u32,
) -> Option<Result<FileRef, i32>> {
    if path != "/proc/kpageflags" {
        return None;
    }
    Some(kpageflags_file(flags, mode))
}

pub fn kpageflags_file(flags: u32, mode: u32) -> Result<FileRef, i32> {
    if flags & O_ACCMODE != O_RDONLY {
        return Err(EACCES);
    }
    let _ = mode;
    let file = alloc_anon_file_with_kind(
        "kpageflags",
        &PROC_KPAGEFLAGS_FILE_OPS,
        0,
        InodeKind::Regular,
        0o400,
    );
    file.flags.store(flags, Ordering::Release);
    Ok(file)
}

fn kpageflags_read(_file: &FileRef, buf: &mut [u8], pos: &mut u64) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }
    if *pos % 8 != 0 {
        return Err(EINVAL);
    }
    let mut written = 0usize;
    while written + 8 <= buf.len() {
        let pfn = (*pos / 8).saturating_add((written / 8) as u64);
        let flags = crate::mm::huge::kpageflags_for_pfn(pfn);
        buf[written..written + 8].copy_from_slice(&flags.to_ne_bytes());
        written += 8;
    }
    *pos += written as u64;
    Ok(written)
}
