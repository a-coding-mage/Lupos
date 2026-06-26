//! linux-parity: partial
//! linux-source: vendor/linux/ipc/namespace.c
//! IPC namespace stub — Milestone 28.
//!
//! Holds the SysV IPC objects (msg queues, semaphores, shared memory).
//! Full IPC arrives in M52; M28 ships only the namespace shell.

extern crate alloc;

use crate::fs::nsfs::{NS_TYPE_IPC, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

#[repr(C)]
pub struct IpcNamespace {
    pub ns: NsCommon,
    pub user_ns: *const UserNamespace,
}

unsafe impl Send for IpcNamespace {}
unsafe impl Sync for IpcNamespace {}

unsafe fn ipc_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut IpcNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}

unsafe fn ipc_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut IpcNamespace;
    if ns.is_null() {
        return;
    }
    if unsafe { (*ns).ns.put() } {
        if core::ptr::eq(ns, &raw const INIT_IPC_NS as *mut IpcNamespace) {
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

unsafe fn ipc_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const IpcNamespace;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static IPC_OPS: NsOperations = NsOperations {
    name: "ipc",
    ns_type: NS_TYPE_IPC,
    get: ipc_get,
    put: ipc_put,
    owner: ipc_owner,
};

pub static INIT_IPC_NS: IpcNamespace = IpcNamespace {
    ns: NsCommon::sticky(&IPC_OPS as *const _, PROC_DYNAMIC_FIRST + 3),
    user_ns: &INIT_USER_NS,
};

pub fn copy_ipc_ns(
    _old: *const IpcNamespace,
    user_ns: *const UserNamespace,
) -> Result<*mut IpcNamespace, i32> {
    let b = alloc::boxed::Box::new(IpcNamespace {
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &IPC_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
        user_ns,
    });
    if !user_ns.is_null() {
        unsafe {
            (*user_ns).ns.get();
        }
    }
    Ok(alloc::boxed::Box::into_raw(b))
}
