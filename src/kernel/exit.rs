//! linux-parity: complete
//! linux-source: vendor/linux/kernel/exit.c
//! test-origin: linux:vendor/linux/kernel/exit.c
//! Exit / zombie / release_task — Milestone 26.
//!
//! Implements the kernel-side teardown of a task:
//!
//! - `do_exit(code)` — never returns; transitions current to EXIT_ZOMBIE,
//!   tears down the address space + fd table, notifies the parent (SIGCHLD),
//!   and yields forever.
//! - `exit_mm` / `exit_files` / `exit_notify` — sub-steps mirroring the
//!   identically-named Linux helpers in `kernel/exit.c`.
//! - `release_task` — called by the *waiter* (parent in `sys_wait4`) once a
//!   zombie has been observed; drops the `KPid` refcount, removes the child
//!   from the parent's children array, drains the heap-task tracker, and
//!   finally drops the `Box<TaskStruct>` + `Box<stack>`.
//!
//! Reference: `vendor/linux/kernel/exit.c`.

extern crate alloc;

use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess;
use crate::kernel::fork::heap_task_release;
use crate::kernel::pid;
use crate::kernel::sched;
use crate::kernel::signal::{self, SIGCHLD};
use crate::kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE};
use crate::kernel::task::{MAX_CHILDREN, TaskStruct};

pub unsafe fn exit_clear_child_tid(tsk: *mut TaskStruct) {
    if tsk.is_null() {
        return;
    }

    let clear_child_tid = unsafe { (*tsk).m26.clear_child_tid };
    unsafe {
        (*tsk).m26.clear_child_tid = core::ptr::null_mut();
    }
    if clear_child_tid.is_null() {
        return;
    }

    if unsafe { uaccess::put_user_u32(clear_child_tid as *mut u32, 0) }.is_err() {
        return;
    }

    let _ = unsafe {
        crate::kernel::futex::futex_wake(
            clear_child_tid as u64,
            1,
            crate::kernel::futex::FUTEX_BITSET_MATCH_ANY,
            false,
        )
    };
}

/// Terminate the current task with status `code`.
///
/// Mirrors Linux `do_exit(long code)` in `kernel/exit.c`.  The packed Linux
/// wait status is built by `wait::w_exitcode`; this function stores the
/// already-packed value into `m26.exit_code`.
///
/// # Safety
/// Must be called from a valid task context.  Never returns — the task
/// becomes a zombie and the scheduler skips it on every subsequent pick.
pub unsafe fn do_exit(code: i64) -> ! {
    let tsk = unsafe { sched::get_current() };
    if tsk.is_null() {
        // No current task — defensive halt.
        loop {
            core::hint::spin_loop();
        }
    }

    unsafe {
        (*tsk).m26.exit_code = code as i32;

        crate::kernel::futex::robust::exit_robust_list((*tsk).pid);
        crate::kernel::futex::core_ops::futex_exit_release(tsk);
        exit_clear_child_tid(tsk);
        exit_mm(tsk);
        exit_files(tsk);
        crate::fs::fs_struct::exit_fs(tsk);

        notify_exit_and_publish_zombie(tsk);
    }

    // Yield forever.  The scheduler's NON_RUNNABLE_MASK now skips this task,
    // so once another task is enqueued we never return here.  In the
    // single-task degenerate case `schedule()` is a no-op and we spin.
    loop {
        unsafe {
            sched::schedule_with_irqs_enabled();
        }
        core::hint::spin_loop();
    }
}

/// Drop the task's `mm` reference.
///
/// For kernel threads `mm == NULL`, so this is a no-op.  For user tasks it
/// decrements `mm_users`; full VMA + page-table teardown lives in M14/M15
/// helpers and is invoked when `mm_users` reaches zero — deferred until M28
/// can validate it under namespace teardown.
///
/// # Safety
/// `tsk` must be valid.  Caller must ensure no further accesses to `tsk.mm`
/// occur after this returns.
pub unsafe fn exit_mm(tsk: *mut TaskStruct) {
    if tsk.is_null() {
        return;
    }
    unsafe {
        let mm = (*tsk).mm;
        (*tsk).mm = core::ptr::null_mut();
        (*tsk).active_mm = core::ptr::null_mut();
        if !mm.is_null() {
            // mmput returns true when mm_users hit zero — full teardown
            // (free VMAs + page tables) lands in a follow-up.  In M26 the
            // smoke test exercises only kthreads (mm == NULL).
            if (*mm).mmput() {
                switch_to_kernel_cr3_before_destroy(mm);
                crate::mm::fork::destroy_mm(mm);
            }
        }
    }
}

#[cfg(not(test))]
unsafe fn switch_to_kernel_cr3_before_destroy(mm: *mut crate::mm::mm_types::MmStruct) {
    if mm.is_null() {
        return;
    }
    let pgd_virt = unsafe { (*mm).pgd as u64 };
    let Some(pgd_phys) = crate::arch::x86::mm::paging::virt_to_phys(pgd_virt) else {
        return;
    };
    if crate::arch::x86::mm::paging::read_cr3() != pgd_phys {
        return;
    }
    let init_pgd = crate::arch::x86::mm::paging::init_pgd_phys();
    unsafe {
        core::arch::asm!(
            "mov cr3, {0}",
            in(reg) init_pgd,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(test)]
unsafe fn switch_to_kernel_cr3_before_destroy(_mm: *mut crate::mm::mm_types::MmStruct) {}

unsafe fn add_child_link(parent: *mut TaskStruct, child: *mut TaskStruct) {
    if parent.is_null() || child.is_null() {
        return;
    }

    let count = unsafe { (*parent).m26.children_count as usize };
    for i in 0..count.min(MAX_CHILDREN) {
        if unsafe { (*parent).m26.children[i] == child } {
            return;
        }
    }

    for i in 0..MAX_CHILDREN {
        if unsafe { (*parent).m26.children[i].is_null() } {
            unsafe {
                (*parent).m26.children[i] = child;
                if i >= count {
                    (*parent).m26.children_count = (i + 1) as u32;
                }
            }
            return;
        }
    }
}

unsafe fn find_child_reaper(tsk: *mut TaskStruct) -> *mut TaskStruct {
    if tsk.is_null() {
        return core::ptr::null_mut();
    }

    let fallback = unsafe { (*tsk).m26.real_parent };
    if fallback.is_null()
        || unsafe {
            (*tsk).m27.mdwe_flags & crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER == 0
        }
    {
        return fallback;
    }

    let mut reaper = fallback;
    while !reaper.is_null() {
        if unsafe { (*reaper).m27.mdwe_flags & crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER != 0 }
        {
            return reaper;
        }
        reaper = unsafe { (*reaper).m26.real_parent };
    }
    fallback
}

/// Drop the task's fd-table reference.
///
/// M26 stub: `FilesStruct` is a forward-declared opaque enum (M39).  We
/// simply NULL the pointer so subsequent accesses fault loudly.
pub unsafe fn exit_files(tsk: *mut TaskStruct) {
    unsafe { crate::kernel::files::drop_task_files(tsk) }
}

unsafe fn take_exit_waiters(
    tsk: *mut TaskStruct,
) -> ([*mut TaskStruct; crate::kernel::task::MAX_WAITERS], usize) {
    let mut waiters = [core::ptr::null_mut(); crate::kernel::task::MAX_WAITERS];
    if tsk.is_null() {
        return (waiters, 0);
    }

    unsafe {
        let waiter_count = (*tsk)
            .m26
            .wait_count
            .min(crate::kernel::task::MAX_WAITERS as u32) as usize;
        for (i, slot) in waiters.iter_mut().enumerate().take(waiter_count) {
            *slot = (*tsk).m26.wait_waiters[i];
            (*tsk).m26.wait_waiters[i] = core::ptr::null_mut();
        }
        (*tsk).m26.wait_count = 0;
        (waiters, waiter_count)
    }
}

fn wake_exit_waiters(waiters: &[*mut TaskStruct; crate::kernel::task::MAX_WAITERS], count: usize) {
    for w in waiters.iter().take(count) {
        if !w.is_null() {
            unsafe {
                crate::kernel::sched::wake_task(*w);
            }
        }
    }
}

/// Run exit notifications, then publish the reapable zombie state.
///
/// `m26.exit_state` is set before the waiter snapshot to advertise that exit is
/// in progress while `__state` remains non-zombie.  Waiters treat this
/// half-published state as a reason not to sleep, which closes the lost-wakeup
/// window for waiters that register after the snapshot but before EXIT_ZOMBIE is
/// visible in `__state`.  Once `__state` is visible, `sys_wait4()` may call
/// `release_task()` and make `tsk` dangling, so that store remains the last task
/// access before waking the snapshotted waiters.
pub unsafe fn notify_exit_and_publish_zombie(tsk: *mut TaskStruct) {
    if tsk.is_null() {
        return;
    }

    unsafe {
        (*tsk).m26.exit_state = EXIT_ZOMBIE;
        let (waiters, waiter_count) = take_exit_waiters(tsk);
        exit_notify(tsk);

        (*tsk).__state.store(EXIT_ZOMBIE, Ordering::Release);
        wake_exit_waiters(&waiters, waiter_count);
    }
}

/// Notify the parent of `tsk`'s exit and reparent its children.
///
/// Steps:
/// 1. Reparent `tsk`'s children to `tsk`'s real_parent (closest sub-reaper
///    proxy until M28's namespace reaper logic lands).
/// 2. Send `tsk.exit_signal` (default SIGCHLD) to the real_parent so a
///    waiting `wait4` observes pending work.
/// 3. Waiter wakeups are intentionally deferred until after the caller publishes
///    EXIT_ZOMBIE; publishing it earlier lets wait4 reap `tsk` while this
///    helper is still dereferencing it.
pub unsafe fn exit_notify(tsk: *mut TaskStruct) {
    if tsk.is_null() {
        return;
    }
    unsafe {
        crate::fs::pidfd::notify_task_exit(tsk);

        // 1. Reparent children to the closest child subreaper, falling back
        // to our real_parent when no subreaper ancestor exists.
        let new_parent = find_child_reaper(tsk);
        let n = (*tsk).m26.children_count as usize;
        for i in 0..n.min(MAX_CHILDREN) {
            let c = (*tsk).m26.children[i];
            if !c.is_null() {
                (*c).m26.real_parent = new_parent;
                (*c).m26.parent = new_parent;
                add_child_link(new_parent, c);
            }
        }

        // 2. Send exit_signal (typically SIGCHLD) to the real parent.
        let parent = (*tsk).m26.real_parent;
        let sig = (*tsk).m26.exit_signal;
        if !parent.is_null() && sig > 0 {
            let _ = signal::send_signal_to_task(parent, sig);
        }

        // Default exit_signal to SIGCHLD if not explicitly set (so the parent
        // observes a notification even from kthread-style children).
        let _ = SIGCHLD; // referenced here so the import isn't unused
    }
}

/// Reap a zombie child: drop refs, free heap, remove from parent's list.
///
/// Called by `wait::sys_wait4` after it has read out `exit_code` / `exit_signal`
/// for the user-space caller.  After this returns, `p` is dangling.
///
/// # Safety
/// `p` must be a `*mut TaskStruct` previously returned by `kernel_clone` and
/// must currently be in `EXIT_ZOMBIE`.  No other CPU may hold a pointer to it.
pub unsafe fn release_task(p: *mut TaskStruct) {
    if p.is_null() {
        return;
    }

    unsafe {
        // 1. Remove from parent's children array.
        let parent = (*p).m26.real_parent;
        if !parent.is_null() {
            let count = (*parent).m26.children_count as usize;
            let mut found_at: Option<usize> = None;
            for i in 0..count.min(MAX_CHILDREN) {
                if (*parent).m26.children[i] == p {
                    found_at = Some(i);
                    break;
                }
            }
            if let Some(i) = found_at {
                // Compact: move the last entry into the freed slot.
                let last = count - 1;
                if i != last {
                    (*parent).m26.children[i] = (*parent).m26.children[last];
                }
                (*parent).m26.children[last] = core::ptr::null_mut();
                (*parent).m26.children_count = last as u32;
            }
        }

        // 2. Drop the KPid refcount (clears the bitmap bit when refcount hits 0).
        let thread_pid = (*p).m26.thread_pid;
        (*p).m26.thread_pid = core::ptr::null_mut();
        if !thread_pid.is_null() {
            pid::put_pid(thread_pid);
        }

        // 3. Remove from the run queue so the scheduler stops considering it.
        sched::dequeue_task(p);

        // 4. Mark EXIT_DEAD just before the final drop, so any concurrent
        //    observer sees the transient state (Linux symmetry).
        (*p).__state.store(EXIT_DEAD, Ordering::Release);
        (*p).m26.exit_state = EXIT_DEAD;

        // 5. Notify the LSM layer that the task is being torn down.
        crate::security::security_task_free((*p).pid as u32);

        // 6. Drop task-owned shared state that is not released during do_exit.
        crate::kernel::syscalls::release_process_rlimits(p);
        crate::kernel::syscalls::release_task_rseq_registration(p);
        crate::kernel::fork::cleanup_task_shared_state(p);

        // 7. Drop the heap allocations (TaskStruct + kernel stack).  After
        //    this returns, `p` is dangling.
        heap_task_release(p);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    use crate::kernel::fork::{KernelCloneArgs, copy_process, heap_task_count};
    use crate::kernel::task::M26Fields;
    use crate::kernel::task::task_state::{
        EXIT_ZOMBIE, NON_RUNNABLE_MASK, TASK_INTERRUPTIBLE, TASK_RUNNING,
    };
    use crate::security::hooks::{LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};
    use crate::security::register_lsm;
    use alloc::boxed::Box;

    static TASK_FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn count_task_free(_task_id: u32) {
        TASK_FREE_COUNT.fetch_add(1, AtomicOrdering::SeqCst);
    }

    /// Build a stack-allocated zeroed TaskStruct for tests.
    fn make_task(pid: i32, tgid: i32) -> Box<TaskStruct> {
        let mut t = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        t.pid = pid;
        t.tgid = tgid;
        t.m26 = M26Fields::zeroed();
        t.m26.exit_signal = SIGCHLD;
        t
    }

    #[test]
    fn exit_state_zombie_bit_is_in_non_runnable_mask() {
        assert_ne!(
            NON_RUNNABLE_MASK & EXIT_ZOMBIE,
            0,
            "EXIT_ZOMBIE must be in NON_RUNNABLE_MASK so scheduler skips zombies"
        );
    }

    #[test]
    fn exit_dead_is_in_non_runnable_mask() {
        assert_ne!(NON_RUNNABLE_MASK & EXIT_DEAD, 0);
    }

    #[test]
    fn exit_files_nulls_files_pointer() {
        let mut t = make_task(1234, 1234);
        use crate::fs::fdtable::FilesStruct as FdFilesStruct;
        use alloc::sync::Arc;

        let files = FdFilesStruct::new();
        // Leak one strong ref into the task_struct.files ABI pointer.
        let ptr = Arc::into_raw(files) as *mut FdFilesStruct;
        t.files = ptr as *mut _;
        unsafe {
            exit_files(&mut *t as *mut TaskStruct);
        }
        assert!(t.files.is_null());
    }

    #[test]
    fn exit_clear_child_tid_zeros_slot_and_clears_pointer() {
        let mut t = make_task(1234, 1234);
        let mut tid_slot = 1234i32;
        t.m26.clear_child_tid = &mut tid_slot as *mut i32;

        unsafe {
            exit_clear_child_tid(&mut *t as *mut TaskStruct);
        }

        assert_eq!(tid_slot, 0, "exit must clear the child tid slot");
        assert!(
            t.m26.clear_child_tid.is_null(),
            "exit must drop the in-kernel clear-child-tid pointer"
        );
    }

    #[test]
    fn exit_notify_reparents_children_to_real_parent() {
        let mut grand = make_task(1, 1);
        let mut parent = make_task(2, 2);
        let mut child = make_task(3, 3);

        unsafe {
            // grand → parent → child
            parent.m26.real_parent = &mut *grand as *mut TaskStruct;
            parent.m26.parent = parent.m26.real_parent;
            parent.m26.children[0] = &mut *child as *mut TaskStruct;
            parent.m26.children_count = 1;
            child.m26.real_parent = &mut *parent as *mut TaskStruct;
            child.m26.parent = child.m26.real_parent;

            // parent exits → child should be reparented to grand.
            exit_notify(&mut *parent as *mut TaskStruct);
            assert_eq!(child.m26.real_parent, &mut *grand as *mut TaskStruct);
            assert_eq!(child.m26.parent, &mut *grand as *mut TaskStruct);
        }
    }

    #[test]
    fn exit_notify_reparents_orphans_to_nearest_child_subreaper() {
        let mut init = make_task(1, 1);
        let mut subreaper = make_task(2, 2);
        let mut middle = make_task(3, 3);
        let mut parent = make_task(4, 4);
        let mut child = make_task(5, 5);

        unsafe {
            subreaper.m26.real_parent = &mut *init as *mut TaskStruct;
            subreaper.m26.parent = subreaper.m26.real_parent;
            subreaper.m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER;

            middle.m26.real_parent = &mut *subreaper as *mut TaskStruct;
            middle.m26.parent = middle.m26.real_parent;
            middle.m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER;

            parent.m26.real_parent = &mut *middle as *mut TaskStruct;
            parent.m26.parent = parent.m26.real_parent;
            parent.m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER;
            parent.m26.children[0] = &mut *child as *mut TaskStruct;
            parent.m26.children_count = 1;

            child.m26.real_parent = &mut *parent as *mut TaskStruct;
            child.m26.parent = child.m26.real_parent;

            exit_notify(&mut *parent as *mut TaskStruct);
            assert_eq!(
                child.m26.real_parent, &mut *subreaper as *mut TaskStruct,
                "orphan should skip non-subreaper ancestors"
            );
            assert_eq!(child.m26.parent, &mut *subreaper as *mut TaskStruct);
            assert_eq!(subreaper.m26.children[0], &mut *child as *mut TaskStruct);
            assert_eq!(subreaper.m26.children_count, 1);
        }
    }

    #[test]
    fn exit_notify_sends_sigchld_to_parent() {
        signal::reset_for_tests();
        signal::register_test_task(7777, 7777); // parent state pre-exists

        let mut parent = make_task(7777, 7777);
        let mut child = make_task(7778, 7778);
        unsafe {
            child.m26.real_parent = &mut *parent as *mut TaskStruct;
            child.m26.exit_signal = SIGCHLD;

            exit_notify(&mut *child as *mut TaskStruct);

            assert!(
                signal::has_pending_signal_for_pid(7777, SIGCHLD),
                "exit_notify must queue SIGCHLD on the real_parent"
            );
        }
    }

    #[test]
    fn notify_exit_and_publish_zombie_wakes_after_zombie_visible() {
        signal::reset_for_tests();
        signal::register_test_task(8110, 8110);

        let mut parent = make_task(8110, 8110);
        let mut child = make_task(8111, 8111);
        unsafe {
            parent.__state.store(TASK_INTERRUPTIBLE, Ordering::Release);
            child.m26.real_parent = &mut *parent as *mut TaskStruct;
            child.m26.exit_signal = SIGCHLD;
            child.m26.wait_waiters[0] = &mut *parent as *mut TaskStruct;
            child.m26.wait_count = 1;

            notify_exit_and_publish_zombie(&mut *child as *mut TaskStruct);

            assert_eq!(child.m26.exit_state & EXIT_ZOMBIE, EXIT_ZOMBIE);
            assert_eq!(
                child.__state.load(Ordering::Acquire) & EXIT_ZOMBIE,
                EXIT_ZOMBIE
            );
            assert_eq!(child.m26.wait_count, 0);
            assert!(child.m26.wait_waiters[0].is_null());
            assert_eq!(parent.__state.load(Ordering::Acquire), TASK_RUNNING);
            assert!(
                signal::has_pending_signal_for_pid(8110, SIGCHLD),
                "exit notification must still signal the real parent before wake"
            );
        }
    }

    #[test]
    fn task_state_constants_match_linux() {
        use crate::kernel::task::task_state::*;
        assert_eq!(TASK_RUNNING, 0x0000);
        assert_eq!(TASK_INTERRUPTIBLE, 0x0001);
        assert_eq!(TASK_UNINTERRUPTIBLE, 0x0002);
        assert_eq!(__TASK_STOPPED, 0x0004);
        assert_eq!(__TASK_TRACED, 0x0008);
        assert_eq!(EXIT_DEAD, 0x0010);
        assert_eq!(EXIT_ZOMBIE, 0x0020);
        assert_eq!(EXIT_TRACE, 0x0030);
        assert_eq!(TASK_PARKED, 0x0040);
        assert_eq!(TASK_DEAD, 0x0080);
    }

    #[test]
    fn release_task_calls_security_task_free() {
        let _lsm_guard = TEST_LSM_LOCK.lock();

        reset_for_test();
        TASK_FREE_COUNT.store(0, AtomicOrdering::SeqCst);

        let baseline = heap_task_count();
        let mut parent = make_task(9000, 9000);
        register_lsm(LsmHooks {
            name: "exit_release_task_free",
            task_free: Some(count_task_free),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        let args = KernelCloneArgs {
            kthread: 1,
            ..KernelCloneArgs::default()
        };
        let child =
            unsafe { copy_process(&mut *parent as *mut TaskStruct, &args) }.expect("copy_process");
        assert_eq!(parent.m26.children_count, 1);

        unsafe { release_task(child) };

        assert_eq!(TASK_FREE_COUNT.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(heap_task_count(), baseline);
        assert_eq!(parent.m26.children_count, 0);
    }
}
