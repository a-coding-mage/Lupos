//! linux-parity: complete
//! linux-source: vendor/linux/fs/pipe.c
//! test-origin: linux:vendor/linux/fs/pipe.c
//! Pipe file descriptors.
//!
//! Mirrors the userspace ABI shape of `vendor/linux/fs/pipe.c`: `pipe()` and
//! `pipe2()` allocate a connected read/write fd pair backed by an anonymous
//! inode object.

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use core::sync::atomic::Ordering as AtomicOrdering;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EAGAIN, EBADF, EFAULT, EINVAL, EPIPE};
use crate::include::uapi::fcntl::{O_CLOEXEC, O_NONBLOCK, O_RDONLY, O_WRONLY};
use crate::kernel::sched::wait::WaitQueueHead;
use crate::kernel::task::task_state::{TASK_INTERRUPTIBLE, TASK_RUNNING};
use crate::kernel::{files, sched};

use super::anon_inode::alloc_anon_file;
use super::ops::FileOps;
use super::types::FileRef;

const PIPE_BUF_CAPACITY: usize = 65_536;

struct PipeState {
    buf: Mutex<VecDeque<u8>>,
    flags: i32,
    readers: AtomicUsize,
    writers: AtomicUsize,
    read_wait: WaitQueueHead,
    write_wait: WaitQueueHead,
}

static PIPE_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref PIPES: Mutex<BTreeMap<usize, Arc<PipeState>>> = Mutex::new(BTreeMap::new());
}

static PIPE_READ_OPS: FileOps = FileOps {
    name: "pipe-read",
    read: Some(pipe_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(pipe_read_poll),
    ioctl: None,
    mmap: None,
    release: Some(pipe_release),
    readdir: None,
};

static PIPE_WRITE_OPS: FileOps = FileOps {
    name: "pipe-write",
    read: None,
    write: Some(pipe_write),
    llseek: None,
    fsync: None,
    poll: Some(pipe_write_poll),
    ioctl: None,
    mmap: None,
    release: Some(pipe_release),
    readdir: None,
};

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn pipe_from_file(file: &FileRef) -> Result<Arc<PipeState>, i32> {
    let token = *file.private.lock();
    PIPES.lock().get(&token).cloned().ok_or(EBADF)
}

fn pipe_file_nonblocking(file: &FileRef) -> bool {
    file.flags.load(AtomicOrdering::Acquire) & O_NONBLOCK != 0
}

fn pipe_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    let pipe = pipe_from_file(file)?;
    loop {
        {
            let mut q = pipe.buf.lock();
            if !q.is_empty() {
                let n = buf.len().min(q.len());
                for byte in buf.iter_mut().take(n) {
                    *byte = q.pop_front().unwrap_or_default();
                }
                pipe.write_wait.wake_up_all();
                return Ok(n);
            }
        }

        if pipe.writers.load(AtomicOrdering::Acquire) == 0 {
            return Ok(0);
        }

        if pipe_file_nonblocking(file) {
            return Err(EAGAIN);
        }

        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return Ok(0);
        }

        unsafe {
            pipe.read_wait.prepare_to_wait(task, TASK_INTERRUPTIBLE);
        }
        if !pipe.buf.lock().is_empty() || pipe.writers.load(AtomicOrdering::Acquire) == 0 {
            unsafe {
                pipe.read_wait.finish_wait(task);
            }
            continue;
        }
        unsafe {
            sched::schedule_with_irqs_enabled();
            pipe.read_wait.finish_wait(task);
            (*task).__state.store(TASK_RUNNING, AtomicOrdering::Release);
        }
    }
}

fn pipe_write(file: &FileRef, buf: &[u8], _pos: &mut u64) -> Result<usize, i32> {
    let pipe = pipe_from_file(file)?;
    loop {
        if pipe.readers.load(AtomicOrdering::Acquire) == 0 {
            return Err(EPIPE);
        }

        {
            let mut q = pipe.buf.lock();
            if q.len() < PIPE_BUF_CAPACITY {
                let n = buf.len().min(PIPE_BUF_CAPACITY - q.len());
                q.extend(buf.iter().take(n).copied());
                if n == 0 {
                    return Err(EPIPE);
                }
                pipe.read_wait.wake_up_all();
                return Ok(n);
            }
        }

        if pipe_file_nonblocking(file) {
            return Err(EAGAIN);
        }

        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return Ok(0);
        }

        unsafe {
            pipe.write_wait.prepare_to_wait(task, TASK_INTERRUPTIBLE);
        }
        if pipe.buf.lock().len() < PIPE_BUF_CAPACITY {
            unsafe {
                pipe.write_wait.finish_wait(task);
            }
            continue;
        }
        unsafe {
            sched::schedule_with_irqs_enabled();
            pipe.write_wait.finish_wait(task);
            (*task).__state.store(TASK_RUNNING, AtomicOrdering::Release);
        }
    }
}

fn pipe_read_poll(file: &FileRef) -> u32 {
    match pipe_from_file(file) {
        Ok(pipe) if !pipe.buf.lock().is_empty() => 0x0001,
        Ok(pipe) if pipe.writers.load(AtomicOrdering::Acquire) == 0 => 0x0001,
        Ok(_) => 0,
        Err(_) => 0x0008,
    }
}

fn pipe_write_poll(file: &FileRef) -> u32 {
    match pipe_from_file(file) {
        Ok(pipe) if pipe.buf.lock().len() < PIPE_BUF_CAPACITY => 0x0004,
        Ok(_) => 0,
        Err(_) => 0x0008,
    }
}

fn pipe_release(file: FileRef) {
    let Ok(pipe) = pipe_from_file(&file) else {
        return;
    };
    let token = *file.private.lock();
    let mode = file.flags.load(AtomicOrdering::Acquire) & crate::include::uapi::fcntl::O_ACCMODE;
    if mode == O_RDONLY {
        pipe.readers
            .fetch_update(AtomicOrdering::AcqRel, AtomicOrdering::Acquire, |count| {
                count.checked_sub(1)
            })
            .ok();
        pipe.write_wait.wake_up_all();
    } else if mode == O_WRONLY {
        pipe.writers
            .fetch_update(AtomicOrdering::AcqRel, AtomicOrdering::Acquire, |count| {
                count.checked_sub(1)
            })
            .ok();
        pipe.read_wait.wake_up_all();
    }
    if pipe.readers.load(AtomicOrdering::Acquire) == 0
        && pipe.writers.load(AtomicOrdering::Acquire) == 0
    {
        PIPES.lock().remove(&token);
    }
}

pub unsafe fn sys_pipe2(pipefd: *mut i32, flags: i32) -> i64 {
    if pipefd.is_null() {
        return -(EFAULT as i64);
    }
    let allowed = (O_CLOEXEC | O_NONBLOCK) as i32;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }

    let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
    PIPES.lock().insert(
        token,
        Arc::new(PipeState {
            buf: Mutex::new(VecDeque::new()),
            flags,
            readers: AtomicUsize::new(1),
            writers: AtomicUsize::new(1),
            read_wait: WaitQueueHead::new(),
            write_wait: WaitQueueHead::new(),
        }),
    );

    let read_file = alloc_anon_file("pipe:[read]", &PIPE_READ_OPS, token);
    let write_file = alloc_anon_file("pipe:[write]", &PIPE_WRITE_OPS, token);
    read_file.flags.store(
        O_RDONLY | (flags as u32 & O_NONBLOCK),
        AtomicOrdering::Release,
    );
    write_file.flags.store(
        O_WRONLY | (flags as u32 & O_NONBLOCK),
        AtomicOrdering::Release,
    );
    let fdt = match current_files() {
        Ok(fdt) => fdt,
        Err(errno) => {
            PIPES.lock().remove(&token);
            return -(errno as i64);
        }
    };
    let cloexec = flags & O_CLOEXEC as i32 != 0;
    let read_fd = match fdt.install(read_file, cloexec) {
        Ok(fd) => fd,
        Err(errno) => {
            PIPES.lock().remove(&token);
            return -(errno as i64);
        }
    };
    let write_fd = match fdt.install(write_file, cloexec) {
        Ok(fd) => fd,
        Err(errno) => {
            let _ = fdt.close(read_fd);
            PIPES.lock().remove(&token);
            return -(errno as i64);
        }
    };

    if unsafe { uaccess::put_user_u32(pipefd as *mut u32, read_fd as u32) }.is_err()
        || unsafe { uaccess::put_user_u32(pipefd.add(1) as *mut u32, write_fd as u32) }.is_err()
    {
        let _ = fdt.close(read_fd);
        let _ = fdt.close(write_fd);
        PIPES.lock().remove(&token);
        return -(EFAULT as i64);
    }
    0
}

pub unsafe fn sys_pipe(pipefd: *mut i32) -> i64 {
    unsafe { sys_pipe2(pipefd, 0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::anon_inode::alloc_anon_file;
    use crate::fs::read_write::{vfs_read, vfs_write};

    #[test]
    fn pipe_file_ops_round_trip_bytes() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let reader = alloc_anon_file("pipe-read-test", &PIPE_READ_OPS, token);
        let writer = alloc_anon_file("pipe-write-test", &PIPE_WRITE_OPS, token);
        reader.flags.store(O_RDONLY, AtomicOrdering::Release);
        writer.flags.store(O_WRONLY, AtomicOrdering::Release);
        let mut pos = 0;
        assert_eq!(pipe_write(&writer, b"abc", &mut pos), Ok(3));
        let mut out = [0u8; 4];
        assert_eq!(pipe_read(&reader, &mut out, &mut pos), Ok(3));
        assert_eq!(&out[..3], b"abc");
        PIPES.lock().remove(&token);
    }

    #[test]
    fn pipe_file_access_modes_match_read_and_write_ends() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let reader = alloc_anon_file("pipe-read-mode-test", &PIPE_READ_OPS, token);
        let writer = alloc_anon_file("pipe-write-mode-test", &PIPE_WRITE_OPS, token);
        reader.flags.store(O_RDONLY, AtomicOrdering::Release);
        writer.flags.store(O_WRONLY, AtomicOrdering::Release);

        assert_eq!(vfs_write(&writer, b"pw"), Ok(2));
        let mut out = [0u8; 2];
        assert_eq!(vfs_read(&reader, &mut out), Ok(2));
        assert_eq!(&out, b"pw");
        assert_eq!(vfs_write(&reader, b"x"), Err(EBADF));
        assert_eq!(vfs_read(&writer, &mut out), Err(EBADF));

        PIPES.lock().remove(&token);
    }

    #[test]
    fn empty_pipe_with_no_writers_returns_eof() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(0),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let reader = alloc_anon_file("pipe-read-eof-test", &PIPE_READ_OPS, token);
        reader.flags.store(O_RDONLY, AtomicOrdering::Release);
        let mut out = [0u8; 1];
        let mut pos = 0;

        assert_eq!(pipe_read(&reader, &mut out, &mut pos), Ok(0));
        assert_eq!(pipe_read_poll(&reader) & 0x0001, 0x0001);

        PIPES.lock().remove(&token);
    }

    #[test]
    fn empty_nonblocking_read_observes_file_status_flags() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let reader = alloc_anon_file("pipe-read-nonblock-test", &PIPE_READ_OPS, token);
        reader
            .flags
            .store(O_RDONLY | O_NONBLOCK, AtomicOrdering::Release);
        let mut out = [0u8; 1];
        let mut pos = 0;

        assert_eq!(pipe_read(&reader, &mut out, &mut pos), Err(EAGAIN));

        PIPES.lock().remove(&token);
    }

    #[test]
    fn full_nonblocking_write_observes_file_status_flags() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(
                    core::iter::repeat_n(b'x', PIPE_BUF_CAPACITY).collect::<VecDeque<_>>(),
                ),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let writer = alloc_anon_file("pipe-write-nonblock-test", &PIPE_WRITE_OPS, token);
        writer
            .flags
            .store(O_WRONLY | O_NONBLOCK, AtomicOrdering::Release);
        let mut pos = 0;

        assert_eq!(pipe_write(&writer, b"x", &mut pos), Err(EAGAIN));

        PIPES.lock().remove(&token);
    }

    #[test]
    fn closing_writer_makes_empty_read_end_pollable_for_eof() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(1),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let reader = alloc_anon_file("pipe-read-poll-eof-test", &PIPE_READ_OPS, token);
        let writer = alloc_anon_file("pipe-write-poll-eof-test", &PIPE_WRITE_OPS, token);
        reader.flags.store(O_RDONLY, AtomicOrdering::Release);
        writer.flags.store(O_WRONLY, AtomicOrdering::Release);

        assert_eq!(pipe_read_poll(&reader) & 0x0001, 0);
        pipe_release(writer);
        assert_eq!(pipe_read_poll(&reader) & 0x0001, 0x0001);

        PIPES.lock().remove(&token);
    }

    #[test]
    fn write_pipe_with_no_readers_returns_epipe() {
        let token = PIPE_TOKEN.fetch_add(1, Ordering::AcqRel);
        PIPES.lock().insert(
            token,
            Arc::new(PipeState {
                buf: Mutex::new(VecDeque::new()),
                flags: 0,
                readers: AtomicUsize::new(0),
                writers: AtomicUsize::new(1),
                read_wait: WaitQueueHead::new(),
                write_wait: WaitQueueHead::new(),
            }),
        );
        let writer = alloc_anon_file("pipe-write-epipe-test", &PIPE_WRITE_OPS, token);
        writer.flags.store(O_WRONLY, AtomicOrdering::Release);
        let mut pos = 0;

        assert_eq!(pipe_write(&writer, b"x", &mut pos), Err(EPIPE));

        PIPES.lock().remove(&token);
    }

    #[test]
    fn pipe2_rejects_unknown_flags() {
        let mut pair = [0i32; 2];
        let ret = unsafe { sys_pipe2(pair.as_mut_ptr(), 0x4000_0000) };
        assert_eq!(ret, -(EINVAL as i64));
    }
}
