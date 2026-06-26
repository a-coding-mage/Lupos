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

use crate::fs::nsfs::{NS_TYPE_PID, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::task::TaskStruct;
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

/// Maximum number of nested PID namespaces.  Linux: `MAX_PID_NS_LEVEL = 32`.
pub const MAX_PID_NS_LEVEL: u32 = 32;

#[repr(C)]
pub struct PidNamespace {
    pub ns: NsCommon,
    pub level: u32,
    pub _pad: u32,
    pub parent: *const PidNamespace,
    pub user_ns: *const UserNamespace,
    pub child_reaper: *mut TaskStruct,
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
        unsafe {
            drop(alloc::boxed::Box::from_raw(ns));
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
};

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
}
