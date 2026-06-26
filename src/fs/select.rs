//! linux-parity: complete
//! linux-source: vendor/linux/fs/select.c
//! test-origin: linux:vendor/linux/fs/select.c
//! `select(2)` and `poll(2)` readiness helpers.
//!
//! Ref: `vendor/linux/fs/select.c`

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EBADF, EFAULT};

use super::fdtable::FilesStruct;
use super::types::FileRef;

pub const POLLIN: i16 = 0x0001;
pub const POLLOUT: i16 = 0x0004;
pub const POLLERR: i16 = 0x0008;
// vendor/linux/include/uapi/asm-generic/poll.h
pub const POLLHUP: i16 = 0x0010;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub fn poll_mask(file: &FileRef) -> u32 {
    file.fops.poll.map(|poll| poll(file)).unwrap_or_else(|| {
        let mut fallback = 0u32;
        if file.fops.read.is_some() {
            fallback |= POLLIN as u32;
        }
        if file.fops.write.is_some() {
            fallback |= POLLOUT as u32;
        }
        fallback
    })
}

pub unsafe fn poll_once(ft: &FilesStruct, fds: *mut PollFd, nfds: usize) -> Result<i64, i32> {
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
                    let mask = poll_mask(&file);
                    pfd.revents = pfd.events & mask as i16;
                    if pfd.revents != 0 {
                        ready += 1;
                    }
                }
                Err(_) => {
                    pfd.revents = POLLERR;
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
        let mask = poll_mask(&file);
        let read_ready = want_read && mask & POLLIN as u32 != 0;
        let write_ready = want_write && mask & POLLOUT as u32 != 0;
        unsafe {
            fdset_assign(readfds, fd, read_ready);
            fdset_assign(writefds, fd, write_ready);
            fdset_assign(exceptfds, fd, false);
        }
        if read_ready {
            ready += 1;
        }
        if write_ready {
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
        poll: Some(|_| POLLIN as u32),
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
        poll: Some(|_| POLLOUT as u32),
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
