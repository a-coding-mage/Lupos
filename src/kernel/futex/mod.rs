//! linux-parity: partial
//! linux-source: vendor/linux/kernel/futex
//! test-origin: linux:vendor/linux/kernel/futex
//! Futex — fast userspace mutex (M32).
//!
//! Mirrors `vendor/linux/kernel/futex/`.  Userspace stores the lock word at a
//! `u32` address `uaddr`; the kernel parks waiters on a hash-bucket keyed by
//! `(mm, uaddr)` for `FUTEX_PRIVATE_FLAG`-tagged ops, or `(inode, off)` for
//! shared mappings.
//!
//! Public surface (UAPI parity):
//!
//!   * `sys_futex(uaddr, op, val, timeout, uaddr2, val3)` — multiplexed
//!   * `sys_futex_wait(uaddr, val, mask, timeout)` — Linux 6.7+ split
//!   * `sys_futex_wake(uaddr, mask, nr)`
//!   * `sys_futex_requeue(waiters, flags, nr_wake, nr_requeue)`
//!   * `sys_set_robust_list` / `sys_get_robust_list`

pub mod core_ops;
pub mod pi;
pub mod requeue_pi;
pub mod robust;
pub mod syscalls;

pub use core_ops::*;
pub use pi::*;
pub use requeue_pi::*;
pub use robust::*;
pub use syscalls::*;

// ── UAPI op codes (vendor/linux/include/uapi/linux/futex.h) ──────────────────

pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_WAKE: u32 = 1;
pub const FUTEX_FD: u32 = 2;
pub const FUTEX_REQUEUE: u32 = 3;
pub const FUTEX_CMP_REQUEUE: u32 = 4;
pub const FUTEX_WAKE_OP: u32 = 5;
pub const FUTEX_LOCK_PI: u32 = 6;
pub const FUTEX_UNLOCK_PI: u32 = 7;
pub const FUTEX_TRYLOCK_PI: u32 = 8;
pub const FUTEX_WAIT_BITSET: u32 = 9;
pub const FUTEX_WAKE_BITSET: u32 = 10;
pub const FUTEX_WAIT_REQUEUE_PI: u32 = 11;
pub const FUTEX_CMP_REQUEUE_PI: u32 = 12;
pub const FUTEX_LOCK_PI2: u32 = 13;

pub const FUTEX_PRIVATE_FLAG: u32 = 128;
pub const FUTEX_CLOCK_REALTIME: u32 = 256;
pub const FUTEX_CMD_MASK: u32 = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

/// Robust-list ownership bits in the futex word.
pub const FUTEX_WAITERS: u32 = 0x80000000;
pub const FUTEX_OWNER_DIED: u32 = 0x40000000;
pub const FUTEX_TID_MASK: u32 = 0x3fffffff;

pub const FUTEX_BITSET_MATCH_ANY: u32 = 0xffffffff;

pub const FUTEX_WAITV_MAX: usize = 128;
pub const FUTEX_32: u32 = 0x02;
pub const FUTEX2_SIZE_MASK: u32 = 0x03;
pub const FUTEX2_NUMA: u32 = 0x04;
pub const FUTEX2_MPOL: u32 = 0x08;
pub const FUTEX2_PRIVATE: u32 = FUTEX_PRIVATE_FLAG;
pub const FUTEX2_VALID_MASK: u32 = FUTEX2_SIZE_MASK | FUTEX2_NUMA | FUTEX2_MPOL | FUTEX2_PRIVATE;

pub const FUTEX_OP_SET: u32 = 0;
pub const FUTEX_OP_ADD: u32 = 1;
pub const FUTEX_OP_OR: u32 = 2;
pub const FUTEX_OP_ANDN: u32 = 3;
pub const FUTEX_OP_XOR: u32 = 4;
pub const FUTEX_OP_OPARG_SHIFT: u32 = 8;

pub const FUTEX_OP_CMP_EQ: u32 = 0;
pub const FUTEX_OP_CMP_NE: u32 = 1;
pub const FUTEX_OP_CMP_LT: u32 = 2;
pub const FUTEX_OP_CMP_LE: u32 = 3;
pub const FUTEX_OP_CMP_GT: u32 = 4;
pub const FUTEX_OP_CMP_GE: u32 = 5;

// ── errno values ─────────────────────────────────────────────────────────────

pub const EAGAIN: i32 = 11;
pub const EINVAL: i32 = 22;
pub const ETIMEDOUT: i32 = 110;
pub const EWOULDBLOCK: i32 = EAGAIN;
pub const ESRCH: i32 = 3;
pub const EFAULT: i32 = 14;
pub const ENOSYS: i32 = 38;
pub const EPERM: i32 = 1;
pub const EDEADLK: i32 = 35;

// ── UAPI: struct futex_waitv ─────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FutexWaitv {
    pub val: u64,
    pub uaddr: u64,
    pub flags: u32,
    pub _reserved: u32,
}

const _: () = assert!(core::mem::size_of::<FutexWaitv>() == 24);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_mask_strips_private_and_clock_flags() {
        let combined = FUTEX_WAIT | FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME;
        assert_eq!(combined & FUTEX_CMD_MASK, FUTEX_WAIT);
    }

    #[test]
    fn futex_waitv_size_matches_uapi() {
        assert_eq!(core::mem::size_of::<FutexWaitv>(), 24);
    }

    #[test]
    fn futex_owner_bits_match_uapi() {
        assert_eq!(FUTEX_WAITERS, 0x80000000);
        assert_eq!(FUTEX_OWNER_DIED, 0x40000000);
        assert_eq!(FUTEX_TID_MASK, 0x3fffffff);
    }

    #[test]
    fn syscall_m78_futex_pidfd_parity() {
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
    }
}
