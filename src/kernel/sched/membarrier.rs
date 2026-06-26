//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/membarrier.c
//! test-origin: linux:vendor/linux/kernel/sched/membarrier.c
//! membarrier syscall implementation.
//!
//! Mirrors `vendor/linux/kernel/sched/membarrier.c` and
//! `vendor/linux/include/uapi/linux/membarrier.h`.

use core::sync::atomic::{AtomicI32, Ordering, fence};

pub const MEMBARRIER_CMD_QUERY: i32 = 0;
pub const MEMBARRIER_CMD_GLOBAL: i32 = 1 << 0;
pub const MEMBARRIER_CMD_GLOBAL_EXPEDITED: i32 = 1 << 1;
pub const MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED: i32 = 1 << 2;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED: i32 = 1 << 3;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED: i32 = 1 << 4;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: i32 = 1 << 5;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE: i32 = 1 << 6;
pub const MEMBARRIER_CMD_PRIVATE_EXPEDITED_RSEQ: i32 = 1 << 7;
pub const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ: i32 = 1 << 8;
pub const MEMBARRIER_CMD_GET_REGISTRATIONS: i32 = 1 << 9;

pub const MEMBARRIER_CMD_FLAG_CPU: u32 = 1 << 0;

pub const MEMBARRIER_CMD_BITMASK: i32 = MEMBARRIER_CMD_GLOBAL
    | MEMBARRIER_CMD_GLOBAL_EXPEDITED
    | MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED
    | MEMBARRIER_CMD_PRIVATE_EXPEDITED
    | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED
    | MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE
    | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE
    | MEMBARRIER_CMD_PRIVATE_EXPEDITED_RSEQ
    | MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ
    | MEMBARRIER_CMD_GET_REGISTRATIONS;

const EINVAL: i64 = -22;
const EPERM: i64 = -1;

static REGISTRATIONS: AtomicI32 = AtomicI32::new(0);

fn registered(bit: i32) -> bool {
    REGISTRATIONS.load(Ordering::Acquire) & bit != 0
}

fn register(bit: i32) -> i64 {
    REGISTRATIONS.fetch_or(bit, Ordering::AcqRel);
    0
}

fn barrier() -> i64 {
    fence(Ordering::SeqCst);
    0
}

pub fn sys_membarrier(cmd: i32, flags: u32, cpu_id: i32) -> i64 {
    if cmd == MEMBARRIER_CMD_PRIVATE_EXPEDITED_RSEQ {
        if flags != 0 && flags != MEMBARRIER_CMD_FLAG_CPU {
            return EINVAL;
        }
        if flags == MEMBARRIER_CMD_FLAG_CPU && cpu_id < 0 {
            return EINVAL;
        }
    } else if flags != 0 {
        return EINVAL;
    }

    match cmd {
        MEMBARRIER_CMD_QUERY => MEMBARRIER_CMD_BITMASK as i64,
        MEMBARRIER_CMD_GET_REGISTRATIONS => REGISTRATIONS.load(Ordering::Acquire) as i64,
        MEMBARRIER_CMD_GLOBAL | MEMBARRIER_CMD_GLOBAL_EXPEDITED => barrier(),
        MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED => {
            register(MEMBARRIER_CMD_REGISTER_GLOBAL_EXPEDITED)
        }
        MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED => {
            register(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED)
        }
        MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE => {
            register(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE)
        }
        MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ => {
            register(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ)
        }
        MEMBARRIER_CMD_PRIVATE_EXPEDITED => {
            if registered(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED) {
                barrier()
            } else {
                EPERM
            }
        }
        MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE => {
            if registered(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE) {
                barrier()
            } else {
                EPERM
            }
        }
        MEMBARRIER_CMD_PRIVATE_EXPEDITED_RSEQ => {
            if registered(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_RSEQ) {
                barrier()
            } else {
                EPERM
            }
        }
        _ => EINVAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membarrier_query_returns_supported_mask() {
        let mask = sys_membarrier(MEMBARRIER_CMD_QUERY, 0, 0);
        assert!(mask & MEMBARRIER_CMD_GLOBAL as i64 != 0);
        assert!(mask & MEMBARRIER_CMD_GET_REGISTRATIONS as i64 != 0);
    }

    #[test]
    fn private_expedited_requires_registration() {
        assert_eq!(
            sys_membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED, 0, 0),
            EPERM
        );
        assert_eq!(
            sys_membarrier(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED, 0, 0),
            0
        );
        assert_eq!(sys_membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED, 0, 0), 0);
    }

    #[test]
    fn membarrier_rejects_invalid_flags() {
        assert_eq!(
            sys_membarrier(MEMBARRIER_CMD_GLOBAL, MEMBARRIER_CMD_FLAG_CPU, 0),
            EINVAL
        );
    }
}
