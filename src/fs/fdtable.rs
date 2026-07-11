//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! Per-task fdtable — `struct files_struct` (M39).
//!
//! Mirrors `vendor/linux/fs/file.c` and `vendor/linux/include/linux/fdtable.h`.
//! Replaces the M28 placeholder `enum FilesStruct {}` in `kernel/task.rs`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EBADF, EINVAL, EMFILE};
use crate::include::uapi::fcntl::FD_CLOEXEC;

use super::file::{fget, fput};
use super::types::FileRef;

pub const NR_OPEN_DEFAULT: usize = 64;
pub const NR_OPEN_MAX: usize = 1024 * 1024;

#[derive(Clone)]
struct Slot {
    file: Option<FileRef>,
    flags: u32, // FD_CLOEXEC bit
}

impl Slot {
    fn empty() -> Self {
        Self {
            file: None,
            flags: 0,
        }
    }
}

pub struct FilesStruct {
    pub count: AtomicUsize,
    table: Mutex<Vec<Slot>>,
    pub max_fds: AtomicUsize,
}

impl FilesStruct {
    pub fn new() -> Arc<Self> {
        let mut v = Vec::with_capacity(NR_OPEN_DEFAULT);
        for _ in 0..NR_OPEN_DEFAULT {
            v.push(Slot::empty());
        }
        Arc::new(Self {
            count: AtomicUsize::new(1),
            table: Mutex::new(v),
            max_fds: AtomicUsize::new(NR_OPEN_DEFAULT),
        })
    }

    fn ensure_len_locked(&self, table: &mut Vec<Slot>, needed: usize) -> Result<(), i32> {
        if needed > NR_OPEN_MAX {
            return Err(EMFILE);
        }
        if table.len() >= needed {
            return Ok(());
        }

        let mut new_len = table.len().max(NR_OPEN_DEFAULT);
        while new_len < needed {
            new_len = new_len.saturating_mul(2).min(NR_OPEN_MAX);
        }
        table.resize(new_len, Slot::empty());
        self.max_fds.store(new_len, Ordering::Release);
        Ok(())
    }

    pub fn install(&self, file: FileRef, cloexec: bool) -> Result<i32, i32> {
        self.install_at_or_above(file, 0, cloexec)
    }

    pub fn install_at_or_above(
        &self,
        file: FileRef,
        min_fd: usize,
        cloexec: bool,
    ) -> Result<i32, i32> {
        if min_fd >= NR_OPEN_MAX {
            return Err(EMFILE);
        }
        let mut t = self.table.lock();
        for (i, slot) in t.iter_mut().enumerate().skip(min_fd) {
            if slot.file.is_none() {
                slot.file = Some(file);
                slot.flags = if cloexec { FD_CLOEXEC } else { 0 };
                return Ok(i as i32);
            }
        }

        let fd = t.len().max(min_fd);
        self.ensure_len_locked(&mut t, fd + 1)?;
        t[fd].file = Some(file);
        t[fd].flags = if cloexec { FD_CLOEXEC } else { 0 };
        Ok(fd as i32)
    }

    pub fn dup_at_or_above(&self, oldfd: i32, min_fd: usize, cloexec: bool) -> Result<i32, i32> {
        if oldfd < 0 {
            return Err(EBADF);
        }
        if min_fd >= NR_OPEN_MAX {
            return Err(EMFILE);
        }

        let mut t = self.table.lock();
        let src = t
            .get(oldfd as usize)
            .and_then(|s| s.file.as_ref())
            .ok_or(EBADF)?;
        let new_file = fget(src);
        for (i, slot) in t.iter_mut().enumerate().skip(min_fd) {
            if slot.file.is_none() {
                slot.file = Some(new_file);
                slot.flags = if cloexec { FD_CLOEXEC } else { 0 };
                return Ok(i as i32);
            }
        }

        let fd = t.len().max(min_fd);
        if let Err(errno) = self.ensure_len_locked(&mut t, fd + 1) {
            fput(new_file);
            return Err(errno);
        }
        t[fd].file = Some(new_file);
        t[fd].flags = if cloexec { FD_CLOEXEC } else { 0 };
        Ok(fd as i32)
    }

    pub fn get(&self, fd: i32) -> Result<FileRef, i32> {
        if fd < 0 {
            return Err(EBADF);
        }
        let t = self.table.lock();
        t.get(fd as usize).and_then(|s| s.file.clone()).ok_or(EBADF)
    }

    pub fn close(&self, fd: i32) -> Result<(), i32> {
        if fd < 0 {
            return Err(EBADF);
        }
        let file = {
            let mut t = self.table.lock();
            let slot = t.get_mut(fd as usize).ok_or(EBADF)?;
            let file = slot.file.take().ok_or(EBADF)?;
            slot.flags = 0;
            file
        };
        super::eventpoll::notify_fd_closed(self, fd, &file);
        fput(file);
        Ok(())
    }

    pub fn dup2(&self, oldfd: i32, newfd: i32) -> Result<i32, i32> {
        if oldfd < 0 || newfd < 0 {
            return Err(EBADF);
        }
        let newfd = newfd as usize;
        if newfd >= NR_OPEN_MAX {
            return Err(EBADF);
        }
        let replaced = {
            let mut t = self.table.lock();
            if newfd as usize >= t.len() {
                self.ensure_len_locked(&mut t, newfd + 1)
                    .map_err(|_| EBADF)?;
            }
            let src = t
                .get(oldfd as usize)
                .and_then(|s| s.file.as_ref())
                .ok_or(EBADF)?;
            if oldfd as usize != newfd {
                let src = fget(src);
                let replaced = t[newfd].file.take();
                t[newfd] = Slot {
                    file: Some(src),
                    flags: 0,
                };
                replaced
            } else {
                None
            }
        };
        if let Some(file) = replaced {
            super::eventpoll::notify_fd_closed(self, newfd as i32, &file);
            fput(file);
        }
        Ok(newfd as i32)
    }

    pub fn close_range(&self, first: usize, last: usize) -> Result<(), i32> {
        if last < first {
            return Err(EINVAL);
        }
        let mut to_put = Vec::new();
        {
            let mut t = self.table.lock();
            if first >= t.len() {
                return Ok(());
            }
            let hi = last.min(t.len().saturating_sub(1));
            for i in first..=hi {
                if let Some(file) = t[i].file.take() {
                    to_put.push((i as i32, file));
                }
                t[i] = Slot::empty();
            }
        }
        for (fd, file) in to_put {
            super::eventpoll::notify_fd_closed(self, fd, &file);
            fput(file);
        }
        Ok(())
    }

    /// Close all fds whose FD_CLOEXEC bit is set, mirroring Linux's
    /// `do_close_on_exec()`.  Called from the `execve` path after the new
    /// `mm_struct` is installed but before handoff to userspace, so the
    /// new program sees only the fds the parent chose to leave un-marked.
    ///
    /// Ref: vendor/linux/fs/file.c::do_close_on_exec
    pub fn close_on_exec(&self) {
        let mut to_put = Vec::new();
        {
            let mut t = self.table.lock();
            for (fd, slot) in t.iter_mut().enumerate() {
                if slot.flags & FD_CLOEXEC != 0 {
                    if let Some(file) = slot.file.take() {
                        to_put.push((fd as i32, file));
                    }
                    slot.flags = 0;
                }
            }
        }
        for (fd, file) in to_put {
            super::eventpoll::notify_fd_closed(self, fd, &file);
            fput(file);
        }
    }

    pub fn set_cloexec_range(&self, first: usize, last: usize) -> Result<(), i32> {
        if last < first {
            return Err(EINVAL);
        }
        let mut t = self.table.lock();
        if first >= t.len() {
            return Ok(());
        }
        let hi = last.min(t.len().saturating_sub(1));
        for i in first..=hi {
            if t[i].file.is_some() {
                t[i].flags |= FD_CLOEXEC;
            }
        }
        Ok(())
    }

    pub fn get_fd_flags(&self, fd: i32) -> Result<u32, i32> {
        if fd < 0 {
            return Err(EBADF);
        }
        let t = self.table.lock();
        let slot = t.get(fd as usize).ok_or(EBADF)?;
        if slot.file.is_none() {
            return Err(EBADF);
        }
        Ok(slot.flags)
    }

    pub fn set_fd_flags(&self, fd: i32, flags: u32) -> Result<(), i32> {
        if fd < 0 {
            return Err(EBADF);
        }
        let mut t = self.table.lock();
        let slot = t.get_mut(fd as usize).ok_or(EBADF)?;
        if slot.file.is_none() {
            return Err(EBADF);
        }
        slot.flags = flags & FD_CLOEXEC;
        Ok(())
    }

    pub fn open_count(&self) -> usize {
        self.table
            .lock()
            .iter()
            .filter(|s| s.file.is_some())
            .count()
    }

    pub fn open_fds(&self) -> Vec<i32> {
        self.table
            .lock()
            .iter()
            .enumerate()
            .filter_map(|(fd, slot)| slot.file.as_ref().map(|_| fd as i32))
            .collect()
    }

    pub fn open_file_refs(&self) -> Vec<FileRef> {
        self.table
            .lock()
            .iter()
            .filter_map(|slot| slot.file.clone())
            .collect()
    }
}

/// `close(2)` â€” userspace entry point.
///
/// Mirrors Linux `sys_close` / `__close_fd` at a very small scale: look up the
/// calling task's fdtable and clear the slot.
///
/// Source of truth: `vendor/linux/fs/open.c`, `vendor/linux/fs/file.c`,
/// `vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl` (nr 3).
impl Drop for FilesStruct {
    fn drop(&mut self) {
        let mut to_put = Vec::new();
        {
            let mut table = self.table.lock();
            for slot in table.iter_mut() {
                if let Some(file) = slot.file.take() {
                    to_put.push(file);
                }
                slot.flags = 0;
            }
        }
        for file in to_put {
            fput(file);
        }
    }
}

pub unsafe fn sys_close(fd: i32) -> i64 {
    use crate::include::uapi::errno::EBADF;
    use crate::kernel::{files, sched};

    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return -(EBADF as i64);
    }
    let Some(ft) = (unsafe { files::get_task_files(task) }) else {
        return -(EBADF as i64);
    };
    match ft.close(fd) {
        Ok(()) => 0,
        Err(errno) => -(errno as i64),
    }
}

/// Clone the fdtable for a forking child.  When `share` is true returns
/// the same Arc (CLONE_FILES); otherwise produces an independent copy.
pub fn dup_fd(parent: &Arc<FilesStruct>, share: bool) -> Arc<FilesStruct> {
    if share {
        parent.count.fetch_add(1, Ordering::AcqRel);
        return parent.clone();
    }
    let new = FilesStruct::new();
    let src = parent.table.lock().clone();
    let mut dst = new.table.lock();
    if dst.len() < src.len() {
        dst.resize(src.len(), Slot::empty());
    }
    for (i, s) in src.iter().enumerate() {
        if let Some(f) = &s.file {
            dst[i] = Slot {
                file: Some(fget(f)),
                flags: s.flags,
            };
        }
    }
    drop(dst);
    new.max_fds.store(src.len(), Ordering::Release);
    new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc;
    use crate::fs::file::alloc_file;
    use crate::fs::ops::{FileOps, NOOP_FILE_OPS};

    static DROP_RELEASES: AtomicUsize = AtomicUsize::new(0);

    fn count_release(_file: FileRef) {
        DROP_RELEASES.fetch_add(1, Ordering::AcqRel);
    }

    static COUNT_RELEASE_OPS: FileOps = FileOps {
        name: "fdtable-drop-release",
        read: None,
        write: None,
        llseek: None,
        fsync: None,
        poll: None,
        ioctl: None,
        mmap: None,
        release: Some(count_release),
        readdir: None,
    };

    #[test]
    fn install_get_close_round_trip() {
        let ft = FilesStruct::new();
        let d = d_alloc("x");
        let f = alloc_file(d, 0, 0, &NOOP_FILE_OPS);
        let fd = ft.install(f.clone(), false).unwrap();
        assert_eq!(fd, 0);
        let f2 = ft.get(fd).unwrap();
        assert!(Arc::ptr_eq(&f, &f2));
        ft.close(fd).unwrap();
        assert!(ft.get(fd).is_err());
    }

    #[test]
    fn dup2_overwrites_target() {
        let ft = FilesStruct::new();
        let d = d_alloc("x");
        let f = alloc_file(d, 0, 0, &NOOP_FILE_OPS);
        let fd0 = ft.install(f, false).unwrap();
        ft.dup2(fd0, 5).unwrap();
        assert!(ft.get(5).is_ok());
    }

    #[test]
    fn dup2_takes_counted_file_reference() {
        let ft = FilesStruct::new();
        let file = alloc_file(d_alloc("dup-count"), 0, 0, &NOOP_FILE_OPS);
        let fd0 = ft.install(file.clone(), false).unwrap();

        ft.dup2(fd0, 5).unwrap();
        assert_eq!(file.f_count.load(Ordering::Acquire), 2);

        ft.close(fd0).unwrap();
        assert_eq!(file.f_count.load(Ordering::Acquire), 1);
        ft.close(5).unwrap();
        assert_eq!(file.f_count.load(Ordering::Acquire), 0);
    }

    #[test]
    fn dup_fd_takes_counted_file_references_for_child_table() {
        let parent = FilesStruct::new();
        let file = alloc_file(d_alloc("fork-count"), 0, 0, &NOOP_FILE_OPS);
        let fd = parent.install(file.clone(), false).unwrap();

        let child = dup_fd(&parent, false);
        assert_eq!(file.f_count.load(Ordering::Acquire), 2);

        child.close(fd).unwrap();
        assert_eq!(file.f_count.load(Ordering::Acquire), 1);
        parent.close(fd).unwrap();
        assert_eq!(file.f_count.load(Ordering::Acquire), 0);
    }

    #[test]
    fn fd_flags_require_live_file() {
        let ft = FilesStruct::new();
        assert_eq!(ft.get_fd_flags(7), Err(EBADF));
        assert_eq!(ft.set_fd_flags(7, FD_CLOEXEC), Err(EBADF));
    }

    #[test]
    fn dropping_fdtable_releases_open_files() {
        DROP_RELEASES.store(0, Ordering::Release);
        {
            let ft = FilesStruct::new();
            let file = alloc_file(d_alloc("drop-release"), 0, 0, &COUNT_RELEASE_OPS);
            ft.install(file, false).unwrap();
        }
        assert_eq!(DROP_RELEASES.load(Ordering::Acquire), 1);
    }

    #[test]
    fn close_on_exec_drops_cloexec_fds_keeps_others() {
        let ft = FilesStruct::new();
        let d_keep = d_alloc("keep");
        let d_drop = d_alloc("drop");
        let f_keep = alloc_file(d_keep, 0, 0, &NOOP_FILE_OPS);
        let f_drop = alloc_file(d_drop, 0, 0, &NOOP_FILE_OPS);

        // fd 0: no CLOEXEC (e.g. stdin survives exec)
        let keep = ft.install(f_keep, false).unwrap();
        // fd 1: CLOEXEC (e.g. an internal socket systemd opened)
        let drop_fd = ft.install(f_drop, true).unwrap();

        assert_eq!(ft.get_fd_flags(drop_fd).unwrap(), FD_CLOEXEC);

        ft.close_on_exec();

        assert!(ft.get(keep).is_ok(), "non-CLOEXEC fd must survive exec");
        assert!(
            matches!(ft.get(drop_fd), Err(e) if e == EBADF),
            "CLOEXEC fd must be gone"
        );
        assert_eq!(ft.get_fd_flags(drop_fd), Err(EBADF));
    }

    #[test]
    fn dup_minimum_fd_and_close_range_cloexec_match_linux_shape() {
        let ft = FilesStruct::new();
        let d = d_alloc("x");
        let f = alloc_file(d, 0, 0, &NOOP_FILE_OPS);
        let fd0 = ft.install(f.clone(), false).unwrap();
        assert_eq!(fd0, 0);

        let fd5 = ft.install_at_or_above(f, 5, true).unwrap();
        assert_eq!(fd5, 5);
        assert_eq!(ft.get_fd_flags(fd5).unwrap(), FD_CLOEXEC);

        ft.set_fd_flags(fd5, 0).unwrap();
        ft.set_cloexec_range(5, 5).unwrap();
        assert_eq!(ft.get_fd_flags(fd5).unwrap(), FD_CLOEXEC);
        assert!(ft.get(fd5).is_ok());
    }
}
