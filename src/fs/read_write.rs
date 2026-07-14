//! linux-parity: partial
//! linux-source: vendor/linux/fs/read_write.c
//! test-origin: linux:vendor/linux/fs/read_write.c
//! `vfs_read`, `vfs_write`, `vfs_lseek`, `vfs_fsync`.
//!
//! Mirrors `vendor/linux/fs/read_write.c`.  Each routine validates the file
//! mode, advances `f_pos`, and dispatches through `file.fops`.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use crate::include::uapi::errno::{EBADF, EINVAL, EISDIR, ENOSYS, ENXIO};
use crate::include::uapi::fcntl::{O_ACCMODE, O_APPEND, O_PATH, O_RDONLY, O_RDWR, O_WRONLY};
use crate::kernel::module::{export_symbol, find_symbol};

use super::file::note_file_access_for_integrity;
use super::permission::check_file_write_mount;
use super::types::{FileRef, InodeKind};

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;
const SEEK_DATA: i32 = 3;
const SEEK_HOLE: i32 = 4;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("default_llseek", linux_default_llseek as usize, false);
    export_symbol_once("noop_llseek", linux_noop_llseek as usize, false);
    export_symbol_once(
        "generic_file_llseek",
        linux_generic_file_llseek as usize,
        false,
    );
    export_symbol_once(
        "generic_file_llseek_size",
        linux_generic_file_llseek_size as usize,
        false,
    );
    export_symbol_once("fixed_size_llseek", linux_fixed_size_llseek as usize, false);
    export_symbol_once(
        "no_seek_end_llseek",
        linux_no_seek_end_llseek as usize,
        false,
    );
    export_symbol_once(
        "no_seek_end_llseek_size",
        linux_no_seek_end_llseek_size as usize,
        false,
    );
    export_symbol_once("stream_open", linux_stream_open as usize, false);
    export_symbol_once("nonseekable_open", linux_stream_open as usize, false);
    export_symbol_once("kernel_write", linux_kernel_write as usize, false);
}

/// `default_llseek` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_default_llseek(_file: *mut c_void, offset: i64, whence: i32) -> i64 {
    match whence {
        SEEK_SET | SEEK_CUR | SEEK_END if offset >= 0 => offset,
        _ => -(EINVAL as i64),
    }
}

/// `noop_llseek` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_noop_llseek(_file: *mut c_void, offset: i64, _whence: i32) -> i64 {
    offset
}

fn linux_llseek_setpos(new_offset: i64, maxsize: i64) -> i64 {
    if new_offset < 0 {
        return -(EINVAL as i64);
    }
    let maxsize = if maxsize < 0 { i64::MAX } else { maxsize };
    if new_offset > maxsize {
        return -(EINVAL as i64);
    }
    new_offset
}

fn linux_llseek_compute(offset: i64, whence: i32, maxsize: i64, eof: i64) -> i64 {
    let eof = eof.max(0);
    match whence {
        SEEK_SET | SEEK_CUR => linux_llseek_setpos(offset, maxsize),
        SEEK_END => match eof.checked_add(offset) {
            Some(new_offset) => linux_llseek_setpos(new_offset, maxsize),
            None => -(EINVAL as i64),
        },
        SEEK_DATA => {
            if offset < 0 || offset >= eof {
                -(ENXIO as i64)
            } else {
                linux_llseek_setpos(offset, maxsize)
            }
        }
        SEEK_HOLE => {
            if offset < 0 || offset >= eof {
                -(ENXIO as i64)
            } else {
                linux_llseek_setpos(eof, maxsize)
            }
        }
        _ => -(EINVAL as i64),
    }
}

/// `generic_file_llseek_size` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_generic_file_llseek_size(
    _file: *mut c_void,
    offset: i64,
    whence: i32,
    maxsize: i64,
    eof: i64,
) -> i64 {
    linux_llseek_compute(offset, whence, maxsize, eof)
}

/// `generic_file_llseek` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_generic_file_llseek(
    file: *mut c_void,
    offset: i64,
    whence: i32,
) -> i64 {
    unsafe { linux_generic_file_llseek_size(file, offset, whence, i64::MAX, 0) }
}

/// `fixed_size_llseek` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_fixed_size_llseek(
    file: *mut c_void,
    offset: i64,
    whence: i32,
    size: i64,
) -> i64 {
    match whence {
        SEEK_SET | SEEK_CUR | SEEK_END => unsafe {
            linux_generic_file_llseek_size(file, offset, whence, size, size)
        },
        _ => -(EINVAL as i64),
    }
}

/// `no_seek_end_llseek` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_no_seek_end_llseek(
    file: *mut c_void,
    offset: i64,
    whence: i32,
) -> i64 {
    match whence {
        SEEK_SET | SEEK_CUR => unsafe {
            linux_generic_file_llseek_size(file, offset, whence, i64::MAX, 0)
        },
        _ => -(EINVAL as i64),
    }
}

/// `no_seek_end_llseek_size` - `vendor/linux/fs/read_write.c`.
pub unsafe extern "C" fn linux_no_seek_end_llseek_size(
    file: *mut c_void,
    offset: i64,
    whence: i32,
    size: i64,
) -> i64 {
    match whence {
        SEEK_SET | SEEK_CUR => unsafe {
            linux_generic_file_llseek_size(file, offset, whence, size, 0)
        },
        _ => -(EINVAL as i64),
    }
}

/// `stream_open` - `vendor/linux/fs/open.c`.
pub unsafe extern "C" fn linux_stream_open(_inode: *mut c_void, _file: *mut c_void) -> i32 {
    0
}

/// `kernel_write` - `vendor/linux/fs/read_write.c:651`.
///
/// Vendor modules pass Linux-layout `struct file *` objects. Until Lupos owns
/// those objects as native `FileRef`s, non-empty writes fail closed instead of
/// pretending bytes reached a backing file.
pub unsafe extern "C" fn linux_kernel_write(
    _file: *mut c_void,
    _buf: *const c_void,
    count: usize,
    _pos: *mut i64,
) -> isize {
    if count == 0 { 0 } else { -(EBADF as isize) }
}

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
    let start = *pos;
    let result = write(file, buf, &mut *pos);
    if let Ok(written) = result {
        unsafe {
            crate::mm::filemap::filemap_update_cached_range(
                inode.mapping(),
                start,
                &buf[..written.min(buf.len())],
            );
        }
    }
    result
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

    #[test]
    fn registers_llseek_module_exports() {
        register_module_exports();

        for name in [
            "default_llseek",
            "noop_llseek",
            "generic_file_llseek",
            "generic_file_llseek_size",
            "fixed_size_llseek",
            "no_seek_end_llseek",
            "no_seek_end_llseek_size",
            "kernel_write",
        ] {
            assert!(
                crate::kernel::module::find_symbol(name).is_some(),
                "missing export {name}"
            );
        }

        assert_eq!(
            unsafe {
                linux_kernel_write(
                    core::ptr::null_mut(),
                    core::ptr::null(),
                    0,
                    core::ptr::null_mut(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                linux_kernel_write(
                    core::ptr::null_mut(),
                    b"x".as_ptr().cast(),
                    1,
                    core::ptr::null_mut(),
                )
            },
            -(EBADF as isize)
        );
    }

    #[test]
    fn generic_file_llseek_size_handles_core_whence_values() {
        unsafe {
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), 12, SEEK_SET, 64, 32),
                12
            );
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), -4, SEEK_END, 64, 32),
                28
            );
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), 8, SEEK_DATA, 64, 32),
                8
            );
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), 8, SEEK_HOLE, 64, 32),
                32
            );
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), 32, SEEK_DATA, 64, 32),
                -(ENXIO as i64)
            );
            assert_eq!(
                linux_generic_file_llseek_size(core::ptr::null_mut(), 65, SEEK_SET, 64, 32),
                -(EINVAL as i64)
            );
        }
    }

    #[test]
    fn fixed_and_no_seek_end_llseek_match_linux_acceptance() {
        unsafe {
            assert_eq!(
                linux_fixed_size_llseek(core::ptr::null_mut(), -2, SEEK_END, 8),
                6
            );
            assert_eq!(
                linux_fixed_size_llseek(core::ptr::null_mut(), 0, SEEK_DATA, 8),
                -(EINVAL as i64)
            );
            assert_eq!(
                linux_no_seek_end_llseek(core::ptr::null_mut(), 4, SEEK_SET),
                4
            );
            assert_eq!(
                linux_no_seek_end_llseek(core::ptr::null_mut(), 0, SEEK_END),
                -(EINVAL as i64)
            );
        }
    }
}
