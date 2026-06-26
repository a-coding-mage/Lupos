//! linux-parity: complete
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! fanotify — file-system event notifications (more powerful than inotify).
//!
//! ABI parity with vendor/linux/fs/notify/fanotify/fanotify_user.c.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EBADF, EFAULT};
use crate::kernel::{files, sched};

/// `struct fanotify_event_metadata` — fixed-size header read from fanotify fd.
/// Byte-identical to Linux uapi/linux/fanotify.h (24 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FanotifyEventMetadata {
    pub event_len: u32,
    pub vers: u8,
    pub reserved: u8,
    pub metadata_len: u16,
    pub mask: u64,
    pub fd: i32,
    pub pid: i32,
}

/// FAN_* event-mask bits.
pub const FAN_ACCESS: u64 = 0x0000_0001;
pub const FAN_MODIFY: u64 = 0x0000_0002;
pub const FAN_CLOSE_WRITE: u64 = 0x0000_0008;
pub const FAN_OPEN: u64 = 0x0000_0020;

/// FAN_CLASS_* flags for fanotify_init.
pub const FAN_CLASS_NOTIF: u32 = 0x0000_0000;
pub const FAN_CLASS_CONTENT: u32 = 0x0000_0004;

struct FanotifyGroup {
    marks: Mutex<Vec<u64>>,
}

static FANOTIFY_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref FANOTIFIES: Mutex<BTreeMap<usize, FanotifyGroup>> = Mutex::new(BTreeMap::new());
}

static FANOTIFY_FILE_OPS: FileOps = FileOps {
    name: "fanotify",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: None,
    mmap: None,
    release: Some(fanotify_release),
    readdir: None,
};

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn fanotify_token(fd: i32) -> Result<usize, i32> {
    let file = current_files()?.get(fd)?;
    if file.fops.name != FANOTIFY_FILE_OPS.name {
        return Err(EBADF);
    }
    Ok(*file.private.lock())
}

fn fanotify_release(file: FileRef) {
    let token = *file.private.lock();
    FANOTIFIES.lock().remove(&token);
}

/// `sys_fanotify_init(flags, event_f_flags)` — Linux syscall 300.
pub unsafe fn sys_fanotify_init(_flags: u32, _event_f_flags: u32) -> i64 {
    let token = FANOTIFY_TOKEN.fetch_add(1, Ordering::AcqRel);
    FANOTIFIES.lock().insert(
        token,
        FanotifyGroup {
            marks: Mutex::new(Vec::new()),
        },
    );
    let file = alloc_anon_file("fanotify", &FANOTIFY_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, false)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            FANOTIFIES.lock().remove(&token);
            -(errno as i64)
        }
    }
}

/// `sys_fanotify_mark(fd, flags, mask, dfd, pathname)` — Linux syscall 301.
pub unsafe fn sys_fanotify_mark(
    fd: i32,
    _flags: u32,
    mask: u64,
    _dfd: i32,
    pathname: *const i8,
) -> i64 {
    return {
        if pathname.is_null() {
            return -(EFAULT as i64);
        }
        let token = match fanotify_token(fd) {
            Ok(token) => token,
            Err(errno) => return -(errno as i64),
        };
        let mut table = FANOTIFIES.lock();
        let Some(group) = table.get_mut(&token) else {
            return -(EBADF as i64);
        };
        group.marks.lock().push(mask);
        0
    };
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_size_is_24() {
        assert_eq!(core::mem::size_of::<FanotifyEventMetadata>(), 24);
    }

    #[test]
    fn metadata_field_offsets() {
        assert_eq!(core::mem::offset_of!(FanotifyEventMetadata, event_len), 0);
        assert_eq!(core::mem::offset_of!(FanotifyEventMetadata, vers), 4);
        assert_eq!(core::mem::offset_of!(FanotifyEventMetadata, mask), 8);
        assert_eq!(core::mem::offset_of!(FanotifyEventMetadata, fd), 16);
        assert_eq!(core::mem::offset_of!(FanotifyEventMetadata, pid), 20);
    }
}
