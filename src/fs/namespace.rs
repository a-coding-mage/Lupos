//! linux-parity: partial
//! linux-source: vendor/linux/fs/namespace.c
//! Mount namespace — Milestones 28 (skeleton) + 39 (real `Mount` plumbing).
//!
//! Linux nests every mount inside an `mnt_namespace`.  M28 gave us the
//! refcount + `NsCommon` shell with a `*mut c_void` root placeholder; M39
//! retypes that root to a real `*mut Mount` so the VFS mount tree is
//! reachable through the namespace.

extern crate alloc;

use crate::fs::mount::Mount;
use crate::fs::nsfs::{NS_TYPE_MNT, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::user_namespace::{INIT_USER_NS, UserNamespace};

#[repr(C)]
pub struct MntNamespace {
    pub ns: NsCommon,
    /// Root mount of this namespace.  Held as a raw pointer for layout
    /// stability; `Arc<Mount>` reference-management is delegated to the
    /// helpers in `crate::fs::mount`.
    pub root: *mut Mount,
    pub user_ns: *const UserNamespace,
}

unsafe impl Send for MntNamespace {}
unsafe impl Sync for MntNamespace {}

unsafe fn mnt_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut MntNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}
unsafe fn mnt_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut MntNamespace;
    if ns.is_null() {
        return;
    }
    if unsafe { (*ns).ns.put() } {
        if core::ptr::eq(ns, &raw const INIT_MNT_NS as *mut MntNamespace) {
            unsafe {
                (*ns)
                    .ns
                    .count
                    .store(usize::MAX / 2, core::sync::atomic::Ordering::Relaxed);
            }
            return;
        }
        unsafe {
            crate::fs::mount::unregister_mount_namespace(ns);
            if !(*ns).user_ns.is_null() {
                (*(*ns).user_ns).ns.put();
            }
            drop(alloc::boxed::Box::from_raw(ns));
        }
    }
}
unsafe fn mnt_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    let ns = ns as *const MntNamespace;
    if ns.is_null() {
        core::ptr::null()
    } else {
        unsafe { (*ns).user_ns as *const _ }
    }
}

pub static MNT_OPS: NsOperations = NsOperations {
    name: "mnt",
    ns_type: NS_TYPE_MNT,
    get: mnt_get,
    put: mnt_put,
    owner: mnt_owner,
};

pub static INIT_MNT_NS: MntNamespace = MntNamespace {
    ns: NsCommon::sticky(&MNT_OPS as *const _, PROC_DYNAMIC_FIRST + 4),
    root: core::ptr::null_mut(),
    user_ns: &INIT_USER_NS,
};

pub fn copy_mnt_ns(
    old: *const MntNamespace,
    user_ns: *const UserNamespace,
) -> Result<*mut MntNamespace, i32> {
    let b = alloc::boxed::Box::new(MntNamespace {
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &MNT_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
        root: core::ptr::null_mut(),
        user_ns,
    });
    if !user_ns.is_null() {
        unsafe {
            (*user_ns).ns.get();
        }
    }
    let ns = alloc::boxed::Box::into_raw(b);
    let root = crate::fs::mount::register_mount_namespace(ns, old);
    unsafe {
        (*ns).root = root;
    }
    Ok(ns)
}
