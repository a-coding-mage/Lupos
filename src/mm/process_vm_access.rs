//! linux-parity: complete
//! linux-source: vendor/linux/mm/process_vm_access.c
//! test-origin: linux:vendor/linux/mm/process_vm_access.c
//! `process_vm_readv(2)` / `process_vm_writev(2)` memory mover.
//!
//! The scheduler owns PID lookup; this module owns Linux iovec validation and
//! byte movement once the target mm is known.
//!
//! References:
//! - `vendor/linux/mm/process_vm_access.c`

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::{EFAULT, EINVAL};

pub const UIO_MAXIOV: usize = 1024;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProcessIoVec {
    pub base: *mut u8,
    pub len: usize,
}

const PROCESS_VM_COPY_CHUNK: usize = 4096;

fn copy_iovecs_from_user(
    user_iov: *const ProcessIoVec,
    iovcnt: usize,
) -> Result<Vec<ProcessIoVec>, i64> {
    let bytes = iovcnt
        .checked_mul(core::mem::size_of::<ProcessIoVec>())
        .ok_or(-(EINVAL as i64))?;
    let mut iovecs = vec![ProcessIoVec::default(); iovcnt];
    if bytes == 0 {
        return Ok(iovecs);
    }
    let not_copied = unsafe {
        uaccess::copy_from_user(iovecs.as_mut_ptr() as *mut u8, user_iov as *const u8, bytes)
    };
    if not_copied == 0 {
        Ok(iovecs)
    } else {
        Err(-(EFAULT as i64))
    }
}

fn user_ptr_at(base: *mut u8, offset: usize) -> Option<*mut u8> {
    (base as usize)
        .checked_add(offset)
        .map(|addr| addr as *mut u8)
}

fn finish_fault(copied: usize) -> i64 {
    if copied == 0 {
        -(EFAULT as i64)
    } else {
        copied as i64
    }
}

pub unsafe fn process_vm_rw_same_mm(
    local: *const ProcessIoVec,
    liovcnt: usize,
    remote: *const ProcessIoVec,
    riovcnt: usize,
    flags: u64,
    write: bool,
) -> i64 {
    if flags != 0 || liovcnt > UIO_MAXIOV || riovcnt > UIO_MAXIOV {
        return -(EINVAL as i64);
    }
    if liovcnt == 0 || riovcnt == 0 {
        return 0;
    }
    if local.is_null() || remote.is_null() {
        return -(EFAULT as i64);
    }

    let local_iovecs = match copy_iovecs_from_user(local, liovcnt) {
        Ok(iovecs) => iovecs,
        Err(err) => return err,
    };
    let remote_iovecs = match copy_iovecs_from_user(remote, riovcnt) {
        Ok(iovecs) => iovecs,
        Err(err) => return err,
    };

    let mut lidx = 0usize;
    let mut ridx = 0usize;
    let mut loff = 0usize;
    let mut roff = 0usize;
    let mut copied = 0usize;
    let mut scratch = [0u8; PROCESS_VM_COPY_CHUNK];

    while lidx < liovcnt && ridx < riovcnt {
        let l = local_iovecs[lidx];
        let r = remote_iovecs[ridx];
        if l.len == 0 {
            lidx += 1;
            loff = 0;
            continue;
        }
        if r.len == 0 {
            ridx += 1;
            roff = 0;
            continue;
        }
        if l.base.is_null() || r.base.is_null() {
            return finish_fault(copied);
        }

        let n = (l.len - loff).min(r.len - roff).min(PROCESS_VM_COPY_CHUNK);
        let local_ptr = match user_ptr_at(l.base, loff) {
            Some(ptr) => ptr,
            None => return finish_fault(copied),
        };
        let remote_ptr = match user_ptr_at(r.base, roff) {
            Some(ptr) => ptr,
            None => return finish_fault(copied),
        };

        let (src, dst) = if write {
            (local_ptr as *const u8, remote_ptr)
        } else {
            (remote_ptr as *const u8, local_ptr)
        };

        let not_read = unsafe { uaccess::copy_from_user(scratch.as_mut_ptr(), src, n) };
        if not_read != 0 {
            return finish_fault(copied);
        }
        let not_written = unsafe { uaccess::copy_to_user(dst, scratch.as_ptr(), n) };
        if not_written != 0 {
            return finish_fault(copied);
        }

        copied += n;
        loff += n;
        roff += n;
        if loff == l.len {
            lidx += 1;
            loff = 0;
        }
        if roff == r.len {
            ridx += 1;
            roff = 0;
        }
    }
    copied as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readv_moves_remote_to_local() {
        let remote = [9u8, 8, 7, 6];
        let mut local = [0u8; 4];
        let liov = [ProcessIoVec {
            base: local.as_mut_ptr(),
            len: local.len(),
        }];
        let riov = [ProcessIoVec {
            base: remote.as_ptr() as *mut u8,
            len: remote.len(),
        }];
        assert_eq!(
            unsafe { process_vm_rw_same_mm(liov.as_ptr(), 1, riov.as_ptr(), 1, 0, false) },
            4
        );
        assert_eq!(local, remote);
    }

    #[test]
    fn writev_moves_local_to_remote() {
        let local = [1u8, 2, 3];
        let mut remote = [0u8; 3];
        let liov = [ProcessIoVec {
            base: local.as_ptr() as *mut u8,
            len: local.len(),
        }];
        let riov = [ProcessIoVec {
            base: remote.as_mut_ptr(),
            len: remote.len(),
        }];
        assert_eq!(
            unsafe { process_vm_rw_same_mm(liov.as_ptr(), 1, riov.as_ptr(), 1, 0, true) },
            3
        );
        assert_eq!(remote, local);
    }

    #[test]
    fn kernel_range_iovec_pointer_returns_efault() {
        let mut local = [0u8; 4];
        let liov = [ProcessIoVec {
            base: local.as_mut_ptr(),
            len: local.len(),
        }];
        let kernel_range_iov =
            crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as *const ProcessIoVec;

        assert_eq!(
            unsafe { process_vm_rw_same_mm(liov.as_ptr(), 1, kernel_range_iov, 1, 0, false) },
            -(EFAULT as i64)
        );
    }

    #[test]
    fn kernel_range_iovec_base_returns_efault_without_copying() {
        let mut local = [0u8; 4];
        let liov = [ProcessIoVec {
            base: local.as_mut_ptr(),
            len: local.len(),
        }];
        let riov = [ProcessIoVec {
            base: crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX as *mut u8,
            len: local.len(),
        }];

        assert_eq!(
            unsafe { process_vm_rw_same_mm(liov.as_ptr(), 1, riov.as_ptr(), 1, 0, false) },
            -(EFAULT as i64)
        );
        assert_eq!(local, [0u8; 4]);
    }
}
