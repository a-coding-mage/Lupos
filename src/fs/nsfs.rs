//! linux-parity: partial
//! linux-source: vendor/linux/fs/nsfs.c
//! test-origin: linux:vendor/linux/fs/nsfs.c
//! `ns_common` — refcount, inum, and ops vtable shared by every namespace.
//!
//! Every Linux namespace embeds `struct ns_common` as its first field so
//! that generic code (e.g. `proc_ns_operations`, `setns()`) can dispatch
//! polymorphically.  The 32-bit `inum` is the inode number the kernel
//! exposes through `/proc/<pid>/ns/<type>` once VFS lands (M39).
//!
//! Reference: Linux `include/linux/ns_common.h`, `fs/proc/proc_ns.c`,
//! `kernel/nsproxy.c`.

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// First "dynamic" inode number issued by the proc-ns allocator.  Linux:
/// `PROC_DYNAMIC_FIRST` from `include/linux/proc_fs.h`.  Every namespace
/// inum lies in `[PROC_DYNAMIC_FIRST, u32::MAX]`.
pub const PROC_DYNAMIC_FIRST: u32 = 0xF000_0000;

/// Namespace type tags.  Linux: `enum proc_ns_type` with the values below.
pub const NS_TYPE_MNT: u8 = 0;
pub const NS_TYPE_CGROUP: u8 = 1;
pub const NS_TYPE_UTS: u8 = 2;
pub const NS_TYPE_IPC: u8 = 3;
pub const NS_TYPE_USER: u8 = 4;
pub const NS_TYPE_PID: u8 = 5;
pub const NS_TYPE_NET: u8 = 6;
pub const NS_TYPE_TIME: u8 = 7;

/// Vtable shared by every namespace flavour.  Mirrors Linux
/// `proc_ns_operations` from `fs/proc/proc_ns.c`.
#[repr(C)]
pub struct NsOperations {
    pub name: &'static str,
    pub ns_type: u8,
    /// Bump refcount.  `ns` points to the head of the namespace struct (which
    /// embeds `NsCommon`).
    pub get: unsafe fn(ns: *mut core::ffi::c_void),
    /// Drop refcount; deallocate when zero.
    pub put: unsafe fn(ns: *mut core::ffi::c_void),
    /// Owner user namespace pointer accessor.  Returns `*const c_void` to
    /// avoid the cyclic type dependency on `user_namespace::UserNamespace`.
    pub owner: unsafe fn(ns: *const core::ffi::c_void) -> *const core::ffi::c_void,
}

unsafe impl Sync for NsOperations {}

/// Common header embedded as the first field of every namespace struct.
#[repr(C)]
pub struct NsCommon {
    pub count: AtomicUsize,
    pub stashed: *mut core::ffi::c_void, // `*mut Dentry` once VFS lands.
    pub ops: *const NsOperations,
    pub inum: u32,
    pub _pad: u32,
}

unsafe impl Send for NsCommon {}
unsafe impl Sync for NsCommon {}

/// Process-wide counter used to issue fresh namespace inums.
///
/// Linux uses an IDA (small-vector ID allocator) — we use a monotonic
/// counter for M28 since every namespace lives forever in our cooperative
/// scheduler.
static NS_INUM_COUNTER: AtomicU32 = AtomicU32::new(PROC_DYNAMIC_FIRST);

/// Allocate a fresh proc-ns inum.  Wraps when it would overflow `u32::MAX`
/// (impossible in practice).
pub fn alloc_ns_inum() -> u32 {
    // Issue distinct values; the wrap branch is unreachable for practical
    // workloads but kept for completeness.
    let v = NS_INUM_COUNTER.fetch_add(1, Ordering::Relaxed);
    if v < PROC_DYNAMIC_FIRST {
        // Wrapped — restart at the base.  In a real kernel this would be
        // bug-territory, but we don't panic in the allocator path.
        NS_INUM_COUNTER.store(PROC_DYNAMIC_FIRST + 1, Ordering::Relaxed);
        return PROC_DYNAMIC_FIRST;
    }
    v
}

impl NsCommon {
    /// Build a fresh `NsCommon` with refcount 1 and a freshly allocated inum.
    pub fn new(ops: *const NsOperations) -> Self {
        Self {
            count: AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops,
            inum: alloc_ns_inum(),
            _pad: 0,
        }
    }

    /// Sticky variant for boot-time singletons that never deallocate.
    pub const fn sticky(ops: *const NsOperations, inum: u32) -> Self {
        Self {
            count: AtomicUsize::new(usize::MAX / 2),
            stashed: core::ptr::null_mut(),
            ops,
            inum,
            _pad: 0,
        }
    }

    #[inline]
    pub fn get(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Drop a reference; returns true when this was the last reference.
    #[inline]
    pub fn put(&self) -> bool {
        let prev = self.count.fetch_sub(1, Ordering::Release);
        prev == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_ns_inum_is_monotonic_and_in_range() {
        let a = alloc_ns_inum();
        let b = alloc_ns_inum();
        assert_ne!(a, b);
        assert!(a >= PROC_DYNAMIC_FIRST);
        assert!(b >= PROC_DYNAMIC_FIRST);
    }

    #[test]
    fn ns_type_tag_values_match_linux() {
        // These values are part of the kernel ABI via `/proc/<pid>/ns/*` inum
        // ordering and the `setns(fd, nstype)` argument.
        assert_eq!(NS_TYPE_MNT, 0);
        assert_eq!(NS_TYPE_CGROUP, 1);
        assert_eq!(NS_TYPE_UTS, 2);
        assert_eq!(NS_TYPE_IPC, 3);
        assert_eq!(NS_TYPE_USER, 4);
        assert_eq!(NS_TYPE_PID, 5);
        assert_eq!(NS_TYPE_NET, 6);
    }
}
