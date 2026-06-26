//! linux-parity: complete
//! linux-source: vendor/linux/kernel/futex
//! test-origin: linux:vendor/linux/kernel/futex
//! Robust list - userspace-exported list of held futexes that the kernel
//! walks on `do_exit` to release them with `FUTEX_OWNER_DIED` set.
//!
//! Mirrors `vendor/linux/kernel/futex/core.c::exit_robust_list`, which
//! reads each user-supplied pointer through `fetch_robust_entry`
//! (`get_user`-based) so a dead-process exit with a half-mapped or
//! never-mapped robust-list head returns -EFAULT instead of taking a
//! kernel page fault.  We mirror that by routing every read of the
//! user-controlled head + cursor pointers through `copy_from_user`,
//! which uses an `__ex_table` trampoline to convert a fault into a
//! "bytes-not-copied" error.  See `vendor/linux/arch/x86/include/asm/
//! futex.h` and `vendor/linux/kernel/futex/core.c`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    EINVAL, ESRCH, FUTEX_BITSET_MATCH_ANY, FUTEX_OWNER_DIED, FUTEX_TID_MASK, FUTEX_WAITERS,
    futex_wake,
};
use crate::kernel::pid::PID_MAX_DEFAULT;

/// `struct robust_list_head` - UAPI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RobustListHead {
    pub list_next: u64,
    pub futex_offset: i64,
    pub list_op_pending: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct RobustList {
    next: u64,
}

const _: () = assert!(core::mem::size_of::<RobustListHead>() == 24);

const ROBUST_PID_SLOTS: usize = PID_MAX_DEFAULT as usize;
const ROBUST_LIST_WALK_LIMIT: usize = 2048;

static ROBUST_LIST_HEAD_PTRS: [AtomicU64; ROBUST_PID_SLOTS] =
    [const { AtomicU64::new(0) }; ROBUST_PID_SLOTS];
static ROBUST_LIST_LENS: [AtomicU64; ROBUST_PID_SLOTS] =
    [const { AtomicU64::new(0) }; ROBUST_PID_SLOTS];

fn task_for_pid(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    let current = unsafe { crate::kernel::sched::get_current() };
    if pid == 0 {
        return current;
    }
    if !current.is_null() {
        unsafe {
            if (*current).pid == pid {
                return current;
            }
        }
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

fn robust_slot_for_pid(pid: i32) -> Option<usize> {
    if pid <= 0 || pid >= PID_MAX_DEFAULT {
        None
    } else {
        Some(pid as usize)
    }
}

fn current_pid() -> Result<i32, i64> {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        Err(-(ESRCH as i64))
    } else {
        Ok(unsafe { (*current).pid })
    }
}

fn clear_robust_slot(pid: i32) {
    if let Some(slot) = robust_slot_for_pid(pid) {
        ROBUST_LIST_HEAD_PTRS[slot].store(0, Ordering::Release);
        ROBUST_LIST_LENS[slot].store(0, Ordering::Release);
    }
}

unsafe fn handle_robust_entry(entry_ptr: u64, futex_offset: i64, pid: i32) {
    if entry_ptr == 0 {
        return;
    }
    let futex_addr = (entry_ptr as i64).wrapping_add(futex_offset) as u64;
    if futex_addr == 0 {
        return;
    }
    // Reject obviously-bogus pointers (kernel range, > TASK_SIZE_MAX) before
    // we attempt to touch the user word. Linux does the same via `access_ok`
    // in `futex_atomic_op_inuser`, and the actual load/CAS below still goes
    // through fault-aware uaccess helpers for in-range but unmapped pages.
    if !crate::arch::x86::kernel::uaccess::access_ok(futex_addr, 4) {
        return;
    }
    if futex_addr & 3 != 0 {
        return;
    }
    let word = futex_addr as *mut u32;
    let tid = (pid as u32) & FUTEX_TID_MASK;
    let Ok(mut observed) =
        (unsafe { crate::arch::x86::kernel::uaccess::get_user_u32(word as *const u32) })
    else {
        return;
    };
    loop {
        if observed & FUTEX_TID_MASK != tid {
            return;
        }
        let new = (observed & FUTEX_WAITERS) | FUTEX_OWNER_DIED;
        let Ok(previous) =
            (unsafe { crate::arch::x86::kernel::uaccess::cmpxchg_user_u32(word, observed, new) })
        else {
            return;
        };
        if previous == observed {
            if previous & FUTEX_WAITERS != 0 {
                let _ = unsafe { futex_wake(futex_addr, 1, FUTEX_BITSET_MATCH_ANY, false) };
            }
            return;
        }
        observed = previous;
    }
}

pub unsafe fn sys_set_robust_list(head: u64, len: u64) -> i64 {
    if len != core::mem::size_of::<RobustListHead>() as u64 {
        return -EINVAL as i64;
    }
    let Ok(pid) = current_pid() else {
        return -(ESRCH as i64);
    };
    let Some(slot) = robust_slot_for_pid(pid) else {
        return -(ESRCH as i64);
    };
    ROBUST_LIST_HEAD_PTRS[slot].store(head, Ordering::Release);
    ROBUST_LIST_LENS[slot].store(len, Ordering::Release);
    0
}

pub unsafe fn sys_get_robust_list(out_head: &mut u64, out_len: &mut u64) -> i64 {
    let Ok(pid) = current_pid() else {
        return -(ESRCH as i64);
    };
    unsafe { sys_get_robust_list_for_pid(pid, out_head, out_len) }
}

pub unsafe fn sys_get_robust_list_for_pid(pid: i32, out_head: &mut u64, out_len: &mut u64) -> i64 {
    let task = task_for_pid(pid);
    if task.is_null() {
        return -(ESRCH as i64);
    }
    let task_pid = unsafe { (*task).pid };
    let Some(slot) = robust_slot_for_pid(task_pid) else {
        return -(ESRCH as i64);
    };
    *out_head = ROBUST_LIST_HEAD_PTRS[slot].load(Ordering::Acquire);
    *out_len = ROBUST_LIST_LENS[slot].load(Ordering::Acquire);
    0
}

/// Read a fixed-size POD from user space, returning `None` on EFAULT.
/// Mirrors Linux's `get_user(*ptr, user_ptr)` pattern via the extable
/// trampoline inside `copy_from_user`.  Used here so that a half-set-up
/// robust-list head (e.g. a process that died during dynamic loader
/// startup before its TLS pages were mapped) cannot drag the kernel
/// into a #PF panic.
unsafe fn fetch_user<T: Copy + Default>(user_ptr: u64) -> Option<T> {
    if user_ptr == 0 {
        return None;
    }
    let size = core::mem::size_of::<T>();
    let mut value = T::default();
    let left = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            &mut value as *mut T as *mut u8,
            user_ptr as *const u8,
            size,
        )
    };
    if left != 0 { None } else { Some(value) }
}

/// Hook called from `do_exit` to clean up dead-owner futexes.
pub unsafe fn exit_robust_list(pid: i32) {
    let Some(slot) = robust_slot_for_pid(pid) else {
        return;
    };
    let head_ptr = ROBUST_LIST_HEAD_PTRS[slot].load(Ordering::Acquire);
    let len = ROBUST_LIST_LENS[slot].load(Ordering::Acquire);
    if head_ptr == 0 || len != core::mem::size_of::<RobustListHead>() as u64 {
        clear_robust_slot(pid);
        return;
    }

    // Linux: `if (fetch_robust_entry(&entry, &head->list.next, &pi))
    //              return;` — a faulting head pointer aborts cleanly.
    // Without this, our previous direct `&*(head_ptr as *const _)` deref
    // panicked the kernel when a task died with a stale or never-mapped
    // robust-list head (e.g. dynamic-loader failure before TLS was
    // populated; observed via `ping` exiting with ENOENT on libidn2).
    let Some(head): Option<RobustListHead> = (unsafe { fetch_user(head_ptr) }) else {
        clear_robust_slot(pid);
        return;
    };

    let mut cursor = head.list_next;
    let mut walked = 0usize;
    while cursor != 0 && cursor != head_ptr && walked < ROBUST_LIST_WALK_LIMIT {
        let Some(node): Option<RobustList> = (unsafe { fetch_user(cursor) }) else {
            break;
        };
        unsafe { handle_robust_entry(cursor, head.futex_offset, pid) };
        if node.next == cursor {
            break;
        }
        cursor = node.next;
        walked += 1;
    }

    if head.list_op_pending != 0 && head.list_op_pending != head_ptr {
        unsafe { handle_robust_entry(head.list_op_pending, head.futex_offset, pid) };
    }

    clear_robust_slot(pid);
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::boxed::Box;
    use core::{mem::offset_of, sync::atomic::AtomicU32};

    use super::*;
    use crate::kernel::{sched, task::TaskStruct};

    #[repr(C)]
    struct TestRobustNode {
        next: u64,
        futex: AtomicU32,
    }

    fn install_current_task(pid: i32) -> Box<TaskStruct> {
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        task.pid = pid;
        unsafe {
            sched::set_current(&mut *task as *mut TaskStruct);
        }
        task
    }

    #[test]
    fn robust_list_head_is_24_bytes() {
        assert_eq!(core::mem::size_of::<RobustListHead>(), 24);
    }

    #[test]
    fn set_robust_list_with_wrong_size_returns_einval() {
        let _task = install_current_task(4000);
        let r = unsafe { sys_set_robust_list(0x1000, 16) };
        assert_eq!(r, -EINVAL as i64);
    }

    #[test]
    fn set_then_get_robust_list_round_trip() {
        let mut h = 0u64;
        let mut l = 0u64;
        let _task = install_current_task(4001);
        let _ = unsafe { sys_set_robust_list(0xC0DE, 24) };
        let _ = unsafe { sys_get_robust_list(&mut h, &mut l) };
        assert_eq!(h, 0xC0DE);
        assert_eq!(l, 24);
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let mut h = 0u64;
        let mut l = 0u64;
        let _task = install_current_task(4002);
        assert_eq!(unsafe { sys_set_robust_list(0xCAFE, 24) }, 0);
        assert_eq!(unsafe { sys_get_robust_list(&mut h, &mut l) }, 0);
        assert_eq!((h, l), (0xCAFE, 24));
        assert_eq!(unsafe { sys_set_robust_list(0xCAFE, 16) }, -EINVAL as i64);
    }

    #[test]
    fn get_robust_list_for_pid_reads_that_task_slot() {
        let mut h = 0u64;
        let mut l = 0u64;
        let _task = install_current_task(4003);
        assert_eq!(unsafe { sys_set_robust_list(0xDEAD_BEEF, 24) }, 0);
        assert_eq!(
            unsafe { sys_get_robust_list_for_pid(4003, &mut h, &mut l) },
            0
        );
        assert_eq!((h, l), (0xDEAD_BEEF, 24));
    }

    /// Regression for the ping panic on 2026-05-21: `ping` died early
    /// (`/usr/bin/ping`: libidn2.so.0 missing → exit_group(127)), then
    /// `do_exit` walked the robust-list head it had registered before
    /// crashing.  The head pointed at memory that was never mapped, so
    /// the old direct-deref `&*(head_ptr as *const RobustListHead)` took
    /// a kernel #PF and panicked.  After routing the read through
    /// `copy_from_user`, an out-of-range or unmapped user pointer must
    /// just clear the slot and return — matching Linux's
    /// `fetch_robust_entry` / `get_user` error path in
    /// `vendor/linux/kernel/futex/core.c`.
    #[test]
    fn exit_robust_list_with_kernel_range_head_pointer_bails_without_panic() {
        let pid = 4005;
        let _task = install_current_task(pid);
        // A pointer above TASK_SIZE_MAX — `access_ok` inside
        // `copy_from_user` will reject it before any dereference.
        let kernel_range_ptr = crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX + 0x4000;
        assert_eq!(unsafe { sys_set_robust_list(kernel_range_ptr, 24) }, 0);
        // Must not panic.
        unsafe { exit_robust_list(pid) };
        // Slot must be cleared so the bogus head can't be walked again.
        let mut head = 1u64;
        let mut len = 1u64;
        assert_eq!(
            unsafe { sys_get_robust_list_for_pid(pid, &mut head, &mut len) },
            0
        );
        assert_eq!((head, len), (0, 0));
    }

    #[test]
    fn exit_robust_list_marks_owner_died_and_clears_slot() {
        let pid = 4004;
        let _task = install_current_task(pid);
        let mut node = Box::new(TestRobustNode {
            next: 0,
            futex: AtomicU32::new(pid as u32),
        });
        let mut head = Box::new(RobustListHead {
            list_next: &mut *node as *mut TestRobustNode as u64,
            futex_offset: offset_of!(TestRobustNode, futex) as i64,
            list_op_pending: 0,
        });
        node.next = &mut *head as *mut RobustListHead as u64;

        assert_eq!(
            unsafe { sys_set_robust_list(&mut *head as *mut RobustListHead as u64, 24) },
            0
        );
        unsafe { exit_robust_list(pid) };
        assert_eq!(
            node.futex.load(Ordering::Acquire),
            FUTEX_OWNER_DIED,
            "dead-owner futexes must be marked OWNER_DIED on exit"
        );

        let mut robust_head = 1u64;
        let mut robust_len = 1u64;
        assert_eq!(
            unsafe { sys_get_robust_list_for_pid(pid, &mut robust_head, &mut robust_len) },
            0
        );
        assert_eq!((robust_head, robust_len), (0, 0));
    }
}
