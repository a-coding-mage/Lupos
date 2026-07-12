//! linux-parity: partial
//! linux-source: vendor/linux/fs/timerfd.c
//! test-origin: linux:vendor/linux/fs/timerfd.c
//! timerfd syscall glue.
//!
//! Bridges `kernel::time::timerfd` objects into VFS file descriptors.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EAGAIN, EBADF, EFAULT, EINVAL};
use crate::include::uapi::fcntl::O_NONBLOCK;
use crate::kernel::task::{TaskStruct, task_state};
use crate::kernel::time::{Itimerspec64, TimerFd};
use crate::kernel::{files, sched};

use super::anon_inode::alloc_anon_file;
use super::ops::FileOps;
use super::types::FileRef;

static TIMERFD_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref TIMERFDS: Mutex<BTreeMap<usize, Arc<TimerFd>>> = Mutex::new(BTreeMap::new());
}

static TIMERFD_FILE_OPS: FileOps = FileOps {
    name: "timerfd",
    read: Some(timerfd_file_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(timerfd_file_poll),
    ioctl: None,
    mmap: None,
    release: Some(timerfd_release),
    readdir: None,
};

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn timerfd_from_fd(fd: i32) -> Result<Arc<TimerFd>, i32> {
    let file = current_files()?.get(fd)?;
    if file.fops.name != TIMERFD_FILE_OPS.name {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    TIMERFDS.lock().get(&token).cloned().ok_or(EBADF)
}

fn timerfd_file_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.len() < core::mem::size_of::<u64>() {
        return Err(EINVAL);
    }
    let token = *file.private.lock();
    let tfd = TIMERFDS.lock().get(&token).cloned().ok_or(EBADF)?;
    let task = unsafe { sched::get_current() };

    loop {
        // Linux's wait_event_interruptible_locked_irq() links the reader
        // before its final ctx->ticks test.  Preserve that order so an expiry
        // between the test and schedule cannot be lost.
        if !task.is_null() {
            unsafe {
                tfd.wait
                    .prepare_to_wait(task, task_state::TASK_INTERRUPTIBLE);
            }
        }

        let ticks = crate::kernel::time::timerfd::timerfd_read(&tfd);
        if ticks != 0 {
            finish_timerfd_wait(&tfd, task);
            buf[..8].copy_from_slice(&ticks.to_ne_bytes());
            return Ok(8);
        }

        if file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0 || task.is_null() {
            finish_timerfd_wait(&tfd, task);
            return Err(EAGAIN);
        }

        // wait_event_interruptible() reports -ERESTARTSYS when an unblocked
        // signal wins the wait.  The syscall-exit signal code performs the
        // Linux restart/EINTR conversion from this internal errno.
        if crate::kernel::signal::has_unblocked_pending_signals(task) {
            finish_timerfd_wait(&tfd, task);
            return Err(512);
        }

        unsafe {
            sched::schedule_with_irqs_enabled();
        }
        finish_timerfd_wait(&tfd, task);
    }
}

fn finish_timerfd_wait(tfd: &TimerFd, task: *mut TaskStruct) {
    if !task.is_null() {
        unsafe {
            tfd.wait.finish_wait(task);
        }
    }
}

fn timerfd_file_poll(file: &FileRef, table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    let token = *file.private.lock();
    let Some(tfd) = TIMERFDS.lock().get(&token).cloned() else {
        return crate::fs::select::POLLERR as u32;
    };

    // `timerfd_poll()` registers first, then samples ticks.  An expiry either
    // becomes visible to this load or wakes the installed poll/epoll waiter.
    crate::fs::select::poll_wait(file, &tfd.wait, table);
    if tfd.has_ticks() {
        crate::fs::select::POLLIN as u32
    } else {
        0
    }
}

fn timerfd_release(file: FileRef) {
    let token = *file.private.lock();
    // Removing the registry's Arc lets the final in-flight file operation or
    // poll pin determine lifetime.  TimerFd::drop performs synchronous
    // hrtimer cancellation before the object and waitqueue are freed.
    TIMERFDS.lock().remove(&token);
}

fn copy_itimerspec_to_user(dst: *mut Itimerspec64, value: &Itimerspec64) -> Result<(), i32> {
    if dst.is_null() {
        return Err(EFAULT);
    }
    let not_copied = unsafe {
        uaccess::copy_to_user(
            dst as *mut u8,
            value as *const Itimerspec64 as *const u8,
            core::mem::size_of::<Itimerspec64>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

pub unsafe fn sys_timerfd_create(clockid: i32, flags: i32) -> i64 {
    let allowed = crate::kernel::time::timerfd::TFD_CLOEXEC
        | crate::kernel::time::timerfd::TFD_NONBLOCK
        | crate::kernel::time::timerfd::TFD_TIMER_ABSTIME
        | crate::kernel::time::timerfd::TFD_TIMER_CANCEL_ON_SET;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    let tfd = match crate::kernel::time::timerfd::sys_timerfd_create(clockid, flags) {
        Ok(tfd) => tfd,
        Err(errno) => return -(errno as i64),
    };
    let token = TIMERFD_TOKEN.fetch_add(1, Ordering::AcqRel);
    TIMERFDS.lock().insert(token, tfd);
    let file = alloc_anon_file("timerfd", &TIMERFD_FILE_OPS, token);
    file.flags
        .store((flags as u32) & O_NONBLOCK, Ordering::Release);
    match current_files()
        .and_then(|ft| ft.install(file, flags & crate::kernel::time::timerfd::TFD_CLOEXEC != 0))
    {
        Ok(fd) => fd as i64,
        Err(errno) => {
            TIMERFDS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

pub unsafe fn sys_timerfd_settime(
    fd: i32,
    flags: i32,
    new_value: *const Itimerspec64,
    old_value: *mut Itimerspec64,
) -> i64 {
    if new_value.is_null() {
        return -(EFAULT as i64);
    }
    let tfd = match timerfd_from_fd(fd) {
        Ok(tfd) => tfd,
        Err(errno) => return -(errno as i64),
    };
    let new_value = unsafe { *new_value };
    let old = match crate::kernel::time::timerfd::sys_timerfd_settime(&tfd, flags, new_value) {
        Ok(old) => old,
        Err(errno) => return -(errno as i64),
    };
    if !old_value.is_null() {
        if let Err(errno) = copy_itimerspec_to_user(old_value, &old) {
            return -(errno as i64);
        }
    }
    0
}

pub unsafe fn sys_timerfd_gettime(fd: i32, curr_value: *mut Itimerspec64) -> i64 {
    let tfd = match timerfd_from_fd(fd) {
        Ok(tfd) => tfd,
        Err(errno) => return -(errno as i64),
    };
    let cur = crate::kernel::time::timerfd::sys_timerfd_gettime(&tfd);
    match copy_itimerspec_to_user(curr_value, &cur) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::time::CLOCK_MONOTONIC;
    use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};
    use alloc::boxed::Box;

    #[test]
    fn timerfd_file_read_exports_u64_ticks() {
        let tfd = crate::kernel::time::timerfd::sys_timerfd_create(CLOCK_MONOTONIC, 0).unwrap();
        tfd.set_pending_for_test(false, 3);
        let token = TIMERFD_TOKEN.fetch_add(1, Ordering::AcqRel);
        TIMERFDS.lock().insert(token, tfd);
        let file = alloc_anon_file("timerfd-test", &TIMERFD_FILE_OPS, token);
        let mut buf = [0u8; 8];
        let mut pos = 0;
        assert_eq!(timerfd_file_read(&file, &mut buf, &mut pos), Ok(8));
        assert_eq!(u64::from_ne_bytes(buf), 3);
        assert_eq!(timerfd_file_read(&file, &mut buf, &mut pos), Err(EAGAIN));
        TIMERFDS.lock().remove(&token);
    }

    #[test]
    fn timerfd_create_nonblock_sets_file_flag_and_empty_read_eagain() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 286;
        current.tgid = 286;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(
                &mut *current as *mut TaskStruct,
                crate::fs::fdtable::FilesStruct::new(),
            );
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = sys_timerfd_create(
                CLOCK_MONOTONIC,
                crate::kernel::time::timerfd::TFD_CLOEXEC
                    | crate::kernel::time::timerfd::TFD_NONBLOCK,
            );
            assert!(fd >= 0);
            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let file = ft.get(fd as i32).unwrap();
            assert_eq!(file.flags.load(Ordering::Acquire) & O_NONBLOCK, O_NONBLOCK);

            let mut buf = [0u8; 8];
            let mut pos = 0;
            assert_eq!(timerfd_file_read(&file, &mut buf, &mut pos), Err(EAGAIN));

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn copy_itimerspec_rejects_null() {
        let spec = Itimerspec64::default();
        assert_eq!(
            copy_itimerspec_to_user(core::ptr::null_mut(), &spec),
            Err(EFAULT)
        );
    }
}
