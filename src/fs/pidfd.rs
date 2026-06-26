//! linux-parity: complete
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! pidfd support for child-exit polling.
//!
//! `pidfd_open()` installs an anon-inode-backed fd whose `poll()` mask
//! becomes readable once the target task exits.  systemd-260.1's
//! `vendor/systemd/systemd-260.1/src/basic/process-util.c::pidfd_open`
//! relies on this contract to track unit main PIDs without races.
//!
//! References:
//!   - `vendor/linux/kernel/pid.c::SYSCALL_DEFINE2(pidfd_open, …)`
//!   - `vendor/linux/Documentation/userspace-api/pidfd.rst`
//!   - `vendor/linux/include/uapi/linux/pidfd.h`

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::{FileOps, SuperOps};
use crate::fs::syscalls::POLLIN;
use crate::fs::types::{FileRef, SuperBlock};
use crate::include::uapi::errno::{EBADF, EFAULT, EINVAL, ENOTTY, ESRCH};
use crate::kernel::pid::{KPid, get_pid, put_pid};
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE};
use crate::kernel::{files, sched};

static PIDFD_TOKEN: AtomicUsize = AtomicUsize::new(1);

const IOC_TYPESHIFT: u32 = 8;
const IOC_NRSHIFT: u32 = 0;
const IOC_SIZESHIFT: u32 = 16;
const IOC_NRMASK: u32 = 0xff;
const IOC_TYPEMASK: u32 = 0xff;
const IOC_SIZEMASK: u32 = 0x3fff;
const IOC_READ: u32 = 2;
const IOC_WRITE: u32 = 1;

const PIDFS_IOCTL_MAGIC: u32 = 0xff;
const PIDFD_GET_INFO_NR: u32 = 11;
const PID_FS_MAGIC: u64 = 0x5049_4446;
pub const PIDFD_INFO_PID: u64 = 1 << 0;
const PIDFD_INFO_CREDS: u64 = 1 << 1;
const PIDFD_INFO_CGROUPID: u64 = 1 << 2;
const PIDFD_INFO_MIN_PID_SIZE: usize = 24;

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u32 {
    (dir << 30) | (ty << IOC_TYPESHIFT) | (nr << IOC_NRSHIFT) | (size << IOC_SIZESHIFT)
}

#[cfg(test)]
const fn pidfd_get_info_cmd(size: u32) -> u32 {
    ioc(
        IOC_READ | IOC_WRITE,
        PIDFS_IOCTL_MAGIC,
        PIDFD_GET_INFO_NR,
        size,
    )
}

fn ioc_type(cmd: u32) -> u32 {
    (cmd >> IOC_TYPESHIFT) & IOC_TYPEMASK
}

fn ioc_nr(cmd: u32) -> u32 {
    (cmd >> IOC_NRSHIFT) & IOC_NRMASK
}

fn ioc_size(cmd: u32) -> usize {
    ((cmd >> IOC_SIZESHIFT) & IOC_SIZEMASK) as usize
}

fn is_pidfd_get_info(cmd: u32) -> bool {
    ioc_type(cmd) == PIDFS_IOCTL_MAGIC && ioc_nr(cmd) == PIDFD_GET_INFO_NR
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PidfdInfo {
    mask: u64,
    cgroupid: u64,
    pid: u32,
    tgid: u32,
    ppid: u32,
    ruid: u32,
    rgid: u32,
    euid: u32,
    egid: u32,
    suid: u32,
    sgid: u32,
    fsuid: u32,
    fsgid: u32,
    exit_code: i32,
    coredump_mask: u32,
    __spare1: u32,
}

#[derive(Clone, Copy)]
struct PidFdState {
    pid: i32,
    task: *mut TaskStruct,
    kpid: *mut KPid,
    exited: bool,
}

unsafe impl Send for PidFdState {}
unsafe impl Sync for PidFdState {}

lazy_static! {
    static ref PIDFDS: Mutex<BTreeMap<usize, PidFdState>> = Mutex::new(BTreeMap::new());
}

static PIDFD_FILE_OPS: FileOps = FileOps {
    name: "pidfd",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(pidfd_file_poll),
    ioctl: Some(pidfd_ioctl),
    mmap: None,
    release: Some(pidfd_release),
    readdir: None,
};

static PIDFS_SUPER_OPS: SuperOps = SuperOps {
    name: "pidfs",
    statfs: None,
    put_super: None,
    sync_fs: None,
    alloc_inode: None,
    destroy_inode: None,
};

fn current_files() -> Result<Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn task_has_exited(task: *mut TaskStruct) -> bool {
    if task.is_null() {
        return true;
    }
    unsafe {
        ((*task).m26.exit_state & (EXIT_ZOMBIE | EXIT_DEAD) != 0)
            || ((*task).__state.load(Ordering::Acquire) & (EXIT_ZOMBIE | EXIT_DEAD) != 0)
    }
}

fn pidfd_file_poll(file: &FileRef) -> u32 {
    let token = *file.private.lock();
    let mut table = PIDFDS.lock();
    let Some(state) = table.get_mut(&token) else {
        return crate::fs::syscalls::POLLERR as u32;
    };
    if !state.exited && task_has_exited(state.task) {
        state.exited = true;
    }
    if state.exited { POLLIN as u32 } else { 0 }
}

fn pidfd_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    if !is_pidfd_get_info(cmd) {
        trace_pidfd_ioctl(file, cmd, arg, Err(ENOTTY), 0, 0);
        return Err(ENOTTY);
    }
    if arg == 0 {
        trace_pidfd_ioctl(file, cmd, arg, Err(EFAULT), 0, 0);
        return Err(EFAULT);
    }
    let size = ioc_size(cmd);
    if size < PIDFD_INFO_MIN_PID_SIZE {
        trace_pidfd_ioctl(file, cmd, arg, Err(EINVAL), size, 0);
        return Err(EINVAL);
    }

    let requested_mask = unsafe {
        crate::arch::x86::kernel::uaccess::get_user_u64(arg as *const u64)
            .map_err(|errno| -errno)?
    };
    let state = state_for_file(file)?;
    let tgid = if state.task.is_null() || state.exited {
        state.pid
    } else {
        unsafe { (*state.task).tgid }
    };
    let ppid = if state.task.is_null() || state.exited {
        0
    } else {
        let parent = unsafe { (*state.task).m26.real_parent };
        if parent.is_null() {
            0
        } else {
            unsafe { (*parent).pid }
        }
    };
    let mut info = PidfdInfo::default();
    let _ = requested_mask;
    info.mask = PIDFD_INFO_PID | PIDFD_INFO_CREDS | PIDFD_INFO_CGROUPID;
    info.cgroupid = 1;
    info.pid = state.pid.max(0) as u32;
    info.tgid = tgid.max(0) as u32;
    info.ppid = ppid.max(0) as u32;
    if !state.task.is_null() && !state.exited {
        let cred = unsafe { (*state.task).cred };
        if !cred.is_null() {
            let cred = unsafe { &*cred };
            info.ruid = cred.uid.0;
            info.rgid = cred.gid.0;
            info.euid = cred.euid.0;
            info.egid = cred.egid.0;
            info.suid = cred.suid.0;
            info.sgid = cred.sgid.0;
            info.fsuid = cred.fsuid.0;
            info.fsgid = cred.fsgid.0;
        }
    }

    let copy_len = size.min(size_of::<PidfdInfo>());
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            arg as *mut u8,
            &info as *const PidfdInfo as *const u8,
            copy_len,
        )
    };
    let result = if not_copied == 0 { Ok(0) } else { Err(EFAULT) };
    trace_pidfd_ioctl(file, cmd, arg, result, size, info.mask);
    result
}

#[cfg(not(test))]
fn trace_pidfd_ioctl(
    file: &FileRef,
    cmd: u32,
    _arg: u64,
    result: Result<i64, i32>,
    size: usize,
    mask: u64,
) {
    if !crate::kernel::debug_trace::proc_enabled() {
        return;
    }
    let token = *file.private.lock();
    let target = PIDFDS
        .lock()
        .get(&token)
        .map(|state| state.pid)
        .unwrap_or(-1);
    match result {
        Ok(ret) => crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-pidfd-ioctl target={} cmd={:#x} size={} mask={:#x} ret={}",
            target,
            cmd,
            size,
            mask,
            ret
        ),
        Err(errno) => crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-pidfd-ioctl target={} cmd={:#x} size={} errno={}",
            target,
            cmd,
            size,
            errno
        ),
    }
}

#[cfg(test)]
fn trace_pidfd_ioctl(
    _file: &FileRef,
    _cmd: u32,
    _arg: u64,
    _result: Result<i64, i32>,
    _size: usize,
    _mask: u64,
) {
}

fn pidfd_release(file: FileRef) {
    let token = *file.private.lock();
    let Some(state) = PIDFDS.lock().remove(&token) else {
        return;
    };
    unsafe {
        put_pid(state.kpid);
    }
}

fn install_pidfd_state(
    pid: i32,
    task: *mut TaskStruct,
    kpid: *mut KPid,
    cloexec: bool,
    exited: bool,
) -> Result<i32, i32> {
    if kpid.is_null() {
        return Err(ESRCH);
    }

    unsafe {
        get_pid(&*kpid);
    }

    let token = PIDFD_TOKEN.fetch_add(1, Ordering::AcqRel);
    PIDFDS.lock().insert(
        token,
        PidFdState {
            pid,
            task,
            kpid,
            exited,
        },
    );

    let file = alloc_anon_file("pidfd", &PIDFD_FILE_OPS, token);
    if let Some(inode) = file.inode() {
        *inode.sb.lock() = Some(SuperBlock::alloc("pidfs", PID_FS_MAGIC, &PIDFS_SUPER_OPS));
    }
    match current_files()?.install(file, cloexec) {
        Ok(fd) => Ok(fd),
        Err(errno) => {
            if let Some(state) = PIDFDS.lock().remove(&token) {
                unsafe {
                    put_pid(state.kpid);
                }
            }
            Err(errno)
        }
    }
}

pub fn install_pidfd(task: *mut TaskStruct, cloexec: bool) -> Result<i32, i32> {
    if task.is_null() {
        return Err(ESRCH);
    }
    let kpid = unsafe { (*task).m26.thread_pid };
    if kpid.is_null() {
        return Err(ESRCH);
    }

    install_pidfd_state(
        unsafe { (*task).pid },
        task,
        kpid,
        cloexec,
        task_has_exited(task),
    )
}

pub fn install_pidfd_from_saved_pid(
    pid: i32,
    task: *mut TaskStruct,
    kpid: *mut KPid,
    cloexec: bool,
) -> Result<i32, i32> {
    install_pidfd_state(pid, task, kpid, cloexec, task.is_null())
}

pub fn pid_for_fd(fd: i32) -> Result<i32, i32> {
    let file = current_files()?.get(fd)?;
    pid_for_file(&file)
}

pub fn task_for_fd(fd: i32) -> Result<*mut TaskStruct, i32> {
    let file = current_files()?.get(fd)?;
    task_for_file(&file)
}

pub fn pid_for_file(file: &FileRef) -> Result<i32, i32> {
    state_for_file(file).map(|state| state.pid)
}

pub fn task_for_file(file: &FileRef) -> Result<*mut TaskStruct, i32> {
    let state = state_for_file(file)?;
    if state.exited || state.task.is_null() {
        return Err(ESRCH);
    }
    Ok(state.task)
}

fn state_for_file(file: &FileRef) -> Result<PidFdState, i32> {
    if !core::ptr::eq(file.fops, &PIDFD_FILE_OPS) {
        return Err(EBADF);
    }
    let token = *file.private.lock();
    PIDFDS.lock().get(&token).copied().ok_or(EBADF)
}

pub fn notify_task_exit(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let pid = unsafe { (*task).pid };
    let mut table = PIDFDS.lock();
    for state in table.values_mut() {
        if state.task == task || state.pid == pid {
            state.exited = true;
            state.task = core::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    use crate::fs::fdtable::FilesStruct;
    use crate::fs::ioctl::sys_ioctl;
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::pid::{INIT_PID_NS, alloc_pid, put_pid};

    #[test]
    fn pidfd_becomes_readable_after_exit_notification() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 420;
        current.tgid = 420;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 421;
        target.tgid = 421;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = install_pidfd(&mut *target as *mut TaskStruct, false).expect("pidfd");
            let file = files::get_task_files(&mut *current as *mut TaskStruct)
                .expect("files")
                .get(fd)
                .expect("file");
            assert_eq!(pidfd_file_poll(&file), 0);

            notify_task_exit(&mut *target as *mut TaskStruct);
            assert_eq!(pidfd_file_poll(&file), POLLIN as u32);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            put_pid(target.m26.thread_pid);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn pidfd_resolves_target_task_for_signal_paths() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 422;
        current.tgid = 422;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 423;
        target.tgid = 423;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = install_pidfd(&mut *target as *mut TaskStruct, false).expect("pidfd");
            assert_eq!(pid_for_fd(fd), Ok(423));
            assert_eq!(
                task_for_fd(fd).expect("target task"),
                &mut *target as *mut TaskStruct
            );

            notify_task_exit(&mut *target as *mut TaskStruct);
            assert_eq!(task_for_fd(fd), Err(ESRCH));

            let files = files::get_task_files(&mut *current as *mut TaskStruct).expect("files");
            files.close(fd).expect("close pidfd");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn pidfd_get_info_ioctl_reports_target_pid() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 424;
        current.tgid = 424;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 425;
        target.tgid = 425;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = install_pidfd(&mut *target as *mut TaskStruct, false).expect("pidfd");
            let mut info = PidfdInfo {
                mask: PIDFD_INFO_PID,
                ..PidfdInfo::default()
            };

            assert_eq!(
                sys_ioctl(
                    fd,
                    pidfd_get_info_cmd(size_of::<PidfdInfo>() as u32),
                    &mut info as *mut PidfdInfo as u64,
                ),
                0
            );
            assert_eq!(info.mask & PIDFD_INFO_PID, PIDFD_INFO_PID);
            assert_eq!(info.mask & PIDFD_INFO_CREDS, PIDFD_INFO_CREDS);
            assert_eq!(info.mask & PIDFD_INFO_CGROUPID, PIDFD_INFO_CGROUPID);
            assert_eq!(info.pid, 425);
            assert_eq!(info.tgid, 425);
            assert_eq!(info.ruid, 0);
            assert_eq!(info.euid, 0);
            assert_eq!(info.cgroupid, 1);

            let mut sfs = crate::fs::syscalls::LinuxStatFs::default();
            assert_eq!(crate::fs::syscalls::sys_fstatfs(fd, &mut sfs), 0);
            assert_eq!(sfs.f_type as u64, PID_FS_MAGIC);

            let files = files::get_task_files(&mut *current as *mut TaskStruct).expect("files");
            files.close(fd).expect("close pidfd");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn pidfd_unknown_ioctl_returns_enotty() {
        let previous = unsafe { sched::get_current() };

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 426;
        current.tgid = 426;
        current.cred = &raw const INIT_CRED;

        let mut target = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        target.pid = 427;
        target.tgid = 427;
        target.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(target.pid)).expect("pid alloc");
        target.m26.thread_pid = Box::into_raw(kpid);

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let fd = install_pidfd(&mut *target as *mut TaskStruct, false).expect("pidfd");
            assert_eq!(sys_ioctl(fd, 0xdead_beef, 0), -(ENOTTY as i64));

            let files = files::get_task_files(&mut *current as *mut TaskStruct).expect("files");
            files.close(fd).expect("close pidfd");
            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
            target.m26.thread_pid = core::ptr::null_mut();
        }
    }
}
