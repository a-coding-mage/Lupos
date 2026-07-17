//! linux-parity: partial
//! linux-source: vendor/linux/net/core/net_namespace.c
//! Network namespace stub — Milestone 28.  Full networking in M48–M53.

extern crate alloc;

use crate::fs::nsfs::{NS_TYPE_NET, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

#[repr(C)]
pub struct Net {
    pub ns: NsCommon,
    pub user_ns: *const UserNamespace,
}

unsafe impl Send for Net {}
unsafe impl Sync for Net {}

unsafe fn net_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut Net;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}
unsafe fn net_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut Net;
    if ns.is_null() {
        return;
    }
    if unsafe { (*ns).ns.put() } {
        if core::ptr::eq(ns, &raw const INIT_NET as *mut Net) {
            unsafe {
                (*ns)
                    .ns
                    .count
                    .store(usize::MAX / 2, core::sync::atomic::Ordering::Relaxed);
            }
            return;
        }
        unsafe {
            crate::net::device::unregister_net_namespace(ns as usize);
            crate::net::socket::unregister_net_namespace(ns as usize);
            drop(alloc::boxed::Box::from_raw(ns));
        }
    }
}
unsafe fn net_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const Net;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static NET_OPS: NsOperations = NsOperations {
    name: "net",
    ns_type: NS_TYPE_NET,
    get: net_get,
    put: net_put,
    owner: net_owner,
};

pub static INIT_NET: Net = Net {
    ns: NsCommon::sticky(&NET_OPS as *const _, PROC_DYNAMIC_FIRST + 5),
    user_ns: &INIT_USER_NS,
};

/// Stable key for the calling task's network namespace.  The init namespace
/// uses zero so global driver registration remains independent of the static
/// object's link address; dynamically allocated namespaces use their pointer.
pub fn current_net_namespace_key() -> usize {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return 0;
    }
    let nsproxy = unsafe { (*task).m28_nsproxy.nsproxy };
    if nsproxy.is_null() {
        return 0;
    }
    let net = unsafe { (*nsproxy).net_ns };
    if net.is_null() || core::ptr::eq(net, &raw const INIT_NET as *mut Net) {
        0
    } else {
        net as usize
    }
}

pub fn copy_net_ns(_old: *const Net, user_ns: *const UserNamespace) -> Result<*mut Net, i32> {
    let b = alloc::boxed::Box::new(Net {
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &NET_OPS as *const _,
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
