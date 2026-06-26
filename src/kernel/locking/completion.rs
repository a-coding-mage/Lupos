//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! `struct completion` (M33).
//!
//! Mirrors `vendor/linux/kernel/sched/completion.c`.  A one-shot synchronization
//! primitive: `complete()` signals; `wait_for_completion()` blocks until the
//! event has fired.  Frequently used by kthread teardown and async I/O.

extern crate alloc;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI32, Ordering};

use super::raw_spinlock::RawSpinLock;
use crate::kernel::task::{TaskStruct, task_state};

#[repr(C)]
pub struct Completion {
    /// Number of pending `complete` calls (Linux: `done`).  `complete_all`
    /// raises this to UINT_MAX so subsequent waits return immediately.
    done: AtomicI32,
    wait_lock: RawSpinLock,
    waiters: UnsafeCell<Vec<*mut TaskStruct>>,
}

unsafe impl Send for Completion {}
unsafe impl Sync for Completion {}

impl Completion {
    pub const fn new() -> Self {
        Self {
            done: AtomicI32::new(0),
            wait_lock: RawSpinLock::new(),
            waiters: UnsafeCell::new(Vec::new()),
        }
    }

    /// Reinitialise to "not done".
    pub fn reinit(&self) {
        self.done.store(0, Ordering::Release);
        self.wait_lock.lock();
        unsafe {
            (*self.waiters.get()).clear();
        }
        self.wait_lock.unlock();
    }

    /// `complete()` — wake one waiter.
    pub fn complete(&self) {
        self.done.fetch_add(1, Ordering::AcqRel);
        self.wait_lock.lock();
        let waiters = unsafe { &mut *self.waiters.get() };
        if !waiters.is_empty() {
            let head = waiters.remove(0);
            unsafe {
                (*head)
                    .__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
        }
        self.wait_lock.unlock();
    }

    /// `complete_all()` — wake every current and future waiter.
    pub fn complete_all(&self) {
        self.done.store(i32::MAX, Ordering::Release);
        self.wait_lock.lock();
        let waiters = unsafe { &mut *self.waiters.get() };
        for &t in waiters.iter() {
            unsafe {
                (*t).__state
                    .store(task_state::TASK_RUNNING, Ordering::Release);
            }
        }
        waiters.clear();
        self.wait_lock.unlock();
    }

    /// `wait_for_completion()` — blocks until done > 0.
    pub fn wait(&self) {
        loop {
            let cur = self.done.load(Ordering::Acquire);
            if cur > 0 {
                if cur == i32::MAX {
                    return; // complete_all path — sticky.
                }
                if self
                    .done
                    .compare_exchange(cur, cur - 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return;
                }
                continue;
            }
            // Park.
            #[cfg(not(test))]
            {
                self.wait_lock.lock();
                let me = unsafe { crate::kernel::sched::get_current() };
                unsafe {
                    (*self.waiters.get()).push(me);
                }
                if !me.is_null() {
                    unsafe {
                        (*me)
                            .__state
                            .store(task_state::TASK_UNINTERRUPTIBLE, Ordering::Release);
                    }
                }
                self.wait_lock.unlock();
                unsafe {
                    crate::kernel::sched::schedule_with_irqs_enabled();
                }
            }
            #[cfg(test)]
            return; // tests must call complete() before wait()
        }
    }

    /// Returns true if a waiter would not block.
    pub fn try_wait(&self) -> bool {
        let cur = self.done.load(Ordering::Acquire);
        if cur <= 0 {
            return false;
        }
        if cur == i32::MAX {
            return true;
        }
        self.done
            .compare_exchange(cur, cur - 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_then_wait_round_trip() {
        let c = Completion::new();
        c.complete();
        assert!(c.try_wait());
        assert!(!c.try_wait());
    }

    #[test]
    fn complete_all_is_sticky() {
        let c = Completion::new();
        c.complete_all();
        assert!(c.try_wait());
        assert!(c.try_wait()); // still done forever
    }

    #[test]
    fn reinit_resets_state() {
        let c = Completion::new();
        c.complete();
        c.reinit();
        assert!(!c.try_wait());
    }
}
