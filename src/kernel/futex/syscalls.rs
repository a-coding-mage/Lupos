//! linux-parity: partial
//! linux-source: vendor/linux/kernel/futex/syscalls.c
//! test-origin: linux:vendor/linux/kernel/futex/syscalls.c
//! Futex syscall dispatch.
//!
//! Mirrors `vendor/linux/kernel/futex/syscalls.c`. Operation-specific logic
//! lives in the futex core, PI, requeue, and robust modules; this file owns the
//! Linux multiplexed syscall selector.

extern crate alloc;

use alloc::vec;

use super::{
    EFAULT, ENOSYS, FUTEX_32, FUTEX_BITSET_MATCH_ANY, FUTEX_CLOCK_REALTIME, FUTEX_CMD_MASK,
    FUTEX_CMP_REQUEUE, FUTEX_CMP_REQUEUE_PI, FUTEX_LOCK_PI, FUTEX_LOCK_PI2, FUTEX_PRIVATE_FLAG,
    FUTEX_REQUEUE, FUTEX_TRYLOCK_PI, FUTEX_UNLOCK_PI, FUTEX_WAIT, FUTEX_WAIT_BITSET,
    FUTEX_WAIT_REQUEUE_PI, FUTEX_WAITV_MAX, FUTEX_WAKE, FUTEX_WAKE_BITSET, FUTEX_WAKE_OP,
    FUTEX2_PRIVATE, FutexDeadline, FutexWaitv, futex_cmp_requeue_pi, futex_lock_pi, futex_requeue,
    futex_trylock_pi, futex_unlock_pi, futex_wait, futex_wait_deadline, futex_wait_requeue_pi,
    futex_waitv_deadline, futex_wake, futex_wake_op,
};

use crate::kernel::time::{CLOCK_MONOTONIC, CLOCK_REALTIME};

/// Linux `sys_futex(uaddr, op, val, utime, uaddr2, val3)`.
///
/// Returns >= 0 on success, negative `errno` on failure.
pub unsafe fn sys_futex(
    uaddr: u64,
    op: u32,
    val: u32,
    timeout_ns: u64,
    uaddr2: u64,
    val3: u32,
) -> i64 {
    let cmd = op & FUTEX_CMD_MASK;
    let private = op & FUTEX_PRIVATE_FLAG != 0;
    if op & FUTEX_CLOCK_REALTIME != 0
        && !matches!(
            cmd,
            FUTEX_WAIT_BITSET | FUTEX_WAIT_REQUEUE_PI | FUTEX_LOCK_PI2
        )
    {
        return -ENOSYS as i64;
    }
    match cmd {
        FUTEX_WAIT => unsafe {
            futex_wait(uaddr, val, FUTEX_BITSET_MATCH_ANY, timeout_ns, private)
        },
        FUTEX_WAKE => unsafe { futex_wake(uaddr, val as i32, FUTEX_BITSET_MATCH_ANY, private) },
        FUTEX_WAIT_BITSET => unsafe { futex_wait(uaddr, val, val3, timeout_ns, private) },
        FUTEX_WAKE_BITSET => unsafe { futex_wake(uaddr, val as i32, val3, private) },
        FUTEX_REQUEUE | FUTEX_CMP_REQUEUE => unsafe {
            futex_requeue(
                uaddr,
                uaddr2,
                val as i32,
                timeout_ns as i32,
                val3,
                cmd == FUTEX_CMP_REQUEUE,
                private,
            )
        },
        FUTEX_WAKE_OP => unsafe {
            futex_wake_op(uaddr, uaddr2, val as i32, timeout_ns as i32, val3, private)
        },
        FUTEX_LOCK_PI | FUTEX_LOCK_PI2 => unsafe { futex_lock_pi(uaddr, timeout_ns, private) },
        FUTEX_UNLOCK_PI => unsafe { futex_unlock_pi(uaddr, private) },
        FUTEX_TRYLOCK_PI => unsafe { futex_trylock_pi(uaddr, private) },
        FUTEX_WAIT_REQUEUE_PI => unsafe {
            futex_wait_requeue_pi(uaddr, val, uaddr2, timeout_ns, private)
        },
        FUTEX_CMP_REQUEUE_PI => unsafe {
            futex_cmp_requeue_pi(uaddr, uaddr2, val as i32, timeout_ns as i32, val3, private)
        },
        _ => -ENOSYS as i64,
    }
}

pub unsafe fn sys_futex_waitv(
    waiters: u64,
    nr_waiters: usize,
    flags: u32,
    timeout_ns: u64,
    clockid: i32,
) -> i64 {
    let deadline = (timeout_ns != 0).then(|| FutexDeadline::relative_monotonic(timeout_ns));
    unsafe { sys_futex_waitv_deadline(waiters, nr_waiters, flags, deadline, clockid) }
}

pub unsafe fn sys_futex_waitv_deadline(
    waiters: u64,
    nr_waiters: usize,
    flags: u32,
    deadline: Option<FutexDeadline>,
    clockid: i32,
) -> i64 {
    if flags != 0 || waiters == 0 || nr_waiters == 0 || nr_waiters > FUTEX_WAITV_MAX {
        return -super::EINVAL as i64;
    }
    if deadline.is_some() && clockid != CLOCK_MONOTONIC && clockid != CLOCK_REALTIME {
        return -super::EINVAL as i64;
    }
    let mut local = vec![FutexWaitv::default(); nr_waiters];
    let bytes = nr_waiters.saturating_mul(core::mem::size_of::<FutexWaitv>());
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            local.as_mut_ptr() as *mut u8,
            waiters as *const u8,
            bytes,
        )
    };
    if not_copied != 0 {
        return -EFAULT as i64;
    }
    for waiter in &local {
        if waiter._reserved != 0
            || waiter.flags & FUTEX_32 == 0
            || waiter.flags & !(FUTEX_32 | FUTEX_PRIVATE_FLAG) != 0
            || waiter.val > u32::MAX as u64
        {
            return -super::EINVAL as i64;
        }
    }
    unsafe { futex_waitv_deadline(&local, deadline) }
}

pub unsafe fn sys_futex_wake2(uaddr: u64, mask: u64, nr: i32, flags: u32) -> i64 {
    if mask > u32::MAX as u64 {
        return -super::EINVAL as i64;
    }
    if let Err(errno) = unsafe { super::core_ops::futex2_prepare_key(uaddr, flags) } {
        return -(errno as i64);
    }
    unsafe { futex_wake(uaddr, nr, mask as u32, flags & FUTEX2_PRIVATE != 0) }
}

pub unsafe fn sys_futex_wait2(uaddr: u64, val: u64, mask: u64, flags: u32, timeout_ns: u64) -> i64 {
    let deadline = (timeout_ns != 0).then(|| FutexDeadline::relative_monotonic(timeout_ns));
    unsafe { sys_futex_wait2_deadline(uaddr, val, mask, flags, deadline) }
}

pub unsafe fn sys_futex_wait2_deadline(
    uaddr: u64,
    val: u64,
    mask: u64,
    flags: u32,
    deadline: Option<FutexDeadline>,
) -> i64 {
    if val > u32::MAX as u64 || mask > u32::MAX as u64 {
        return -super::EINVAL as i64;
    }
    if let Err(errno) = unsafe { super::core_ops::futex2_prepare_key(uaddr, flags) } {
        return -(errno as i64);
    }
    unsafe {
        futex_wait_deadline(
            uaddr,
            val as u32,
            mask as u32,
            deadline,
            flags & FUTEX2_PRIVATE != 0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::futex::{EFAULT, EINVAL, FUTEX_OWNER_DIED, FUTEX_TID_MASK, FUTEX_WAITERS};

    #[test]
    fn cmd_mask_strips_private_and_clock_flags() {
        let combined = FUTEX_WAIT | FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME;
        assert_eq!(combined & FUTEX_CMD_MASK, FUTEX_WAIT);
    }

    #[test]
    fn futex_owner_bits_match_uapi() {
        assert_eq!(FUTEX_WAITERS, 0x80000000);
        assert_eq!(FUTEX_OWNER_DIED, 0x40000000);
        assert_eq!(FUTEX_TID_MASK, 0x3fffffff);
    }

    #[test]
    fn syscall_m78_futex_dispatch_parity() {
        let mut word = 0u32;
        let addr = &mut word as *mut u32 as u64;
        assert!(unsafe { sys_futex(addr, FUTEX_WAKE, 1, 0, 0, 0) } >= 0);
        assert_eq!(
            unsafe { sys_futex(addr, 0xffff, 0, 0, 0, 0) },
            -ENOSYS as i64
        );
        assert_eq!(
            unsafe { sys_futex(0, FUTEX_WAIT, 0, 0, 0, 0) },
            -EFAULT as i64
        );
        assert!(unsafe { sys_futex(addr, FUTEX_WAKE | FUTEX_PRIVATE_FLAG, 1, 0, 0, 0) } >= 0);
        assert_ne!(EINVAL, ENOSYS);
    }

    #[test]
    fn clock_realtime_is_rejected_for_plain_wait() {
        let mut word = 0u32;
        let addr = &mut word as *mut u32 as u64;
        assert_eq!(
            unsafe { sys_futex(addr, FUTEX_WAIT | FUTEX_CLOCK_REALTIME, 0, 0, 0, 0) },
            -ENOSYS as i64
        );
    }

    #[test]
    fn waitv_requires_futex32_waiter_flags() {
        let word = 0u32;
        let waiters = [FutexWaitv {
            val: 0,
            uaddr: &word as *const u32 as u64,
            flags: FUTEX_PRIVATE_FLAG,
            _reserved: 0,
        }];
        assert_eq!(
            unsafe {
                sys_futex_waitv(
                    waiters.as_ptr() as u64,
                    waiters.len(),
                    0,
                    0,
                    CLOCK_MONOTONIC,
                )
            },
            -EINVAL as i64
        );
    }
}
