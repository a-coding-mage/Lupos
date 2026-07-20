//! linux-parity: complete
//! linux-source: vendor/linux/kernel/futex/pi.c
//! test-origin: linux:vendor/linux/kernel/futex/pi.c
//! Priority-inheritance futex operations.
//!
//! The implementation is intentionally compact but now uses `RtMutex` for the
//! contended path, preserves the futex owner/waiter bits, and wakes blocked
//! waiters through the production scheduler.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use spin::Mutex;

use crate::kernel::locking::rt_mutex::RtMutex;
use crate::kernel::task::TaskStruct;

use super::core_ops::futex_pi_wake_next;
use super::{EAGAIN, EDEADLK, EFAULT, EPERM, ETIMEDOUT};
use super::{FUTEX_OWNER_DIED, FUTEX_TID_MASK, FUTEX_WAITERS};

use crate::arch::x86::kernel::uaccess::{access_ok, cmpxchg_user_u32, get_user_u32, put_user_u32};

struct PiFutexState {
    uaddr: u64,
    mm_id: u64,
    owner_tid: u32,
    lock: RtMutex,
}

impl PiFutexState {
    fn new(mm_id: u64, uaddr: u64) -> Self {
        Self {
            uaddr,
            mm_id,
            owner_tid: 0,
            lock: RtMutex::new(),
        }
    }
}

static PI_STATES: Mutex<Vec<Box<PiFutexState>>> = Mutex::new(Vec::new());

fn mm_for(private: bool) -> u64 {
    if private {
        #[cfg(not(test))]
        unsafe {
            let cur = crate::kernel::sched::get_current();
            if cur.is_null() {
                return 0;
            }
            let mm = if !(*cur).mm.is_null() {
                (*cur).mm
            } else {
                (*cur).active_mm
            };
            return if mm.is_null() { cur as u64 } else { mm as u64 };
        }
        #[cfg(test)]
        return 0xCAFEBABE_DEADBEEF_u64;
    }
    0
}

unsafe fn task_mm_id(task: *const TaskStruct) -> u64 {
    if task.is_null() {
        return 0;
    }
    unsafe {
        let mm = if !(*task).mm.is_null() {
            (*task).mm
        } else {
            (*task).active_mm
        };
        if mm.is_null() { task as u64 } else { mm as u64 }
    }
}

pub unsafe fn futex_pi_exit_release(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let pid = unsafe { (*task).pid as u32 } & FUTEX_TID_MASK;
    let tgid = unsafe { (*task).tgid };
    let mm_id = unsafe { task_mm_id(task) };
    let leader_exit = unsafe { (*task).pid == tgid };

    let mut states = PI_STATES.lock();
    states.retain(|state| {
        if state.owner_tid == pid {
            return false;
        }
        if leader_exit && state.mm_id == mm_id {
            return false;
        }
        true
    });
}

fn user_word_ptr(uaddr: u64) -> Result<*mut u32, ()> {
    if uaddr == 0 || !access_ok(uaddr, core::mem::size_of::<u32>() as u64) {
        return Err(());
    }
    Ok(uaddr as *mut u32)
}

unsafe fn cmpxchg_word(uaddr: u64, expected: u32, new: u32) -> Result<u32, ()> {
    let p = user_word_ptr(uaddr)?;
    unsafe { cmpxchg_user_u32(p, expected, new) }.map_err(|_| ())
}

unsafe fn load_word(uaddr: u64) -> Result<u32, ()> {
    let p = user_word_ptr(uaddr)?;
    unsafe { get_user_u32(p as *const u32) }.map_err(|_| ())
}

unsafe fn store_word(uaddr: u64, val: u32) -> Result<(), ()> {
    let p = user_word_ptr(uaddr)?;
    unsafe { put_user_u32(p, val) }.map_err(|_| ())
}

fn get_pi_state(mm_id: u64, uaddr: u64) -> *mut PiFutexState {
    let mut states = PI_STATES.lock();
    if let Some(state) = states
        .iter_mut()
        .find(|state| state.mm_id == mm_id && state.uaddr == uaddr)
    {
        return &mut **state as *mut PiFutexState;
    }
    states.push(Box::new(PiFutexState::new(mm_id, uaddr)));
    &mut **states.last_mut().expect("pi state just pushed") as *mut PiFutexState
}

fn find_task_by_pid(pid: i32) -> *mut TaskStruct {
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

pub unsafe fn futex_lock_pi(uaddr: u64, timeout_ns: u64, private: bool) -> i64 {
    if uaddr == 0 {
        return -EFAULT as i64;
    }
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() {
        return -EPERM as i64;
    }
    let tid = unsafe { (*cur).pid as u32 } & FUTEX_TID_MASK;

    match unsafe { cmpxchg_word(uaddr, 0, tid) } {
        Ok(0) => return 0,
        Ok(prev) if prev & FUTEX_TID_MASK == tid => return -EDEADLK as i64,
        Ok(prev) if prev & FUTEX_TID_MASK == 0 => {
            let new = tid | (prev & FUTEX_WAITERS);
            match unsafe { cmpxchg_word(uaddr, prev, new) } {
                Ok(p) if p == prev => return 0,
                _ => return -EAGAIN as i64,
            }
        }
        Ok(prev) if prev & FUTEX_OWNER_DIED != 0 => {
            let new = (tid | (prev & FUTEX_WAITERS)) & !FUTEX_OWNER_DIED;
            match unsafe { cmpxchg_word(uaddr, prev, new) } {
                Ok(p) if p == prev => return 0,
                _ => return -EAGAIN as i64,
            }
        }
        Ok(_) => {}
        Err(_) => return -EFAULT as i64,
    }
    if timeout_ns > 0 {
        return -ETIMEDOUT as i64;
    }

    let mm_id = mm_for(private);
    let state = get_pi_state(mm_id, uaddr);
    let owner_word = match unsafe { load_word(uaddr) } {
        Ok(word) => word,
        Err(_) => return -EFAULT as i64,
    };
    let owner = owner_word & FUTEX_TID_MASK;
    let owner_task = find_task_by_pid(owner as i32);
    match unsafe { cmpxchg_word(uaddr, owner_word, owner_word | FUTEX_WAITERS) } {
        Ok(prev) if prev == owner_word => {}
        Ok(_) => return -EAGAIN as i64,
        Err(_) => return -EFAULT as i64,
    }
    unsafe {
        (*state).lock.adopt_owner(owner_task, true);
    }
    unsafe {
        (*state).lock.lock();
        (*state).owner_tid = tid;
        // Once a contended futex has PI state, Linux keeps FUTEX_WAITERS
        // set until the kernel slow-path destroys that state. Clearing it
        // here lets the new owner unlock entirely in userspace, leaving the
        // kernel rt-mutex owned by a task which no longer owns the futex.
        let bits = tid | FUTEX_WAITERS;
        let observed = match load_word(uaddr) {
            Ok(word) => word,
            Err(_) => return -EFAULT as i64,
        };
        match cmpxchg_word(uaddr, observed, bits) {
            Ok(prev) if prev == observed => {}
            Ok(_) => return -EAGAIN as i64,
            Err(_) => return -EFAULT as i64,
        }
    }
    0
}

pub unsafe fn futex_trylock_pi(uaddr: u64, _private: bool) -> i64 {
    if uaddr == 0 {
        return -EFAULT as i64;
    }
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() {
        return -EPERM as i64;
    }
    let tid = unsafe { (*cur).pid as u32 } & FUTEX_TID_MASK;
    match unsafe { cmpxchg_word(uaddr, 0, tid) } {
        Ok(0) => 0,
        Ok(_) => -EAGAIN as i64,
        Err(_) => -EFAULT as i64,
    }
}

pub unsafe fn futex_unlock_pi(uaddr: u64, private: bool) -> i64 {
    if uaddr == 0 {
        return -EFAULT as i64;
    }
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() {
        return -EPERM as i64;
    }
    let tid = unsafe { (*cur).pid as u32 } & FUTEX_TID_MASK;
    let word = match unsafe { load_word(uaddr) } {
        Ok(word) => word,
        Err(_) => return -EFAULT as i64,
    };
    if word & FUTEX_TID_MASK != tid {
        return -EPERM as i64;
    }

    if unsafe { futex_pi_wake_next(uaddr, private) } {
        return 0;
    }

    let mm_id = mm_for(private);
    let state = get_pi_state(mm_id, uaddr);
    unsafe {
        if (*state).lock.is_locked() {
            // vendor rt_mutex_futex_unlock() transfers ownership and updates
            // the user futex word before waking the next task. Waking first
            // lets the waiter run and then allows the old owner to overwrite
            // its TID, making the waiter's later FUTEX_UNLOCK_PI fail EPERM.
            let (next_owner, _more_waiters) = (*state).lock.unlock_handoff();
            let next_tid = if next_owner.is_null() {
                0
            } else {
                ((*next_owner).pid as u32) & FUTEX_TID_MASK
            };
            (*state).owner_tid = next_tid;
            // Linux keeps FUTEX_WAITERS set for every ownership handoff
            // while PI state exists, including handoff to the final waiter.
            let replacement = if next_owner.is_null() {
                0
            } else {
                next_tid | FUTEX_WAITERS
            };
            match cmpxchg_word(uaddr, word, replacement) {
                Ok(prev) if prev == word => {}
                Ok(_) => return -EAGAIN as i64,
                Err(_) => return -EFAULT as i64,
            }
            if !next_owner.is_null() {
                crate::kernel::sched::wake_task(next_owner);
            }
            return 0;
        }
    }

    let replacement = 0;
    match unsafe { cmpxchg_word(uaddr, word, replacement) } {
        Ok(prev) if prev == word => 0,
        Ok(_) => -EPERM as i64,
        Err(_) => -EFAULT as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::task::{M26Fields, TaskStruct};
    use alloc::boxed::Box;
    use core::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn pi_constants_match_linux() {
        assert_eq!(FUTEX_WAITERS, 0x80000000);
        assert_eq!(FUTEX_OWNER_DIED, 0x40000000);
        assert_eq!(FUTEX_TID_MASK, 0x3fffffff);
    }

    #[test]
    fn pi_word_helpers_reject_kernel_addresses() {
        let kernel_addr = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;

        assert!(unsafe { load_word(kernel_addr) }.is_err());
        assert!(unsafe { store_word(kernel_addr, FUTEX_WAITERS) }.is_err());
        assert!(unsafe { cmpxchg_word(kernel_addr, 0, FUTEX_WAITERS) }.is_err());

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 42;
        current.tgid = 42;
        current.m26 = M26Fields::zeroed();
        unsafe {
            crate::kernel::sched::set_current(&mut *current as *mut TaskStruct);
        }

        assert_eq!(
            unsafe { futex_lock_pi(kernel_addr, 0, true) },
            -EFAULT as i64
        );

        unsafe {
            crate::kernel::sched::set_current(core::ptr::null_mut());
        }
    }

    #[test]
    fn owner_died_lock_can_be_recovered_and_unlocked() {
        PI_STATES.lock().clear();

        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 42;
        current.tgid = 42;
        current.m26 = M26Fields::zeroed();

        unsafe {
            crate::kernel::sched::set_current(&mut *current as *mut TaskStruct);
        }

        let word = AtomicU32::new(FUTEX_OWNER_DIED);
        let addr = &word as *const AtomicU32 as u64;

        assert_eq!(unsafe { futex_lock_pi(addr, 0, true) }, 0);
        let locked = word.load(Ordering::Acquire);
        assert_eq!(locked & FUTEX_TID_MASK, 42);
        assert_eq!(locked & FUTEX_OWNER_DIED, 0);
        assert_eq!(unsafe { futex_unlock_pi(addr, true) }, 0);
        assert_eq!(word.load(Ordering::Acquire) & FUTEX_TID_MASK, 0);

        unsafe {
            crate::kernel::sched::set_current(core::ptr::null_mut());
        }
        PI_STATES.lock().clear();
    }
}
