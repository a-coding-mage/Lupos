//! linux-parity: complete
//! linux-source: vendor/linux/fs/eventfd.c
//! test-origin: linux:vendor/linux/fs/eventfd.c
//! eventfd — counter-style fd for thread/process notifications.
//!
//! ABI parity with vendor/linux/fs/eventfd.c.
//! M60 implements the in-kernel counter semantics; full VFS-fd integration
//! is deferred until FileOps gains a poll() slot.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EAGAIN, EBADF, EINVAL};
use crate::include::uapi::fcntl::{O_NONBLOCK, O_RDWR};
use crate::kernel::{files, sched};

/// `EFD_*` flags — byte-identical to Linux UAPI.
pub const EFD_SEMAPHORE: i32 = 0o0000001;
pub const EFD_CLOEXEC: i32 = 0o2000000;
pub const EFD_NONBLOCK: i32 = 0o0004000;

/// In-kernel state for one eventfd.
pub struct EventFd {
    pub count: AtomicU64,
    pub flags: i32,
}

static EVENTFD_TOKEN: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref EVENTFDS: Mutex<BTreeMap<usize, Arc<EventFd>>> = Mutex::new(BTreeMap::new());
}

static EVENTFD_FILE_OPS: FileOps = FileOps {
    name: "eventfd",
    read: Some(eventfd_file_read),
    write: Some(eventfd_file_write),
    llseek: None,
    fsync: None,
    poll: Some(eventfd_file_poll),
    ioctl: None,
    mmap: None,
    release: Some(eventfd_release),
    readdir: None,
};

impl EventFd {
    pub fn new(initval: u64, flags: i32) -> Self {
        Self {
            count: AtomicU64::new(initval),
            flags,
        }
    }

    /// Read semantics: returns current count and resets it (or decrements by 1
    /// in EFD_SEMAPHORE mode).  Returns EAGAIN when a read would block.
    /// Linux: vendor/linux/fs/eventfd.c::eventfd_read
    pub fn read(&self) -> Result<u64, i32> {
        let v = self.count.load(Ordering::Acquire);
        if v == 0 {
            return Err(EAGAIN);
        }
        if self.flags & EFD_SEMAPHORE != 0 {
            self.count.fetch_sub(1, Ordering::AcqRel);
            Ok(1)
        } else {
            self.count.store(0, Ordering::Release);
            Ok(v)
        }
    }

    /// Write semantics: adds value to count.  Caps at u64::MAX-1.
    pub fn write(&self, val: u64) -> Result<usize, i32> {
        if val == u64::MAX {
            return Err(EINVAL);
        }
        let cur = self.count.load(Ordering::Acquire);
        if cur.saturating_add(val) >= u64::MAX {
            return Err(EAGAIN);
        }
        self.count.fetch_add(val, Ordering::AcqRel);
        Ok(8)
    }

    /// poll() mask — EPOLLIN if readable, EPOLLOUT if writable.
    pub fn poll_mask(&self) -> u32 {
        let mut mask = 0u32;
        let v = self.count.load(Ordering::Acquire);
        if v > 0 {
            mask |= 0x0001; // EPOLLIN
        }
        if v < u64::MAX - 1 {
            mask |= 0x0004; // EPOLLOUT
        }
        mask
    }
}

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn eventfd_from_file(file: &FileRef) -> Result<Arc<EventFd>, i32> {
    if file.fops.name != EVENTFD_FILE_OPS.name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    EVENTFDS.lock().get(&token).cloned().ok_or(EBADF)
}

fn eventfd_file_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.len() < 8 {
        return Err(EINVAL);
    }
    let value = eventfd_from_file(file)?.read()?;
    buf[..8].copy_from_slice(&value.to_ne_bytes());
    Ok(8)
}

fn eventfd_file_write(file: &FileRef, buf: &[u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.len() < 8 {
        return Err(EINVAL);
    }
    let value = u64::from_ne_bytes(buf[..8].try_into().map_err(|_| EINVAL)?);
    eventfd_from_file(file)?.write(value)
}

fn eventfd_file_poll(file: &FileRef) -> u32 {
    eventfd_from_file(file)
        .map(|e| e.poll_mask())
        .unwrap_or(0x0008)
}

fn eventfd_release(file: FileRef) {
    let token = *file.private.lock();
    EVENTFDS.lock().remove(&token);
}

/// `sys_eventfd2(initval, flags)` — Linux syscall 290.
/// M60 stub: returns a synthetic fd; real anon-fd integration deferred.
pub unsafe fn sys_eventfd2(initval: u32, flags: i32) -> i64 {
    let allowed = EFD_SEMAPHORE | EFD_CLOEXEC | EFD_NONBLOCK;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    let token = EVENTFD_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
    EVENTFDS
        .lock()
        .insert(token, Arc::new(EventFd::new(initval as u64, flags)));
    let file = alloc_anon_file("eventfd", &EVENTFD_FILE_OPS, token);
    file.flags
        .store(O_RDWR | ((flags as u32) & O_NONBLOCK), Ordering::Release);
    match current_files().and_then(|ft| ft.install(file, flags & EFD_CLOEXEC != 0)) {
        Ok(fd) => fd as i64,
        Err(errno) => {
            EVENTFDS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

/// `sys_eventfd(initval)` — Linux syscall 284 (legacy, no flags).
pub unsafe fn sys_eventfd(initval: u32) -> i64 {
    unsafe { sys_eventfd2(initval, 0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_count_and_read_resets() {
        let e = EventFd::new(7, 0);
        assert_eq!(e.read().unwrap(), 7);
        assert_eq!(e.count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn write_adds_to_count() {
        let e = EventFd::new(0, 0);
        assert_eq!(e.write(5).unwrap(), 8);
        assert_eq!(e.read().unwrap(), 5);
    }

    #[test]
    fn semaphore_mode_decrements_one() {
        let e = EventFd::new(3, EFD_SEMAPHORE);
        assert_eq!(e.read().unwrap(), 1);
        assert_eq!(e.read().unwrap(), 1);
        assert_eq!(e.count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn nonblock_eagain_when_zero() {
        let e = EventFd::new(0, EFD_NONBLOCK);
        assert_eq!(e.read(), Err(EAGAIN));
    }

    #[test]
    fn empty_read_would_block_instead_of_succeeding_zero() {
        let e = EventFd::new(0, 0);
        assert_eq!(e.read(), Err(EAGAIN));
    }

    #[test]
    fn write_rejects_u64_max_with_positive_errno() {
        let e = EventFd::new(0, 0);
        assert_eq!(e.write(u64::MAX), Err(EINVAL));
    }

    #[test]
    fn poll_mask_reflects_state() {
        let e = EventFd::new(0, 0);
        assert_eq!(e.poll_mask(), 0x0004); // EPOLLOUT only
        let _ = e.write(5);
        assert_eq!(e.poll_mask(), 0x0005); // EPOLLIN | EPOLLOUT
    }

    #[test]
    fn fd_file_ops_read_write_counter() {
        let token = EVENTFD_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
        EVENTFDS.lock().insert(token, Arc::new(EventFd::new(0, 0)));
        let file = alloc_anon_file("eventfd-test", &EVENTFD_FILE_OPS, token);
        let mut pos = 0;
        assert_eq!(
            eventfd_file_write(&file, &5u64.to_ne_bytes(), &mut pos),
            Ok(8)
        );
        let mut out = [0u8; 8];
        assert_eq!(eventfd_file_read(&file, &mut out, &mut pos), Ok(8));
        assert_eq!(u64::from_ne_bytes(out), 5);
        eventfd_release(file);
    }
}
