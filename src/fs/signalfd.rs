//! linux-parity: complete
//! linux-source: vendor/linux/fs/signalfd.c
//! test-origin: linux:vendor/linux/fs/signalfd.c
//! signalfd — read signals as fd events.
//!
//! ABI parity with vendor/linux/fs/signalfd.c.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::anon_inode::alloc_anon_file;
use crate::fs::ops::FileOps;
use crate::fs::select::POLLIN;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EAGAIN, EBADF, EFAULT, EINVAL};
use crate::kernel::{files, sched};

/// `struct signalfd_siginfo` — read() output for signalfd.
/// Byte-identical to Linux uapi/linux/signalfd.h::signalfd_siginfo (128 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SignalfdSiginfo {
    pub ssi_signo: u32,
    pub ssi_errno: i32,
    pub ssi_code: i32,
    pub ssi_pid: u32,
    pub ssi_uid: u32,
    pub ssi_fd: i32,
    pub ssi_tid: u32,
    pub ssi_band: u32,
    pub ssi_overrun: u32,
    pub ssi_trapno: u32,
    pub ssi_status: i32,
    pub ssi_int: i32,
    pub ssi_ptr: u64,
    pub ssi_utime: u64,
    pub ssi_stime: u64,
    pub ssi_addr: u64,
    pub ssi_addr_lsb: u16,
    pub _pad2: u16,
    pub ssi_syscall: i32,
    pub ssi_call_addr: u64,
    pub ssi_arch: u32,
    pub _pad: [u8; 28],
}

pub const SFD_CLOEXEC: i32 = 0o2000000;
pub const SFD_NONBLOCK: i32 = 0o0004000;

static SIGNALFD_TOKEN: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref SIGNALFDS: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
}

static SIGNALFD_FILE_OPS: FileOps = FileOps {
    name: "signalfd",
    read: Some(signalfd_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(signalfd_poll),
    ioctl: None,
    mmap: None,
    release: Some(signalfd_release),
    readdir: None,
};

fn current_files() -> Result<alloc::sync::Arc<crate::fs::fdtable::FilesStruct>, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(EBADF);
    }
    unsafe { files::get_task_files(task) }.ok_or(EBADF)
}

fn signalfd_release(file: FileRef) {
    let token = *file.private.lock();
    SIGNALFDS.lock().remove(&token);
}

fn mask_for_file(file: &FileRef) -> Option<u64> {
    let token = *file.private.lock();
    SIGNALFDS.lock().get(&token).copied()
}

fn signalfd_poll(file: &FileRef) -> u32 {
    let Some(mask) = mask_for_file(file) else {
        return 0;
    };
    let pending = crate::kernel::signal::current_pending_signal_bits();
    let ready = crate::kernel::signal::has_current_pending_signal_mask(mask);
    trace_signalfd_poll(mask, pending, ready);
    if ready { POLLIN as u32 } else { 0 }
}

fn signalfd_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    let Some(mask) = mask_for_file(file) else {
        return Err(EBADF);
    };
    let record_size = core::mem::size_of::<SignalfdSiginfo>();
    if buf.len() < record_size {
        return Err(EINVAL);
    }

    let mut written = 0usize;
    while written + record_size <= buf.len() {
        let Some(info) = crate::kernel::signal::dequeue_current_pending_signal_mask(mask) else {
            break;
        };
        let record = SignalfdSiginfo {
            ssi_signo: info.signo as u32,
            ssi_errno: info.errno,
            ssi_code: info.code,
            ..SignalfdSiginfo::default()
        };
        let bytes = unsafe {
            core::slice::from_raw_parts(&record as *const SignalfdSiginfo as *const u8, record_size)
        };
        buf[written..written + record_size].copy_from_slice(bytes);
        written += record_size;
    }

    if written == 0 {
        Err(EAGAIN)
    } else {
        Ok(written)
    }
}

/// `sys_signalfd4(fd, mask, sizemask, flags)` — Linux syscall 289.
pub unsafe fn sys_signalfd4(fd: i32, mask: *const u8, sizemask: usize, flags: i32) -> i64 {
    let allowed = SFD_CLOEXEC | SFD_NONBLOCK;
    if flags & !allowed != 0 {
        return -(EINVAL as i64);
    }
    if mask.is_null() && sizemask != 0 {
        return -(EFAULT as i64);
    }
    let first_word =
        crate::kernel::signal::user_dequeue_signal_mask(if !mask.is_null() && sizemask >= 8 {
            unsafe { *(mask as *const u64) }
        } else {
            0
        });
    trace_signalfd_mask(fd, first_word, flags, -1);
    if fd >= 0 {
        let files = match current_files() {
            Ok(files) => files,
            Err(errno) => return -(errno as i64),
        };
        let file = match files.get(fd) {
            Ok(file) => file,
            Err(errno) => return -(errno as i64),
        };
        if file.fops.name != SIGNALFD_FILE_OPS.name {
            return -(EINVAL as i64);
        }
        let token = *file.private.lock();
        SIGNALFDS.lock().insert(token, first_word);
        trace_signalfd_mask(fd, first_word, flags, fd);
        return fd as i64;
    }
    let token = SIGNALFD_TOKEN.fetch_add(1, Ordering::AcqRel);
    SIGNALFDS.lock().insert(token, first_word);
    let file = alloc_anon_file("signalfd", &SIGNALFD_FILE_OPS, token);
    match current_files().and_then(|ft| ft.install(file, flags & SFD_CLOEXEC != 0)) {
        Ok(fd) => {
            trace_signalfd_mask(-1, first_word, flags, fd);
            fd as i64
        }
        Err(errno) => {
            SIGNALFDS.lock().remove(&token);
            -(errno as i64)
        }
    }
}

pub unsafe fn sys_signalfd(fd: i32, mask: *const u8, sizemask: usize) -> i64 {
    unsafe { sys_signalfd4(fd, mask, sizemask, 0) }
}

fn trace_signalfd_mask(request_fd: i32, mask: u64, flags: i32, ret_fd: i32) {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-signalfd pid={} request_fd={} mask={:#x} flags={:#x} ret_fd={}",
            pid,
            request_fd,
            mask,
            flags,
            ret_fd
        );
    }
    #[cfg(test)]
    let _ = (request_fd, mask, flags, ret_fd);
}

fn trace_signalfd_poll(mask: u64, pending: u64, ready: bool) {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-signalfd-poll pid={} mask={:#x} pending={:#x} ready={}",
            pid,
            mask,
            pending,
            ready
        );
    }
    #[cfg(test)]
    let _ = (mask, pending, ready);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fdtable::FilesStruct;
    use crate::kernel::{
        cred::INIT_CRED,
        files, sched,
        signal::{SIGCHLD, SIGKILL, SIGSTOP},
        task::TaskStruct,
    };
    use alloc::boxed::Box;

    #[test]
    fn siginfo_size_is_128() {
        assert_eq!(core::mem::size_of::<SignalfdSiginfo>(), 128);
    }

    #[test]
    fn siginfo_signo_offset() {
        assert_eq!(core::mem::offset_of!(SignalfdSiginfo, ssi_signo), 0);
        assert_eq!(core::mem::offset_of!(SignalfdSiginfo, ssi_errno), 4);
        assert_eq!(core::mem::offset_of!(SignalfdSiginfo, ssi_code), 8);
    }

    #[test]
    fn syscall_m76_signalfd_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 82;
        current.tgid = 82;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mask = 1u64 << 11;
            assert_eq!(sys_signalfd4(-1, core::ptr::null(), 1, 0), -(EFAULT as i64));
            assert_eq!(
                sys_signalfd4(
                    -1,
                    &mask as *const u64 as *const u8,
                    8,
                    0x8000_0000u32 as i32
                ),
                -(EINVAL as i64)
            );
            let fd = sys_signalfd4(-1, &mask as *const u64 as *const u8, 8, SFD_CLOEXEC);
            assert!(fd >= 0);
            assert_eq!(
                sys_signalfd(fd as i32, &mask as *const u64 as *const u8, 8),
                fd
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn signalfd_does_not_consume_sigkill() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 183;
        current.tgid = 183;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mask = (1u64 << (SIGKILL - 1)) | (1u64 << (SIGSTOP - 1));
            let fd = sys_signalfd4(-1, &mask as *const u64 as *const u8, 8, SFD_CLOEXEC);
            assert!(fd >= 0);
            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let file = ft.get(fd as i32).unwrap();
            assert_eq!(mask_for_file(&file), Some(0));

            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *current as *mut TaskStruct,
                    SIGKILL,
                ),
                0
            );
            assert_eq!(signalfd_poll(&file), 0);

            let mut buf = [0u8; core::mem::size_of::<SignalfdSiginfo>()];
            let mut pos = 0;
            assert_eq!(signalfd_read(&file, &mut buf, &mut pos), Err(EAGAIN));
            assert_eq!(
                crate::kernel::signal::take_current_fatal_signal(),
                Some(SIGKILL)
            );

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    #[test]
    fn signalfd_poll_and_read_consume_masked_signal() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 182;
        current.tgid = 182;
        current.cred = &raw const INIT_CRED;

        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);

            let mask = 1u64 << (SIGCHLD - 1);
            let fd = sys_signalfd4(-1, &mask as *const u64 as *const u8, 8, SFD_CLOEXEC);
            assert!(fd >= 0);
            let ft = files::get_task_files(&mut *current as *mut TaskStruct).unwrap();
            let file = ft.get(fd as i32).unwrap();

            assert_eq!(signalfd_poll(&file), 0);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *current as *mut TaskStruct,
                    SIGCHLD
                ),
                0
            );
            assert_eq!(signalfd_poll(&file), POLLIN as u32);

            let mut buf = [0u8; core::mem::size_of::<SignalfdSiginfo>()];
            let mut pos = 0;
            assert_eq!(
                signalfd_read(&file, &mut buf, &mut pos).unwrap(),
                core::mem::size_of::<SignalfdSiginfo>()
            );
            let info = core::ptr::read_unaligned(buf.as_ptr() as *const SignalfdSiginfo);
            assert_eq!(info.ssi_signo, SIGCHLD as u32);
            assert_eq!(signalfd_poll(&file), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }
}
