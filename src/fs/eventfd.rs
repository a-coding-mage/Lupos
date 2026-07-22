//! linux-parity: partial
//! linux-source: vendor/linux/fs/eventfd.c
//! test-origin: linux:vendor/linux/fs/eventfd.c
//! eventfd — counter-style fd for thread/process notifications.
//!
//! Counter, poll readiness, and file-level blocking semantics mirror
//! vendor/linux/fs/eventfd.c.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::select::{self, POLLERR, POLLIN, POLLOUT, PollTable};
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EAGAIN, EBADF, EINVAL};
use crate::include::uapi::fcntl::{O_NONBLOCK, O_RDWR};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::sched::wait::WaitQueueHead;
use crate::kernel::task::{TaskStruct, task_state};
use crate::kernel::{files, sched};

const ERESTARTSYS: i32 = 512;

/// `EFD_*` flags — byte-identical to Linux UAPI.
pub const EFD_SEMAPHORE: i32 = 0o0000001;
pub const EFD_CLOEXEC: i32 = 0o2000000;
pub const EFD_NONBLOCK: i32 = 0o0004000;

/// In-kernel state for one eventfd.
pub struct EventFd {
    pub count: AtomicU64,
    pub flags: i32,
    wqh: WaitQueueHead,
}

static EVENTFD_TOKEN: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref EVENTFDS: Mutex<BTreeMap<usize, Arc<EventFd>>> = Mutex::new(BTreeMap::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("eventfd_ctx_fdget", linux_eventfd_ctx_fdget as usize, true);
    export_symbol_once("eventfd_ctx_put", linux_eventfd_ctx_put as usize, true);
    export_symbol_once(
        "eventfd_signal_mask",
        linux_eventfd_signal_mask as usize,
        true,
    );
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
            wqh: WaitQueueHead::new(),
        }
    }

    /// Read semantics: returns current count and resets it (or decrements by 1
    /// in EFD_SEMAPHORE mode).  Returns EAGAIN when a read would block.
    /// Linux: vendor/linux/fs/eventfd.c::eventfd_read
    pub fn read(&self) -> Result<u64, i32> {
        loop {
            let count = self.count.load(Ordering::Acquire);
            if count == 0 {
                return Err(EAGAIN);
            }
            let value = if self.flags & EFD_SEMAPHORE != 0 {
                1
            } else {
                count
            };
            if self
                .count
                .compare_exchange(count, count - value, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // eventfd_read() wakes EPOLLOUT after consuming the counter.
                self.wqh.wake_up_all();
                return Ok(value);
            }
        }
    }

    /// Write semantics: adds value to count.  Caps at u64::MAX-1.
    pub fn write(&self, val: u64) -> Result<usize, i32> {
        if val == u64::MAX {
            return Err(EINVAL);
        }
        loop {
            let count = self.count.load(Ordering::Acquire);
            if u64::MAX - count <= val {
                return Err(EAGAIN);
            }
            if self
                .count
                .compare_exchange(count, count + val, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // eventfd_write() wakes EPOLLIN after every successful write,
                // including a write of zero.
                self.wqh.wake_up_all();
                return Ok(8);
            }
        }
    }

    /// poll() mask — EPOLLIN if readable, EPOLLOUT if writable.
    pub fn poll_mask(&self) -> u32 {
        let mut mask = 0u32;
        let v = self.count.load(Ordering::Acquire);
        if v > 0 {
            mask |= POLLIN as u32;
        }
        if v == u64::MAX {
            mask |= POLLERR as u32;
        }
        if v < u64::MAX - 1 {
            mask |= POLLOUT as u32;
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

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

fn is_err_ptr<T>(ptr: *mut T) -> bool {
    (ptr as usize) >= usize::MAX - 4095
}

unsafe fn eventfd_arc_from_raw(ctx: *mut EventFd) -> Option<Arc<EventFd>> {
    if ctx.is_null() || is_err_ptr(ctx) {
        return None;
    }
    Some(unsafe { Arc::from_raw(ctx) })
}

extern "C" fn linux_eventfd_ctx_fdget(fd: i32) -> *mut c_void {
    let eventfd = match current_files()
        .and_then(|ft| ft.get(fd))
        .and_then(|file| eventfd_from_file(&file))
    {
        Ok(eventfd) => eventfd,
        Err(errno) => return err_ptr(errno),
    };
    Arc::into_raw(eventfd) as *mut c_void
}

unsafe extern "C" fn linux_eventfd_ctx_put(ctx: *mut EventFd) {
    if let Some(eventfd) = unsafe { eventfd_arc_from_raw(ctx) } {
        drop(eventfd);
    }
}

unsafe extern "C" fn linux_eventfd_signal_mask(ctx: *mut EventFd, _mask: u32) {
    if ctx.is_null() || is_err_ptr(ctx) {
        return;
    }

    let eventfd = unsafe { &*ctx };
    loop {
        let count = eventfd.count.load(Ordering::Acquire);
        if count == u64::MAX {
            break;
        }
        if eventfd
            .count
            .compare_exchange(count, count + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
    }
    eventfd.wqh.wake_up_all();
}

fn eventfd_file_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.len() < 8 {
        return Err(EINVAL);
    }
    let eventfd = eventfd_from_file(file)?;
    let task = unsafe { sched::get_current() };

    loop {
        // Linux wait_event_interruptible_locked_irq() links the waiter before
        // its final ctx->count test.  Preserve that order so a producer cannot
        // update the counter and miss the wakeup between test and sleep.
        if !task.is_null() {
            unsafe {
                eventfd
                    .wqh
                    .prepare_to_wait(task, task_state::TASK_INTERRUPTIBLE);
            }
        }

        match eventfd.read() {
            Ok(value) => {
                finish_eventfd_wait(&eventfd, task);
                buf[..8].copy_from_slice(&value.to_ne_bytes());
                return Ok(8);
            }
            Err(EAGAIN) => {}
            Err(errno) => {
                finish_eventfd_wait(&eventfd, task);
                return Err(errno);
            }
        }

        if file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0 || task.is_null() {
            finish_eventfd_wait(&eventfd, task);
            return Err(EAGAIN);
        }

        if crate::kernel::signal::has_unblocked_pending_signals(task) {
            finish_eventfd_wait(&eventfd, task);
            return Err(ERESTARTSYS);
        }

        unsafe {
            sched::schedule_with_irqs_enabled();
        }
        finish_eventfd_wait(&eventfd, task);
    }
}

fn eventfd_file_write(file: &FileRef, buf: &[u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.len() != 8 {
        return Err(EINVAL);
    }
    let value = u64::from_ne_bytes(buf.try_into().map_err(|_| EINVAL)?);
    let eventfd = eventfd_from_file(file)?;
    let task = unsafe { sched::get_current() };

    loop {
        if !task.is_null() {
            unsafe {
                eventfd
                    .wqh
                    .prepare_to_wait(task, task_state::TASK_INTERRUPTIBLE);
            }
        }

        match eventfd.write(value) {
            Ok(n) => {
                finish_eventfd_wait(&eventfd, task);
                return Ok(n);
            }
            Err(EAGAIN) => {}
            Err(errno) => {
                finish_eventfd_wait(&eventfd, task);
                return Err(errno);
            }
        }

        if file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0 || task.is_null() {
            finish_eventfd_wait(&eventfd, task);
            return Err(EAGAIN);
        }

        if crate::kernel::signal::has_unblocked_pending_signals(task) {
            finish_eventfd_wait(&eventfd, task);
            return Err(ERESTARTSYS);
        }

        unsafe {
            sched::schedule_with_irqs_enabled();
        }
        finish_eventfd_wait(&eventfd, task);
    }
}

fn finish_eventfd_wait(eventfd: &EventFd, task: *mut TaskStruct) {
    if !task.is_null() {
        unsafe {
            eventfd.wqh.finish_wait(task);
        }
    }
}

fn eventfd_file_poll(file: &FileRef, table: Option<&mut PollTable>) -> u32 {
    match eventfd_from_file(file) {
        Ok(eventfd) => {
            // Linux eventfd_poll() registers before reading count so a writer
            // cannot change readiness between the sample and queue insertion.
            select::poll_wait(file, &eventfd.wqh, table);
            eventfd.poll_mask()
        }
        Err(_) => POLLERR as u32,
    }
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
    fn linux_signal_mask_increments_counter() {
        let e = Arc::new(EventFd::new(0, 0));
        let ptr = Arc::into_raw(e.clone()) as *mut EventFd;
        unsafe {
            linux_eventfd_signal_mask(ptr, POLLIN as u32);
            linux_eventfd_ctx_put(ptr);
        }
        assert_eq!(e.read().unwrap(), 1);
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

    #[test]
    fn file_write_requires_exactly_u64_count() {
        let token = EVENTFD_TOKEN.fetch_add(1, Ordering::AcqRel) as usize;
        EVENTFDS.lock().insert(token, Arc::new(EventFd::new(0, 0)));
        let file = alloc_anon_file("eventfd-write-size-test", &EVENTFD_FILE_OPS, token);
        let mut pos = 0;

        assert_eq!(eventfd_file_write(&file, &[0u8; 4], &mut pos), Err(EINVAL));
        assert_eq!(eventfd_file_write(&file, &[0u8; 16], &mut pos), Err(EINVAL));

        eventfd_release(file);
    }
}
