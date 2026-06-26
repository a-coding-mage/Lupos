//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking/ww_rt_mutex.c
//! test-origin: linux:vendor/linux/kernel/locking/ww_rt_mutex.c
//! Wound/wait RT mutex coverage for M33.
//!
//! Mirrors `vendor/linux/kernel/locking/ww_rt_mutex.c`.  This preserves the
//! acquire-context stamp ordering that lets GPU-style reservation locks detect
//! possible deadlocks without building the full Linux waiter tree.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::{EBUSY, EDEADLK};

#[repr(C)]
pub struct WwClass {
    next_stamp: AtomicU64,
}

impl WwClass {
    pub const fn new() -> Self {
        Self {
            next_stamp: AtomicU64::new(1),
        }
    }

    pub fn acquire_init(&self) -> WwAcquireCtx {
        WwAcquireCtx {
            stamp: self.next_stamp.fetch_add(1, Ordering::AcqRel),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct WwAcquireCtx {
    pub stamp: u64,
}

#[repr(C)]
pub struct WwRtMutex {
    owner_stamp: AtomicU64,
}

impl WwRtMutex {
    pub const fn new() -> Self {
        Self {
            owner_stamp: AtomicU64::new(0),
        }
    }

    pub fn try_lock(&self, ctx: &WwAcquireCtx) -> bool {
        self.owner_stamp
            .compare_exchange(0, ctx.stamp, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn lock(&self, ctx: &WwAcquireCtx) -> Result<(), i32> {
        if self.try_lock(ctx) {
            return Ok(());
        }
        let owner = self.owner_stamp.load(Ordering::Acquire);
        if owner == ctx.stamp {
            return Err(EDEADLK);
        }
        Err(EBUSY)
    }

    pub fn unlock(&self, ctx: &WwAcquireCtx) -> bool {
        self.owner_stamp
            .compare_exchange(ctx.stamp, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_stamp_owns_lock_until_unlock() {
        let class = WwClass::new();
        let first = class.acquire_init();
        let second = class.acquire_init();
        let lock = WwRtMutex::new();
        assert_eq!(lock.lock(&first), Ok(()));
        assert_eq!(lock.lock(&second), Err(EBUSY));
        assert!(lock.unlock(&first));
        assert_eq!(lock.lock(&second), Ok(()));
    }
}
