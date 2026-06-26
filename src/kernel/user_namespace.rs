//! linux-parity: partial
//! linux-source: vendor/linux/kernel/user_namespace.c
//! test-origin: linux:vendor/linux/kernel/user_namespace.c
//! User namespace — Milestone 28.
//!
//! `struct user_namespace` carries UID/GID maps and a parent pointer.
//! M28 implements identity mapping (no remap) by default; nested user
//! namespaces inherit the parent map.  Real `uid_map` / `gid_map` writing
//! through `/proc/<pid>/{u,g}id_map` lands with VFS in M39.
//!
//! Reference: Linux `include/linux/user_namespace.h`,
//! `kernel/user_namespace.c`.

use core::sync::atomic::Ordering;

use crate::fs::nsfs::{NS_TYPE_USER, NsCommon, NsOperations, PROC_DYNAMIC_FIRST, alloc_ns_inum};
use crate::kernel::cred::{KGid, KUid};

/// Maximum `uid_extents` / `gid_extents` per `user_namespace`.  Linux uses
/// `UID_GID_MAP_MAX_BASE_EXTENTS = 5` plus a heap-spilled tail; M28 caps at
/// 5 because every consumer in-kernel uses the inline path.
pub const UID_GID_MAP_MAX_EXTENTS: usize = 5;

/// One range entry in a uid/gid map.  Linux: `struct uid_gid_extent`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UidGidExtent {
    pub first: u32,
    pub lower_first: u32,
    pub count: u32,
}

/// `struct uid_gid_map` — fixed-size for M28.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct UidGidMap {
    pub nr_extents: u32,
    pub _pad: u32,
    pub extent: [UidGidExtent; UID_GID_MAP_MAX_EXTENTS],
}

impl Default for UidGidMap {
    fn default() -> Self {
        Self {
            nr_extents: 0,
            _pad: 0,
            extent: [UidGidExtent::default(); UID_GID_MAP_MAX_EXTENTS],
        }
    }
}

impl UidGidMap {
    /// An identity map with one extent: `{0, 0, u32::MAX}`.
    pub const fn identity() -> Self {
        Self {
            nr_extents: 1,
            _pad: 0,
            extent: [
                UidGidExtent {
                    first: 0,
                    lower_first: 0,
                    count: u32::MAX,
                },
                UidGidExtent {
                    first: 0,
                    lower_first: 0,
                    count: 0,
                },
                UidGidExtent {
                    first: 0,
                    lower_first: 0,
                    count: 0,
                },
                UidGidExtent {
                    first: 0,
                    lower_first: 0,
                    count: 0,
                },
                UidGidExtent {
                    first: 0,
                    lower_first: 0,
                    count: 0,
                },
            ],
        }
    }

    /// Translate `id` from the inner namespace to the outer (parent) namespace.
    /// Returns `u32::MAX` (`INVALID_UID/GID`) when no extent covers `id`.
    pub fn translate(&self, id: u32) -> u32 {
        let n = (self.nr_extents as usize).min(UID_GID_MAP_MAX_EXTENTS);
        for ext in &self.extent[..n] {
            if ext.count == 0 {
                continue;
            }
            let end = ext.first.saturating_add(ext.count);
            if id >= ext.first && id < end {
                return ext.lower_first + (id - ext.first);
            }
        }
        u32::MAX
    }
}

/// `struct user_namespace`.
///
/// `level` is 0 for `INIT_USER_NS`; nested user_ns increment by 1 (Linux
/// caps at 32).
#[repr(C)]
pub struct UserNamespace {
    pub ns: NsCommon,
    pub uid_map: UidGidMap,
    pub gid_map: UidGidMap,
    pub projid_map: UidGidMap,
    pub parent: *const UserNamespace,
    pub level: u32,
    pub owner: KUid,
    pub group: KGid,
}

unsafe impl Send for UserNamespace {}
unsafe impl Sync for UserNamespace {}

// ── Operations vtable ────────────────────────────────────────────────────────

unsafe fn user_ns_get(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut UserNamespace;
    if !ns.is_null() {
        unsafe {
            (*ns).ns.get();
        }
    }
}

unsafe fn user_ns_put(ns: *mut core::ffi::c_void) {
    let ns = ns as *mut UserNamespace;
    if ns.is_null() {
        return;
    }
    let last = unsafe { (*ns).ns.put() };
    if last {
        // INIT_USER_NS is sticky; never freed.
        if core::ptr::eq(ns, &raw const INIT_USER_NS as *mut UserNamespace) {
            // Restore sticky count so any spurious extra put is harmless.
            unsafe {
                (*ns).ns.count.store(usize::MAX / 2, Ordering::Relaxed);
            }
            return;
        }
        extern crate alloc;
        unsafe {
            drop(alloc::boxed::Box::from_raw(ns));
        }
    }
}

unsafe fn user_ns_owner(ns: *const core::ffi::c_void) -> *const core::ffi::c_void {
    ns
}

pub static USER_NS_OPS: NsOperations = NsOperations {
    name: "user",
    ns_type: NS_TYPE_USER,
    get: user_ns_get,
    put: user_ns_put,
    owner: user_ns_owner,
};

// ── INIT_USER_NS ─────────────────────────────────────────────────────────────

/// Singleton init user namespace.
pub static INIT_USER_NS: UserNamespace = UserNamespace {
    ns: NsCommon::sticky(&USER_NS_OPS as *const _, PROC_DYNAMIC_FIRST),
    uid_map: UidGidMap::identity(),
    gid_map: UidGidMap::identity(),
    projid_map: UidGidMap::identity(),
    parent: core::ptr::null(),
    level: 0,
    owner: KUid(0),
    group: KGid(0),
};

// ── create_user_ns ───────────────────────────────────────────────────────────

extern crate alloc;

/// Allocate a fresh nested user namespace owned by `parent`.
///
/// Mirrors Linux `create_user_ns()`.  Returns `Err(-ENOMEM)` on allocation
/// failure or `Err(-EUSERS)` if the level cap (32) is exceeded.
pub fn create_user_ns(parent: *const UserNamespace) -> Result<*mut UserNamespace, i32> {
    let parent_level = if parent.is_null() {
        0
    } else {
        unsafe { (*parent).level }
    };
    if parent_level >= 32 {
        return Err(-87); // EUSERS
    }

    let mut b: alloc::boxed::Box<UserNamespace> = alloc::boxed::Box::new(UserNamespace {
        ns: NsCommon {
            count: core::sync::atomic::AtomicUsize::new(1),
            stashed: core::ptr::null_mut(),
            ops: &USER_NS_OPS as *const _,
            inum: alloc_ns_inum(),
            _pad: 0,
        },
        uid_map: UidGidMap::default(), // empty until written via setid_map
        gid_map: UidGidMap::default(),
        projid_map: UidGidMap::default(),
        parent,
        level: parent_level + 1,
        owner: KUid(0),
        group: KGid(0),
    });
    if !parent.is_null() {
        unsafe {
            (*parent).ns.get();
        }
        // Inherit parent identity owner if present.
        b.owner = unsafe { (*parent).owner };
        b.group = unsafe { (*parent).group };
    }
    Ok(alloc::boxed::Box::into_raw(b))
}

// ── make_kuid / make_kgid ────────────────────────────────────────────────────

/// Translate a uid in `ns` to a `KUid` in the parent namespace.
pub fn make_kuid(ns: *const UserNamespace, uid: u32) -> KUid {
    if ns.is_null() {
        return KUid(uid);
    }
    let mapped = unsafe { (*ns).uid_map.translate(uid) };
    if mapped == u32::MAX {
        crate::kernel::cred::INVALID_UID
    } else {
        KUid(mapped)
    }
}

/// Translate a gid in `ns` to a `KGid` in the parent namespace.
pub fn make_kgid(ns: *const UserNamespace, gid: u32) -> KGid {
    if ns.is_null() {
        return KGid(gid);
    }
    let mapped = unsafe { (*ns).gid_map.translate(gid) };
    if mapped == u32::MAX {
        crate::kernel::cred::INVALID_GID
    } else {
        KGid(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_user_ns_is_root_owner() {
        assert_eq!(INIT_USER_NS.level, 0);
        assert_eq!(INIT_USER_NS.owner, KUid(0));
    }

    #[test]
    fn identity_map_round_trips() {
        let m = UidGidMap::identity();
        assert_eq!(m.translate(0), 0);
        assert_eq!(m.translate(1000), 1000);
        // u32::MAX-1 round trips because count == u32::MAX covers
        // [0, u32::MAX) inclusive.
        assert_eq!(m.translate(u32::MAX - 1), u32::MAX - 1);
    }

    #[test]
    fn empty_map_returns_invalid() {
        let m = UidGidMap::default();
        assert_eq!(m.translate(1000), u32::MAX);
    }

    #[test]
    fn create_user_ns_increments_level() {
        let p = create_user_ns(&INIT_USER_NS as *const _).unwrap();
        unsafe {
            assert_eq!((*p).level, 1);
            // Cleanup
            super::user_ns_put(p as *mut core::ffi::c_void);
        }
    }
}
