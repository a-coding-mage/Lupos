//! linux-parity: partial
//! linux-source: vendor/linux/fs/select.c
//! test-origin: linux:vendor/linux/fs/select.c
//! `select(2)` and `poll(2)` readiness helpers.
//!
//! Ref: `vendor/linux/fs/select.c`

extern crate alloc;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EBADF, EFAULT};
use crate::kernel::sched::wait::{WaitQueueCallback, WaitQueueHead};
use crate::kernel::task::{TaskStruct, task_state};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::fdtable::FilesStruct;
use super::file::{fget, fput};
use super::types::FileRef;

pub const POLLIN: i16 = 0x0001;
pub const POLLPRI: i16 = 0x0002;
pub const POLLOUT: i16 = 0x0004;
pub const POLLERR: i16 = 0x0008;
// vendor/linux/include/uapi/asm-generic/poll.h
pub const POLLHUP: i16 = 0x0010;
pub const POLLNVAL: i16 = 0x0020;
pub const POLLRDNORM: i16 = 0x0040;
pub const POLLRDBAND: i16 = 0x0080;
pub const POLLWRNORM: i16 = 0x0100;
pub const POLLWRBAND: i16 = 0x0200;

const SELECT_READ_MASK: u32 =
    (POLLIN | POLLRDNORM | POLLRDBAND | POLLHUP | POLLERR | POLLNVAL) as u32;
const SELECT_WRITE_MASK: u32 = (POLLOUT | POLLWRNORM | POLLWRBAND | POLLERR | POLLNVAL) as u32;
const SELECT_EXCEPT_MASK: u32 = (POLLPRI | POLLNVAL) as u32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

struct PollTableEntry {
    file: FileRef,
    queue: *const WaitQueueHead,
}

/// Rust-native `struct poll_wqueues` / `poll_table`.
///
/// A file's poll callback calls [`poll_wait`] before sampling readiness.  Each
/// entry pins the file for as long as the current task is installed on the
/// waitqueue, matching `fs/select.c::__pollwait()` and `poll_freewait()`.
pub struct PollTable {
    owner: PollTableOwner,
    entries: Vec<PollTableEntry>,
    /// Linux `poll_wqueues.triggered`: once any registered queue wakes this
    /// remains set until the sleep cycle has observed it.
    triggered: Arc<AtomicBool>,
    wait_calls: usize,
    unregistered_sources: bool,
}

#[derive(Clone, Copy)]
enum PollTableOwner {
    Task(*mut TaskStruct),
    Callback {
        id: usize,
        callback: WaitQueueCallback,
        data1: usize,
        data2: usize,
    },
}

// Persistent eventpoll tables live inside EpItem objects protected by the
// EventPoll mutex. Their raw waitqueue pointers remain valid through the FileRef
// pin in each PollTableEntry and are removed before those pins are dropped.
unsafe impl Send for PollTable {}

impl PollTable {
    pub fn new(task: *mut TaskStruct) -> Self {
        Self {
            owner: PollTableOwner::Task(task),
            entries: Vec::new(),
            triggered: Arc::new(AtomicBool::new(false)),
            wait_calls: 0,
            unregistered_sources: false,
        }
    }

    pub fn new_callback(
        id: usize,
        callback: WaitQueueCallback,
        data1: usize,
        data2: usize,
    ) -> Self {
        Self {
            owner: PollTableOwner::Callback {
                id,
                callback,
                data1,
                data2,
            },
            entries: Vec::new(),
            triggered: Arc::new(AtomicBool::new(false)),
            wait_calls: 0,
            unregistered_sources: false,
        }
    }

    fn register(&mut self, file: &FileRef, queue: &WaitQueueHead) {
        self.wait_calls = self.wait_calls.saturating_add(1);
        if matches!(self.owner, PollTableOwner::Task(task) if task.is_null()) {
            return;
        }

        let queue_ptr = queue as *const WaitQueueHead;
        // WaitQueueHead stores task pointers rather than Linux wait entries,
        // so one task can only be linked once on a given queue.  Keep the
        // poll-table side equally unique and avoid unbalanced fget/fput pins.
        if self.entries.iter().any(|entry| entry.queue == queue_ptr) {
            return;
        }

        self.entries.push(PollTableEntry {
            file: fget(file),
            queue: queue_ptr,
        });
        match self.owner {
            PollTableOwner::Task(task) => unsafe {
                queue.add_poll_wait(task, self.triggered.clone());
            },
            PollTableOwner::Callback {
                id,
                callback,
                data1,
                data2,
            } => queue.add_callback(id, callback, data1, data2),
        }
    }

    pub fn has_registrations(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Number of persistent waitqueue entries, and therefore FileRef pins,
    /// owned by this poll table. Eventpoll's final-close accounting discounts
    /// each of these implementation references from `File::f_count`.
    pub fn registration_count(&self) -> usize {
        self.entries.len()
    }

    pub fn has_unregistered_sources(&self) -> bool {
        self.unregistered_sources
    }

    /// Linux `poll_schedule_timeout()` state/trigger handshake.  The SeqCst
    /// operations provide the full barrier pairing between the readiness
    /// producer and this last check before `schedule()`.
    pub fn prepare_to_sleep(&self) -> bool {
        let PollTableOwner::Task(task) = self.owner else {
            return false;
        };
        if task.is_null() {
            return false;
        }
        unsafe {
            (*task)
                .__state
                .store(task_state::TASK_INTERRUPTIBLE, Ordering::SeqCst);
        }
        if self.triggered.load(Ordering::SeqCst) {
            unsafe {
                (*task)
                    .__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
            false
        } else {
            true
        }
    }

    /// Remove all installed wait entries and drop their file pins.
    pub fn finish(&mut self) {
        for entry in self.entries.drain(..) {
            // The file pin held by the entry keeps the file-owned waitqueue
            // alive through removal, just as poll_freewait() calls
            // remove_wait_queue() before fput().
            match self.owner {
                PollTableOwner::Task(task) => unsafe {
                    (&*entry.queue).finish_wait(task);
                },
                PollTableOwner::Callback { id, .. } => unsafe {
                    (&*entry.queue).remove_callback(id);
                },
            }
            fput(entry.file);
        }
    }
}

impl Drop for PollTable {
    fn drop(&mut self) {
        self.finish();
    }
}

/// Register the polling task on a file waitqueue.
///
/// Poll callbacks invoke this before testing their readiness condition, which
/// closes the check/sleep race in the same order as Linux `poll_wait()`.
pub fn poll_wait(file: &FileRef, queue: &WaitQueueHead, table: Option<&mut PollTable>) {
    if let Some(table) = table {
        table.register(file, queue);
    }
}

pub fn poll_mask(file: &FileRef) -> u32 {
    poll_mask_with_table(file, None)
}

pub fn poll_mask_with_table(file: &FileRef, table: Option<&mut PollTable>) -> u32 {
    let fallback = || {
        let mut fallback = 0u32;
        if file.fops.read.is_some() {
            fallback |= (POLLIN | POLLRDNORM) as u32;
        }
        if file.fops.write.is_some() {
            fallback |= (POLLOUT | POLLWRNORM) as u32;
        }
        fallback
    };
    let Some(poll) = file.fops.poll else {
        return fallback();
    };
    match table {
        Some(table) => {
            let before = table.wait_calls;
            let mask = poll(file, Some(table));
            if table.wait_calls == before {
                table.unregistered_sources = true;
            }
            mask
        }
        None => poll(file, None),
    }
}

pub unsafe fn poll_once(ft: &FilesStruct, fds: *mut PollFd, nfds: usize) -> Result<i64, i32> {
    unsafe { poll_once_with_table(ft, fds, nfds, None) }
}

pub unsafe fn poll_once_with_table(
    ft: &FilesStruct,
    fds: *mut PollFd,
    nfds: usize,
    mut table: Option<&mut PollTable>,
) -> Result<i64, i32> {
    if nfds != 0 && fds.is_null() {
        return Err(EFAULT);
    }
    let mut ready = 0i64;
    for idx in 0..nfds {
        let user_pfd = unsafe { fds.add(idx) };
        let mut pfd = PollFd::default();
        let not_copied = unsafe {
            uaccess::copy_from_user(
                (&mut pfd as *mut PollFd).cast::<u8>(),
                user_pfd.cast::<u8>(),
                core::mem::size_of::<PollFd>(),
            )
        };
        if not_copied != 0 {
            return Err(EFAULT);
        }
        pfd.revents = 0;
        if pfd.fd >= 0 {
            match ft.get(pfd.fd) {
                Ok(file) => {
                    let mask = poll_mask_with_table(&file, table.as_deref_mut());
                    // Linux always reports ERR/HUP/NVAL, even when userspace
                    // did not request those bits.
                    pfd.revents =
                        (pfd.events & mask as i16) | (mask as i16 & (POLLERR | POLLHUP | POLLNVAL));
                    if pfd.revents != 0 {
                        ready += 1;
                    }
                }
                Err(_) => {
                    pfd.revents = POLLNVAL;
                    ready += 1;
                }
            }
        }
        let not_copied = unsafe {
            uaccess::copy_to_user(
                user_pfd.cast::<u8>(),
                (&pfd as *const PollFd).cast::<u8>(),
                core::mem::size_of::<PollFd>(),
            )
        };
        if not_copied != 0 {
            return Err(EFAULT);
        }
    }
    Ok(ready)
}

pub unsafe fn fdset_is_set(set: *const u64, fd: usize) -> bool {
    if set.is_null() {
        return false;
    }
    unsafe { (*set.add(fd / 64) & (1u64 << (fd % 64))) != 0 }
}

pub unsafe fn fdset_assign(set: *mut u64, fd: usize, value: bool) {
    if set.is_null() {
        return;
    }
    let word = unsafe { set.add(fd / 64) };
    let mask = 1u64 << (fd % 64);
    unsafe {
        if value {
            *word |= mask;
        } else {
            *word &= !mask;
        }
    }
}

pub unsafe fn select_once(
    ft: &FilesStruct,
    nfds: i32,
    readfds: *mut u64,
    writefds: *mut u64,
    exceptfds: *mut u64,
) -> Result<i64, i32> {
    unsafe { select_once_with_table(ft, nfds, readfds, writefds, exceptfds, None) }
}

pub unsafe fn select_once_with_table(
    ft: &FilesStruct,
    nfds: i32,
    readfds: *mut u64,
    writefds: *mut u64,
    exceptfds: *mut u64,
    mut table: Option<&mut PollTable>,
) -> Result<i64, i32> {
    if nfds < 0 {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    let mut ready = 0i64;
    for fd in 0..nfds as usize {
        let want_read = unsafe { fdset_is_set(readfds, fd) };
        let want_write = unsafe { fdset_is_set(writefds, fd) };
        let want_except = unsafe { fdset_is_set(exceptfds, fd) };
        if !want_read && !want_write && !want_except {
            continue;
        }
        let file = ft.get(fd as i32).map_err(|_| EBADF)?;
        let mask = poll_mask_with_table(&file, table.as_deref_mut());
        let read_ready = want_read && mask & SELECT_READ_MASK != 0;
        let write_ready = want_write && mask & SELECT_WRITE_MASK != 0;
        let except_ready = want_except && mask & SELECT_EXCEPT_MASK != 0;
        unsafe {
            fdset_assign(readfds, fd, read_ready);
            fdset_assign(writefds, fd, write_ready);
            fdset_assign(exceptfds, fd, except_ready);
        }
        if read_ready {
            ready += 1;
        }
        if write_ready {
            ready += 1;
        }
        if except_ready {
            ready += 1;
        }
    }
    Ok(ready)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::fdtable::FilesStruct;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::{FileOps, NOOP_FILE_OPS};

    static READABLE_OPS: FileOps = FileOps {
        name: "readable",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(|_, _| POLLIN as u32),
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    static WRITABLE_OPS: FileOps = FileOps {
        name: "writable",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: Some(|_, _| POLLOUT as u32),
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    #[test]
    fn poll_once_reports_read_write_and_unsupported_masks() {
        let ft = FilesStruct::new();
        let rfd = ft
            .install(alloc_file(d_alloc("r"), 0, 0, &READABLE_OPS), false)
            .unwrap();
        let wfd = ft
            .install(alloc_file(d_alloc("w"), 0, 0, &WRITABLE_OPS), false)
            .unwrap();
        let nfd = ft
            .install(alloc_file(d_alloc("n"), 0, 0, &NOOP_FILE_OPS), false)
            .unwrap();

        let mut fds = [
            PollFd {
                fd: rfd,
                events: POLLIN | POLLOUT,
                revents: 0,
            },
            PollFd {
                fd: wfd,
                events: POLLIN | POLLOUT,
                revents: 0,
            },
            PollFd {
                fd: nfd,
                events: POLLIN | POLLOUT,
                revents: 0,
            },
        ];
        let ready = unsafe { poll_once(&ft, fds.as_mut_ptr(), fds.len()) }.unwrap();
        assert_eq!(ready, 2);
        assert_eq!(fds[0].revents, POLLIN);
        assert_eq!(fds[1].revents, POLLOUT);
        assert_eq!(fds[2].revents, 0);
    }
}
