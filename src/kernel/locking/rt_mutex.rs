//! linux-parity: partial
//! linux-source: vendor/linux/kernel/locking/rtmutex.c
//! linux-source: vendor/linux/kernel/sched/core.c
//! test-origin: linux:vendor/linux/kernel/locking
//! Priority-inheritance mutex.
//!
//! This is still intentionally small, but it now blocks contended waiters
//! through the scheduler and wakes them with cross-CPU aware task wakeups.

extern crate alloc;
#[cfg(test)]
extern crate std;

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
                waiter_task,
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
        let previous_owner = (prev & !RT_MUTEX_HAS_WAITERS) as *mut TaskStruct;
        if prev & RT_MUTEX_HAS_WAITERS == 0 {
            self.owner.store(0, Ordering::Release);
            restore_owner_prio(previous_owner);
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
        // Linux removes the old owner's donation and requeues it before the
        // handoff target can run. `unlock()` and futex PI publish/wake the new
        // owner only after this function returns.
        restore_owner_prio(previous_owner);
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
    {
        static TEST_CURRENT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        return *TEST_CURRENT.get_or_init(|| {
            let mut task = alloc::boxed::Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
            task.m29 = crate::kernel::task::M29SchedFields::zeroed();
            alloc::boxed::Box::into_raw(task) as usize
        }) as u64;
    }
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

fn boost_owner_prio(owner_bits: u64, pi_task: *mut TaskStruct) {
    let owner = owner_bits as *mut TaskStruct;
    if owner.is_null() || pi_task.is_null() {
        return;
    }
    unsafe {
        if (*owner).m29.prio > (*pi_task).m29.prio {
            crate::kernel::sched::rt_mutex_setprio(owner, pi_task);
        }
    }
}

fn restore_owner_prio(owner: *mut TaskStruct) {
    if owner.is_null() {
        return;
    }
    unsafe {
        crate::kernel::sched::rt_mutex_setprio(owner, core::ptr::null_mut());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

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

    // test-origin: linux:vendor/linux/tools/testing/selftests/futex/functional/futex_requeue_pi.c
    // and linux:vendor/linux/kernel/sched/core.c:rt_mutex_setprio
    // The upstream userspace test covers PI ownership and scheduling. This
    // focused adaptation checks the internal queue invariant that is otherwise
    // only observable as a system-wide scheduler hang.
    #[test]
    fn pi_boost_keeps_rt_task_removable_during_policy_reset() {
        const TEST_CPU: u32 = (crate::kernel::sched::MAX_CPUS - 2) as u32;

        struct ResetRunqueue(u32);

        impl Drop for ResetRunqueue {
            fn drop(&mut self) {
                let _ = crate::kernel::sched::rq::with_rq(self.0, |rq| {
                    *rq = crate::kernel::sched::rq::Rq::new(self.0);
                });
            }
        }

        crate::kernel::sched::rq::init_rqs();
        crate::kernel::sched::rq::with_rq(TEST_CPU, |rq| {
            *rq = crate::kernel::sched::rq::Rq::new(TEST_CPU);
        })
        .expect("test runqueue exists");
        let _reset_runqueue = ResetRunqueue(TEST_CPU);

        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;
        task.m29 = crate::kernel::task::M29SchedFields::zeroed();
        task.m29.sched_class = &crate::kernel::sched::fair::FAIR_SCHED_CLASS;
        task.m29.policy = crate::kernel::sched::prio::SCHED_NORMAL;
        task.m29.prio = crate::kernel::sched::prio::DEFAULT_PRIO;
        task.m29.normal_prio = crate::kernel::sched::prio::DEFAULT_PRIO;
        task.m29.static_prio = crate::kernel::sched::prio::DEFAULT_PRIO;
        task.m29.cpus_mask = crate::kernel::sched::entity::CpuMask::one(TEST_CPU);
        task.m29.cpus_ptr = &task.m29.cpus_mask;
        task.m29.nr_cpus_allowed = 1;
        task.thread_info.cpu = TEST_CPU;

        unsafe {
            crate::kernel::sched::enqueue_on_rq(
                TEST_CPU,
                task_ptr,
                crate::kernel::sched::class::ENQUEUE_INITIAL,
            );
        }
        let fifo = crate::kernel::sched::syscalls::SchedAttr {
            size: crate::kernel::sched::syscalls::SCHED_ATTR_SIZE_VER1,
            sched_policy: crate::kernel::sched::prio::SCHED_FIFO,
            sched_priority: 83,
            ..crate::kernel::sched::syscalls::SchedAttr::default()
        };
        assert_eq!(
            unsafe { crate::kernel::sched::syscalls::sys_sched_setattr(task_ptr, &fifo) },
            0
        );
        crate::kernel::sched::rq::with_rq(TEST_CPU, |rq| {
            assert_eq!(rq.rt.highest_prio(), Some(16));
        })
        .expect("test runqueue exists");

        let mut donor = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        donor.m29 = crate::kernel::task::M29SchedFields::zeroed();
        donor.m29.prio = 11;
        let donor_ptr = &mut *donor as *mut TaskStruct;
        let mutex = RtMutex::new();
        mutex.adopt_owner(task_ptr, true);
        unsafe {
            (*mutex.waiters.get()).push(RtMutexWaiter {
                task: donor_ptr,
                prio: donor.m29.prio,
                deadline: 0,
            });
        }

        boost_owner_prio(task_ptr as u64, donor_ptr);
        assert_eq!(task.m29.prio, 11);

        let (handoff, more_waiters) = mutex.unlock_handoff();
        assert_eq!(handoff, donor_ptr);
        assert!(!more_waiters);
        assert_eq!(task.m29.prio, 16);
        crate::kernel::sched::rq::with_rq(TEST_CPU, |rq| {
            assert_eq!(rq.rt.highest_prio(), Some(16));
        })
        .expect("test runqueue exists");

        let normal = crate::kernel::sched::syscalls::SchedAttr {
            size: crate::kernel::sched::syscalls::SCHED_ATTR_SIZE_VER1,
            sched_policy: crate::kernel::sched::prio::SCHED_NORMAL,
            ..crate::kernel::sched::syscalls::SchedAttr::default()
        };
        assert_eq!(
            unsafe { crate::kernel::sched::syscalls::sys_sched_setattr(task_ptr, &normal) },
            0
        );
        crate::kernel::sched::rq::with_rq(TEST_CPU, |rq| {
            assert_eq!(rq.rt.nr_running, 0);
            assert_eq!(rq.cfs.nr_running, 1);
            assert_eq!(rq.nr_running, 1);
        })
        .expect("test runqueue exists");
        assert_eq!(task.m29.rt.on_rq, 0);
        assert_eq!(task.m29.se.on_rq, 1);
    }

    #[test]
    fn unlock_handoff_returns_next_owner_before_wakeup() {
        let mutex = RtMutex::new();
        let mut owner_task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        owner_task.m29 = crate::kernel::task::M29SchedFields::zeroed();
        let owner = &mut *owner_task as *mut TaskStruct;
        let mut next_task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        next_task.m29 = crate::kernel::task::M29SchedFields::zeroed();
        let next = &mut *next_task as *mut TaskStruct;
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
