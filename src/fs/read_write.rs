//! linux-parity: partial
//! linux-source: vendor/linux/fs/read_write.c
//! test-origin: linux:vendor/linux/fs/read_write.c
//! `vfs_read`, `vfs_write`, `vfs_lseek`, `vfs_fsync`.
//!
//! Mirrors `vendor/linux/fs/read_write.c`.  Each routine validates the file
//! mode, advances `f_pos`, and dispatches through `file.fops`.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EBADF, EINVAL, EISDIR, ENOSYS};
use crate::include::uapi::fcntl::{O_ACCMODE, O_APPEND, O_PATH, O_RDONLY, O_RDWR, O_WRONLY};

use super::file::note_file_access_for_integrity;
use super::permission::check_file_write_mount;
use super::types::{FileRef, InodeKind};

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;

#[inline]
fn read_allowed(flags: u32) -> bool {
    let m = flags & O_ACCMODE;
    m == O_RDONLY || m == O_RDWR
}
#[inline]
fn write_allowed(flags: u32) -> bool {
    let m = flags & O_ACCMODE;
    m == O_WRONLY || m == O_RDWR
}

pub fn vfs_read(file: &FileRef, buf: &mut [u8]) -> Result<usize, i32> {
    let flags = file.flags.load(core::sync::atomic::Ordering::Acquire);
    if flags & O_PATH != 0 {
        return Err(EBADF);
    }
    if !read_allowed(flags) {
        return Err(EBADF);
    }
    let inode = file.inode().ok_or(EBADF)?;
    if inode.kind == InodeKind::Directory {
        return Err(EISDIR);
    }
    let read = file.fops.read.ok_or(ENOSYS)?;
    note_file_access_for_integrity(None, file);
    if inode.kind == InodeKind::Chardev {
        // Character devices such as consoles may wait/yield for input. Do not
        // hold the per-file position spinlock across that device callback.
        let mut pos = *file.pos.lock();
        let result = read(file, buf, &mut pos);
        if result.is_ok() {
            *file.pos.lock() = pos;
        }
        return result;
    }
    let mut pos = file.pos.lock();
    read(file, buf, &mut *pos)
}

pub fn vfs_write(file: &FileRef, buf: &[u8]) -> Result<usize, i32> {
    let flags = file.flags.load(core::sync::atomic::Ordering::Acquire);
    if flags & O_PATH != 0 {
        return Err(EBADF);
    }
    if !write_allowed(flags) {
        return Err(EBADF);
    }
    let inode = file.inode().ok_or(EBADF)?;
    if inode.kind == InodeKind::Directory {
        return Err(EISDIR);
    }
    check_file_write_mount(&file.dentry, inode.kind)?;
    let write = file.fops.write.ok_or(ENOSYS)?;
    let mut pos = file.pos.lock();
    if flags & O_APPEND != 0 {
        *pos = inode.size.load(core::sync::atomic::Ordering::Acquire);
    }
    write(file, buf, &mut *pos)
}

pub fn vfs_lseek(file: &FileRef, off: i64, whence: i32) -> Result<u64, i32> {
    if file.flags.load(core::sync::atomic::Ordering::Acquire) & O_PATH != 0 {
        return Err(EBADF);
    }
    if let Some(llseek) = file.fops.llseek {
        return llseek(file, off, whence);
    }
    // Generic fallback.
    let inode = file.inode().ok_or(EBADF)?;
    let mut pos = file.pos.lock();
    let new = match whence {
        SEEK_SET => off,
        SEEK_CUR => *pos as i64 + off,
        SEEK_END => inode.size.load(core::sync::atomic::Ordering::Acquire) as i64 + off,
        _ => return Err(EINVAL),
    };
    if new < 0 {
        return Err(EINVAL);
    }
    *pos = new as u64;
    Ok(*pos)
}

pub fn vfs_fsync(file: &FileRef) -> Result<(), i32> {
    if file.flags.load(core::sync::atomic::Ordering::Acquire) & O_PATH != 0 {
        return Err(EBADF);
    }
    if let Some(fsync) = file.fops.fsync {
        fsync(file)
    } else {
        Ok(())
    }
}

/// `sys_write(2)` — userspace entry point.
///
/// Mirrors the Linux shape in `vendor/linux/fs/read_write.c` at the syscall
/// layer: fetch the file from the calling task's fdtable, copy data from user
/// memory, then call `vfs_write`.
///
/// Returns the number of bytes written or `-errno`.
pub unsafe fn sys_write(fd: i32, buf: *const u8, count: usize) -> i64 {
    use crate::arch::x86::kernel::uaccess;
    use crate::include::uapi::errno::EFAULT;
    use crate::kernel::{files, sched};

    if count == 0 {
        return 0;
    }
    if buf.is_null() {
        return -(EFAULT as i64);
    }

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }

    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };
    let file = match ft.get(fd) {
        Ok(f) => f,
        Err(errno) => return -(errno as i64),
    };

    // Chunked copy so we don't try to allocate `count` bytes in one go.
    const CHUNK: usize = 4096;
    let mut written: usize = 0;
    let mut user = buf;
    let mut remaining = count;

    while remaining > 0 {
        let this = remaining.min(CHUNK);
        let mut kbuf: Vec<u8> = alloc::vec![0u8; this];
        let not_copied = unsafe { uaccess::copy_from_user(kbuf.as_mut_ptr(), user, this) };
        let copied = this - not_copied;

        if copied == 0 {
            return if written > 0 {
                written as i64
            } else {
                -(EFAULT as i64)
            };
        }

        kbuf.truncate(copied);
        let result = vfs_write(&file, &kbuf);
        match result {
            Ok(n) => {
                written += n;
                if n < copied {
                    return written as i64;
                }
            }
            Err(errno) => {
                return if written > 0 {
                    written as i64
                } else {
                    -(errno as i64)
                };
            }
        }

        unsafe {
            user = user.add(copied);
        }
        remaining -= copied;

        if not_copied != 0 {
            // User buffer faulted mid-span: return what we managed to write.
            break;
        }
    }

    written as i64
}

/// `sys_read(2)` — userspace entry point.
///
/// Mirrors Linux `ksys_read`: fetch the file from the calling task's fdtable,
/// read through the VFS into a kernel bounce buffer, then copy the bytes to the
/// user pointer with the x86 uaccess fault contract.
pub unsafe fn sys_read(fd: i32, buf: *mut u8, count: usize) -> i64 {
    use crate::arch::x86::kernel::uaccess;
    use crate::include::uapi::errno::EFAULT;
    use crate::kernel::{files, sched};

    if count == 0 {
        return 0;
    }
    if buf.is_null() {
        return -(EFAULT as i64);
    }

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }

    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };
    let file = match ft.get(fd) {
        Ok(f) => f,
        Err(errno) => return -(errno as i64),
    };

    const CHUNK: usize = 4096;
    let mut done = 0usize;
    let mut user = buf;
    let mut remaining = count;

    while remaining > 0 {
        let this = remaining.min(CHUNK);
        let mut kbuf = alloc::vec![0u8; this];
        let n = match vfs_read(&file, &mut kbuf) {
            Ok(n) => n,
            Err(errno) => {
                return if done > 0 {
                    done as i64
                } else {
                    -(errno as i64)
                };
            }
        };
        if n == 0 {
            break;
        }

        let not_copied = unsafe { uaccess::copy_to_user(user, kbuf.as_ptr(), n) };
        let copied = n - not_copied;
        done += copied;
        unsafe {
            user = user.add(copied);
        }

        if copied < n {
            return if done > 0 {
                done as i64
            } else {
                -(EFAULT as i64)
            };
        }
        if n < this {
            break;
        }
        remaining -= copied;
    }

    done as i64
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::fs::dcache::{d_alloc, d_alloc_child};
    use crate::fs::file::{alloc_file, set_path_hint};
    use crate::fs::ops::{FileOps, NOOP_INODE_OPS};
    use crate::fs::ramfs::RAMFS_FILE_OPS;
    use crate::fs::types::{Inode, InodePrivate};
    use crate::security::integrity::ima;

    fn len_write(_file: &FileRef, buf: &[u8], _pos: &mut u64) -> Result<usize, i32> {
        Ok(buf.len())
    }

    static TEST_WRITE_OPS: FileOps = FileOps {
        name: "read-write-test",
        read: None,
        write: Some(len_write),
        llseek: None,
        fsync: None,
        poll: None,
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    #[test]
    fn vfs_read_measures_byte_backed_regular_files_for_ima() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        ima::reset_for_test();
        ima::init();
        ima::set_file_hook_measurements_for_test(true);

        let inode = Inode::new(
            100,
            InodeKind::Regular,
            0o444,
            &crate::fs::ramfs::RAMFS_FILE_INODE_OPS,
            &RAMFS_FILE_OPS,
            InodePrivate::StaticBytes(b"read measurement"),
        );
        inode.size.store(16, Ordering::Release);
        let root = d_alloc("/");
        let dentry = d_alloc_child(&root, "ima-read-probe");
        dentry.instantiate(inode);
        let file = alloc_file(dentry, O_RDONLY, 0o444, &RAMFS_FILE_OPS);
        set_path_hint(&file, String::from("/etc/ima-read-probe"));

        let mut out = [0u8; 32];
        assert_eq!(vfs_read(&file, &mut out).expect("read"), 16);

        let ascii = ima::ascii_runtime_measurements_sha1();
        assert!(ascii.contains("/etc/ima-read-probe"));

        ima::reset_for_test();
        crate::security::lsm_list::reset_for_test();
    }

    #[test]
    fn vfs_write_uses_open_file_mode_not_current_inode_mode_bits() {
        let inode = Inode::new(
            101,
            InodeKind::Socket,
            0o000,
            &NOOP_INODE_OPS,
            &TEST_WRITE_OPS,
            InodePrivate::Opaque(0),
        );
        let dentry = d_alloc("handoff-socket");
        dentry.instantiate(inode);
        let file = alloc_file(dentry, O_WRONLY, 0, &TEST_WRITE_OPS);

        assert_eq!(vfs_write(&file, b"handoff timestamp"), Ok(17));
    }
}
