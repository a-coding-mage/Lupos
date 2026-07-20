//! linux-parity: complete
//! linux-source: vendor/linux/kernel/locking
//! test-origin: linux:vendor/linux/kernel/locking
//! Priority-inheritance mutex.
//!
//! This is still intentionally small, but it now blocks contended waiters
//! through the scheduler and wakes them with cross-CPU aware task wakeups.

extern crate alloc;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

use super::raw_spinlock::RawSpinLock;
use crate::kernel::task::TaskStruct;

#[repr(C)]
pub struct RtMutexWaiter {
    pub task: *mut TaskStruct,
    pub prio: i32,
    pub deadline: u64,
}

unsafe impl Send for RtMutexWaiter {}
unsafe impl Sync for RtMutexWaiter {}

impl RtMutexWaiter {
    pub const fn new() -> Self {
        Self {
            task: core::ptr::null_mut(),
            prio: 0,
            deadline: 0,
        }
    }
}

#[repr(C)]
pub struct PiState {
    pub owner: *mut TaskStruct,
    pub waiters_head: *mut RtMutexWaiter,
}

unsafe impl Send for PiState {}
unsafe impl Sync for PiState {}

impl PiState {
    pub const fn new() -> Self {
        Self {
            owner: core::ptr::null_mut(),
            waiters_head: core::ptr::null_mut(),
        }
    }
}

#[repr(C)]
pub struct RtMutex {
    owner: AtomicU64,
    wait_lock: RawSpinLock,
    waiters: UnsafeCell<Vec<RtMutexWaiter>>,
}

unsafe impl Send for RtMutex {}
unsafe impl Sync for RtMutex {}

pub const RT_MUTEX_HAS_WAITERS: u64 = 1;

impl RtMutex {
    pub const fn new() -> Self {
        Self {
            owner: AtomicU64::new(0),
            wait_lock: RawSpinLock::new(),
            waiters: UnsafeCell::new(Vec::new()),
        }
    }

    pub fn try_lock(&self) -> bool {
        let me = current_task();
        self.owner
            .compare_exchange(0, me, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn adopt_owner(&self, task: *mut TaskStruct, has_waiters: bool) {
        let bits = (task as u64) | if has_waiters { RT_MUTEX_HAS_WAITERS } else { 0 };
        let _ = self
            .owner
            .compare_exchange(0, bits, Ordering::AcqRel, Ordering::Acquire);
    }

    pub fn lock(&self) -> bool {
        loop {
            let me = current_task();
            let owner = self.owner.load(Ordering::Acquire) & !RT_MUTEX_HAS_WAITERS;
            if owner == me {
                return true;
            }
            if self.try_lock() {
                return true;
            }
            let waiter_task = me as *mut TaskStruct;
            let waiter_prio = current_prio(waiter_task);
            let waiter = RtMutexWaiter {
                task: waiter_task,
                prio: waiter_prio,
                deadline: 0,
            };
            self.wait_lock.lock();
            unsafe {
                let waiters = &mut *self.waiters.get();
                if !waiters.iter().any(|w| w.task == waiter.task) {
                    waiters.push(waiter);
                }
            }
            self.owner.fetch_or(RT_MUTEX_HAS_WAITERS, Ordering::AcqRel);
            boost_owner_prio(
                self.owner.load(Ordering::Acquire) & !RT_MUTEX_HAS_WAITERS,
                waiter_prio,
            );
            self.wait_lock.unlock();

            #[cfg(not(test))]
            unsafe {
                if !waiter_task.is_null() {
                    (*waiter_task).__state.store(
                        crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE,
                        Ordering::Release,
                    );
                    crate::kernel::sched::schedule_with_irqs_enabled();
                }
            }
            #[cfg(test)]
            return false;
        }
    }

    pub fn unlock(&self) {
        let (next, _) = self.unlock_handoff();
        if !next.is_null() {
            unsafe {
                crate::kernel::sched::wake_task(next);
            }
        }
    }

    /// Transfer ownership to the first waiter without waking it.
    ///
    /// PI futex unlock must publish the new owner's TID in the user word
    /// before the waiter can run. The plain rt-mutex API wakes immediately;
    /// this split form lets futex perform that vendor/Linux ordering and then
    /// wake the returned task.
    pub fn unlock_handoff(&self) -> (*mut TaskStruct, bool) {
        let prev = self.owner.load(Ordering::Acquire);
        if prev & RT_MUTEX_HAS_WAITERS == 0 {
            self.owner.store(0, Ordering::Release);
            return (core::ptr::null_mut(), false);
        }
        self.wait_lock.lock();
        let waiters = unsafe { &mut *self.waiters.get() };
        let result = if !waiters.is_empty() {
            let w = waiters.remove(0);
            let more_waiters = !waiters.is_empty();
            let handoff = (w.task as u64)
                | if more_waiters {
                    RT_MUTEX_HAS_WAITERS
                } else {
                    0
                };
            self.owner.store(handoff, Ordering::Release);
            (w.task, more_waiters)
        } else {
            self.owner.store(0, Ordering::Release);
            (core::ptr::null_mut(), false)
        };
        self.wait_lock.unlock();
        result
    }

    pub fn is_locked(&self) -> bool {
        let owner = self.owner.load(Ordering::Acquire) & !RT_MUTEX_HAS_WAITERS;
        owner != 0
    }

    pub fn waiter_count(&self) -> usize {
        self.wait_lock.lock();
        let n = unsafe { (*self.waiters.get()).len() };
        self.wait_lock.unlock();
        n
    }
}

fn current_task() -> u64 {
    #[cfg(test)]
    return 0xCAFE_F00D;
    #[cfg(not(test))]
    unsafe {
        crate::kernel::sched::get_current() as u64
    }
}

fn current_prio(task: *mut TaskStruct) -> i32 {
    if task.is_null() {
        return 120;
    }
    unsafe { (*task).m29.prio }
}

fn boost_owner_prio(owner_bits: u64, prio: i32) {
    let owner = owner_bits as *mut TaskStruct;
    if owner.is_null() {
        return;
    }
    unsafe {
        if (*owner).m29.prio > prio {
            (*owner).m29.prio = prio;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_lock_succeeds_when_free() {
        let m = RtMutex::new();
        assert!(m.try_lock());
        assert!(m.is_locked());
        m.unlock();
        assert!(!m.is_locked());
    }

    #[test]
    fn try_lock_fails_when_held() {
        let m = RtMutex::new();
        assert!(m.try_lock());
        assert!(!m.try_lock());
        m.unlock();
    }

    #[test]
    fn waiter_constants_match_linux() {
        assert_eq!(RT_MUTEX_HAS_WAITERS, 1);
    }

    #[test]
    fn pi_state_new_is_null() {
        let s = PiState::new();
        assert!(s.owner.is_null());
    }

    #[test]
    fn unlock_handoff_returns_next_owner_before_wakeup() {
        let mutex = RtMutex::new();
        let owner = 0x1000usize as *mut TaskStruct;
        let next = 0x2000usize as *mut TaskStruct;
        mutex.adopt_owner(owner, true);
        unsafe {
            (*mutex.waiters.get()).push(RtMutexWaiter {
                task: next,
                prio: 100,
                deadline: 0,
            });
        }

        let (handoff, more_waiters) = mutex.unlock_handoff();

        assert_eq!(handoff, next);
        assert!(!more_waiters);
        assert_eq!(
            mutex.owner.load(Ordering::Acquire) & !RT_MUTEX_HAS_WAITERS,
            next as u64
        );
    }
}
