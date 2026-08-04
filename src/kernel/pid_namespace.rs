//! linux-parity: partial
//! linux-source: vendor/linux/kernel/pid_namespace.c
//! test-origin: linux:vendor/linux/kernel/pid_namespace.c
//! PID namespace — Milestone 28.
//!
//! Wraps the existing `INIT_PID_NS` from `kernel::pid` with an `NsCommon`
//! header so the namespace machinery can dispatch refcount/inum/ops on it.
//! Multi-level PID namespaces (Linux's `level` field, `child_reaper`,
//! per-ns PID allocators) are scoped here but full multi-level allocation
//! is gated behind M28 because the existing `KPid::numbers[]` array
//! already supports a single level today.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

use crate::fs::nsfs::{NS_TYPE_PID, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::task::TaskStruct;
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

/// Maximum number of nested PID namespaces.  Linux: `MAX_PID_NS_LEVEL = 32`.
pub const MAX_PID_NS_LEVEL: u32 = 32;
const PID_MAX_LIMIT: u32 = 32_768;

#[repr(C)]
pub struct PidNamespace {
    pub ns: NsCommon,
    pub level: u32,
    pub _pad: u32,
    pub parent: *const PidNamespace,
    pub user_ns: *const UserNamespace,
    pub child_reaper: *mut TaskStruct,
    /// Next PID for this namespace's `alloc_pid()` path.
    ///
    /// Linux uses an IDR protected by `pidmap_lock`.  The compact Lupos task
    /// registry has no IDR yet, so a monotonic per-namespace cursor provides
    /// the same visible-PID ordering and, importantly, starts each nested
    /// namespace at PID 1.
    pub next_pid: AtomicU32,
}

unsafe impl Send for PidNamespace {}
unsafe impl Sync for PidNamespace {}

unsafe fn pid_ns_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut PidNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}

unsafe fn pid_ns_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut PidNamespace;
    if ns.is_null() {
        return;
    }
    let last = unsafe { (*ns).ns.put() };
    if last {
        if core::ptr::eq(ns, &raw const INIT_PID_NS_M28 as *mut PidNamespace) {
            unsafe {
                (*ns)
                    .ns
                    .count
                    .store(usize::MAX / 2, core::sync::atomic::Ordering::Relaxed);
            }
            return;
        }
        let parent = unsafe { (*ns).parent };
        let user_ns = unsafe { (*ns).user_ns };
        unsafe {
            drop(alloc::boxed::Box::from_raw(ns));
        }
        if !parent.is_null() {
            pid_ns_put(parent as *mut core::ffi::c_void);
        }
        if !user_ns.is_null() {
            crate::kernel::user_namespace::put_user_ns(user_ns);
        }
    }
}

unsafe fn pid_ns_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const PidNamespace;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static PID_NS_OPS: NsOperations = NsOperations {
    name: "pid",
    ns_type: NS_TYPE_PID,
    get: pid_ns_get,
    put: pid_ns_put,
    owner: pid_ns_owner,
};

/// M28 PID namespace singleton.  This is the namespace that wraps the
/// existing M22 `INIT_PID_NS` PID-allocator with the `NsCommon` header.
pub static INIT_PID_NS_M28: PidNamespace = PidNamespace {
    ns: NsCommon::sticky(&PID_NS_OPS as *const _, PROC_DYNAMIC_FIRST + 2),
    level: 0,
    _pad: 0,
    parent: core::ptr::null(),
    user_ns: &INIT_USER_NS,
    child_reaper: core::ptr::null_mut(),
    next_pid: AtomicU32::new(1),
};

struct TaskPidInfo {
    task: *mut TaskStruct,
    ns: *mut PidNamespace,
    pid: i32,
    tgid: i32,
}

// Raw task pointers are stable while the task is present in the heap tracker;
// the entry is removed by cleanup_task_shared_state before that allocation is
// reclaimed.
unsafe impl Send for TaskPidInfo {}

static TASK_PID_INFOS: Mutex<Vec<TaskPidInfo>> = Mutex::new(Vec::new());

#[inline]
fn is_init_pid_ns(ns: *const PidNamespace) -> bool {
    core::ptr::eq(ns, &raw const INIT_PID_NS_M28)
}

fn task_info(task: *mut TaskStruct) -> Option<(PidNamespacePtr, i32, i32)> {
    let infos = TASK_PID_INFOS.lock();
    infos
        .iter()
        .find(|info| info.task == task)
        .map(|info| (PidNamespacePtr(info.ns), info.pid, info.tgid))
}

#[derive(Clone, Copy)]
struct PidNamespacePtr(*mut PidNamespace);

/// Return the PID namespace in which `task` executes.
///
/// This is the Lupos equivalent of Linux `task_active_pid_ns()`.  A task which
/// has not needed a nested namespace has no side-table entry and falls back to
/// the namespace recorded in its nsproxy fields.
pub fn task_active_pid_ns(task: *mut TaskStruct) -> *mut PidNamespace {
    if task.is_null() {
        return &raw const INIT_PID_NS_M28 as *mut _;
    }
    if let Some((ns, _, _)) = task_info(task) {
        return ns.0;
    }
    let ns = unsafe { (*task).m28_nsproxy.thread_pid_ns_for_children as *mut PidNamespace };
    if ns.is_null() {
        &raw const INIT_PID_NS_M28 as *mut _
    } else {
        ns
    }
}

/// Return `task`'s PID as observed from `ns`, mirroring Linux
/// `task_pid_nr_ns()` for the namespace levels represented by Lupos.
pub fn task_pid_nr_ns(task: *mut TaskStruct, ns: *mut PidNamespace) -> i32 {
    if task.is_null() {
        return 0;
    }
    let ns = if ns.is_null() {
        task_active_pid_ns(task)
    } else {
        ns
    };
    if is_init_pid_ns(ns) {
        return unsafe { (*task).pid };
    }
    task_info(task)
        .filter(|(task_ns, _, _)| task_ns.0 == ns)
        .map(|(_, pid, _)| pid)
        .unwrap_or(0)
}

/// Return `task`'s thread-group ID as observed from `ns`.
pub fn task_tgid_nr_ns(task: *mut TaskStruct, ns: *mut PidNamespace) -> i32 {
    if task.is_null() {
        return 0;
    }
    let ns = if ns.is_null() {
        task_active_pid_ns(task)
    } else {
        ns
    };
    if is_init_pid_ns(ns) {
        return unsafe { (*task).tgid };
    }
    task_info(task)
        .filter(|(task_ns, _, _)| task_ns.0 == ns)
        .map(|(_, _, tgid)| tgid)
        .unwrap_or(0)
}

pub fn task_pid_vnr(task: *mut TaskStruct) -> i32 {
    let ns = task_active_pid_ns(task);
    task_pid_nr_ns(task, ns)
}

pub fn task_tgid_vnr(task: *mut TaskStruct) -> i32 {
    let ns = task_active_pid_ns(task);
    task_tgid_nr_ns(task, ns)
}

/// Linux `pid_alive()` (`include/linux/pid.h`): a task structure is stale once
/// `release_task()` has detached its `thread_pid`, and pointers inside it must
/// not be dereferenced after that.
pub fn pid_alive(task: *mut TaskStruct) -> bool {
    !task.is_null() && !unsafe { (*task).m26.thread_pid }.is_null()
}

/// Linux `task_ppid_nr_ns()` (`include/linux/pid.h`).
///
/// The reported parent is the parent's **thread-group** ID, never the raw TID
/// of the thread that happened to call `fork()`.  A process forked from a
/// non-leader thread must still report the parent process, otherwise `ps`,
/// `pkill -P` and every process-tree walker see a parent that does not exist.
pub fn task_ppid_nr_ns(task: *mut TaskStruct, ns: *mut PidNamespace) -> i32 {
    if !pid_alive(task) {
        return 0;
    }
    let parent = unsafe { (*task).m26.real_parent };
    task_tgid_nr_ns(parent, ns)
}

pub fn task_ppid_vnr(task: *mut TaskStruct) -> i32 {
    let ns = task_active_pid_ns(task);
    task_ppid_nr_ns(task, ns)
}

fn alloc_visible_pid(ns: *mut PidNamespace) -> Result<i32, i32> {
    let nr = unsafe { (*ns).next_pid.fetch_add(1, Ordering::AcqRel) };
    if nr == 0 || nr >= PID_MAX_LIMIT {
        return Err(-11); // EAGAIN, matching alloc_pid() exhaustion.
    }
    Ok(nr as i32)
}

/// Allocate and publish the namespace-visible PID for a newly copied task.
///
/// Linux performs this in `copy_process()` after `copy_namespaces()`, using
/// `p->nsproxy->pid_ns_for_children`.  The global init PID remains in the
/// existing M22 allocator; only nested namespaces need a second visible ID.
pub unsafe fn register_child_pid(
    parent: *mut TaskStruct,
    child: *mut TaskStruct,
    clone_thread: bool,
) -> Result<(), i32> {
    if child.is_null() {
        return Err(-22);
    }
    let ns = unsafe { (*child).m28_nsproxy.thread_pid_ns_for_children as *mut PidNamespace };
    let ns = if ns.is_null() {
        &raw const INIT_PID_NS_M28 as *mut _
    } else {
        ns
    };
    if is_init_pid_ns(ns) {
        return Ok(());
    }
    let pid = alloc_visible_pid(ns)?;
    let tgid = if clone_thread {
        task_tgid_vnr(parent)
    } else {
        pid
    };
    let mut infos = TASK_PID_INFOS.lock();
    if infos.iter().any(|info| info.task == child) {
        return Err(-22);
    }
    infos.push(TaskPidInfo {
        task: child,
        ns,
        pid,
        tgid,
    });
    drop(infos);
    if pid == 1 {
        unsafe {
            (*ns).child_reaper = child;
        }
    }
    Ok(())
}

/// Remove the namespace-visible PID owned by a task during Linux's
/// `release_task()` teardown.
pub unsafe fn unregister_task_pid(task: *mut TaskStruct) {
    if task.is_null() {
        return;
    }
    let mut infos = TASK_PID_INFOS.lock();
    if let Some(index) = infos.iter().position(|info| info.task == task) {
        let info = infos.swap_remove(index);
        if !info.ns.is_null() && unsafe { (*info.ns).child_reaper == task } {
            unsafe {
                (*info.ns).child_reaper = core::ptr::null_mut();
            }
        }
    }
}

/// Find a task by PID in the caller's active namespace, equivalent to Linux
/// `find_task_by_pid_ns()` for the task registry currently available in Lupos.
pub fn find_task_by_pid_ns(pid: i32, ns: *mut PidNamespace) -> *mut TaskStruct {
    if pid <= 0 || ns.is_null() {
        return core::ptr::null_mut();
    }
    let mut found: *mut TaskStruct = core::ptr::null_mut();
    let current = unsafe { crate::kernel::sched::get_current() };
    if task_pid_nr_ns(current, ns) == pid {
        return current;
    }
    crate::kernel::fork::for_each_heap_task(|task| {
        if found.is_null() && task_pid_nr_ns(task, ns) == pid {
            found = task;
        }
    });
    if found.is_null() {
        crate::kernel::sched::for_each_pool_task(|task| {
            if found.is_null() && task_pid_nr_ns(task, ns) == pid {
                found = task;
            }
        });
    }
    found
}

/// Allocate a fresh nested PID namespace.
pub fn copy_pid_ns(
    parent: *const PidNamespace,
    user_ns: *const UserNamespace,
) -> Result<*mut PidNamespace, i32> {
    let parent = if parent.is_null() {
        &INIT_PID_NS_M28 as *const _
    } else {
        parent
    };
    let level = unsafe { (*parent).level + 1 };
    if level >= MAX_PID_NS_LEVEL {
        return Err(-22); // EINVAL — nesting limit
    }
    let b = alloc::boxed::Box::new(PidNamespace {
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &PID_NS_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
        level,
        _pad: 0,
        parent,
        user_ns,
        child_reaper: core::ptr::null_mut(),
        next_pid: AtomicU32::new(1),
    });
    unsafe {
        (*parent).ns.get();
    }
    if !user_ns.is_null() {
        unsafe {
            (*user_ns).ns.get();
        }
    }
    Ok(alloc::boxed::Box::into_raw(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_pid_ns_is_level_zero() {
        assert_eq!(INIT_PID_NS_M28.level, 0);
    }

    #[test]
    fn copy_pid_ns_increments_level() {
        let p = copy_pid_ns(&INIT_PID_NS_M28, &INIT_USER_NS).unwrap();
        unsafe {
            assert_eq!((*p).level, 1);
            pid_ns_put(p as *mut _);
        }
    }

    #[test]
    fn nested_pid_allocation_starts_at_one() {
        let p = copy_pid_ns(&INIT_PID_NS_M28, &INIT_USER_NS).unwrap();
        assert_eq!(alloc_visible_pid(p), Ok(1));
        assert_eq!(alloc_visible_pid(p), Ok(2));
        unsafe {
            pid_ns_put(p as *mut _);
        }
    }
}
