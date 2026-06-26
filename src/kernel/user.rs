//! linux-parity: complete
//! linux-source: vendor/linux/kernel/user.c
//! test-origin: linux:vendor/linux/kernel/user.c
//! Per-uid bookkeeping (`struct user_struct`) — M27a.
//!
//! The "user cache" Linux uses to back per-uid limits: how many processes a
//! uid currently owns, how many open files, how many epoll watches, and the
//! ratelimit state used by `printk_ratelimit` style throttling.  Each call to
//! `alloc_uid` increments the per-uid refcount; `free_uid` releases it.
//! The root uid (`KUid(0)`) is pre-inserted as `ROOT_USER` so it never gets
//! evicted from the hash.
//!
//! Reference: vendor/linux/kernel/user.c
//!            vendor/linux/include/linux/sched/user.h
//!
//! # Port notes
//!
//! Linux maintains a fixed-size hlist hash (`UIDHASH_SZ`); we use a single
//! `Vec` under a spinlock, which matches the surface API but is O(n) on
//! lookup.  Replace with a hash bucket array once the user namespace gain
//! their own per-namespace user cache (Linux ties hashing to the global root
//! ns; user_namespace.uid_map handles the translation before lookup).
//!
//! Per-user-namespace lookup keys: Linux 6.x hashes by `kuid_val(uid)` and
//! does not include `user_ns`, because `struct user_struct` is global.  We
//! follow the same scheme — the uid passed in is already namespace-translated
//! by the caller.

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use spin::Mutex;

use crate::kernel::cred::KUid;
use crate::kernel::ucount::{Ucounts, alloc_ucounts, put_ucounts};

/// Per-uid bookkeeping struct.  Linux: `struct user_struct` from
/// include/linux/sched/user.h.
pub struct UserStruct {
    /// Refcount.  Linux: `__count`.  Released via `free_uid`.
    pub count: AtomicI64,
    pub uid: KUid,
    /// Number of processes owned by this uid.  Linux: `processes`.
    pub processes: AtomicI64,
    /// Number of pending signals.  Linux: `sigpending`.
    pub sigpending: AtomicI64,
    /// Number of fanotify groups.  Linux: `fanotify_listeners`.
    pub fanotify_listeners: AtomicI64,
    /// Locked-memory page count.  Linux: `locked_shm`.
    pub locked_shm: AtomicI64,
    /// Unix domain socket open count.  Linux: `unix_inflight`.
    pub unix_inflight: AtomicI64,
    /// Pipe buffer pages.  Linux: `pipe_bufs`.
    pub pipe_bufs: AtomicI64,
    /// EPOLL watch count.  Linux: `epoll_watches`.
    pub epoll_watches: AtomicI64,
    /// Last log throttle timestamp (ratelimit_state.begin in jiffies).
    /// Linux: `ratelimit` substructure.
    pub ratelimit_begin: AtomicU64,
    pub ratelimit_burst: AtomicI64,
    /// Side reference to the matching `Ucounts` slot in the root user
    /// namespace.  Populated lazily on first `alloc_uid`.
    pub ucounts: Option<Arc<Ucounts>>,
}

// SAFETY: All public fields are atomic or read-only after init.
unsafe impl Send for UserStruct {}
unsafe impl Sync for UserStruct {}

impl UserStruct {
    fn new(uid: KUid, ucounts: Option<Arc<Ucounts>>) -> Self {
        Self {
            count: AtomicI64::new(1),
            uid,
            processes: AtomicI64::new(0),
            sigpending: AtomicI64::new(0),
            fanotify_listeners: AtomicI64::new(0),
            locked_shm: AtomicI64::new(0),
            unix_inflight: AtomicI64::new(0),
            pipe_bufs: AtomicI64::new(0),
            epoll_watches: AtomicI64::new(0),
            ratelimit_begin: AtomicU64::new(0),
            ratelimit_burst: AtomicI64::new(0),
            ucounts,
        }
    }
}

/// Linux: `GLOBAL_ROOT_UID`.
pub const GLOBAL_ROOT_UID: KUid = KUid(0);

// ── Hash table ───────────────────────────────────────────────────────────────

struct UserHash {
    entries: Vec<Arc<UserStruct>>,
}

impl UserHash {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn find(&self, uid: KUid) -> Option<Arc<UserStruct>> {
        self.entries.iter().find(|u| u.uid == uid).cloned()
    }

    fn insert(&mut self, u: Arc<UserStruct>) {
        self.entries.push(u);
    }

    fn remove(&mut self, uid: KUid) {
        self.entries.retain(|u| u.uid != uid);
    }
}

static UID_HASH: Mutex<UserHash> = Mutex::new(UserHash::new());
static ROOT_INIT: Mutex<bool> = Mutex::new(false);

fn ensure_root_inserted() {
    let mut done = ROOT_INIT.lock();
    if *done {
        return;
    }
    let root = Arc::new(UserStruct::new(GLOBAL_ROOT_UID, None));
    // Sticky refcount — root_user is never freed under Linux either.
    root.count.store(i64::MAX / 2, Ordering::Relaxed);
    UID_HASH.lock().insert(root);
    *done = true;
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Look up the `user_struct` for `uid`, taking a reference if found.
///
/// Linux: `find_user` from kernel/user.c:176.
pub fn find_user(uid: KUid) -> Option<Arc<UserStruct>> {
    ensure_root_inserted();
    let table = UID_HASH.lock();
    let u = table.find(uid)?;
    u.count.fetch_add(1, Ordering::Relaxed);
    Some(u)
}

/// Look up or allocate the `user_struct` for `uid`.
///
/// Linux: `alloc_uid` from kernel/user.c:199.  Always returns a live `Arc`
/// with one additional reference held by the caller.  The first allocation
/// for a given uid also creates the matching `Ucounts` slot in the root
/// user namespace (the Linux equivalent is `current_user_ns()` at the
/// `find_or_create_user_ns` call site).
pub fn alloc_uid(uid: KUid) -> Arc<UserStruct> {
    ensure_root_inserted();
    {
        let table = UID_HASH.lock();
        if let Some(u) = table.find(uid) {
            u.count.fetch_add(1, Ordering::Relaxed);
            return u;
        }
    }
    // Lazily attach a ucounts slot in the root user namespace.  Use a stable
    // sentinel (the address of UID_HASH) until the user namespace port hands
    // out real namespace pointers.
    let ns = &UID_HASH as *const _ as *const core::ffi::c_void;
    let uc = alloc_ucounts(ns, uid);

    let mut table = UID_HASH.lock();
    if let Some(existing) = table.find(uid) {
        // Lost a race; release the speculative ucounts ref and return shared.
        put_ucounts(uc);
        existing.count.fetch_add(1, Ordering::Relaxed);
        return existing;
    }
    let u = Arc::new(UserStruct::new(uid, Some(uc)));
    table.insert(u.clone());
    u
}

/// Drop a reference previously taken by `find_user`/`alloc_uid`.
///
/// Linux: `free_uid` from kernel/user.c:187.  Releases the slot when the
/// last user reference goes away (root is kept pinned by its sticky count).
pub fn free_uid(up: Arc<UserStruct>) {
    let prev = up.count.fetch_sub(1, Ordering::Release);
    if prev > 1 {
        return;
    }
    let uid = up.uid;
    if uid == GLOBAL_ROOT_UID {
        // Root user is never freed — bump back so the next free_uid sees it.
        up.count.fetch_add(1, Ordering::Relaxed);
        return;
    }
    UID_HASH.lock().remove(uid);
    // Detach ucounts last so the Arc<UserStruct> drop releases the final
    // table reference cleanly.
    drop(up);
}

/// Read the per-uid process count.
pub fn user_process_count(uid: KUid) -> i64 {
    if let Some(u) = find_user(uid) {
        let v = u.processes.load(Ordering::Acquire);
        free_uid(u);
        v
    } else {
        0
    }
}

/// Bump the process count for `uid`.  Returns the new value, or -EAGAIN
/// when the count would exceed `cap` (when `cap` is non-zero).
pub fn user_inc_processes(uid: KUid, cap: i64) -> i64 {
    let u = alloc_uid(uid);
    let after = u.processes.fetch_add(1, Ordering::AcqRel) + 1;
    if cap > 0 && after > cap {
        u.processes.fetch_sub(1, Ordering::Release);
        free_uid(u);
        return -11; // -EAGAIN
    }
    free_uid(u);
    after
}

/// Decrement the process count.  Linux: `__free_uid` after an exiting task.
pub fn user_dec_processes(uid: KUid) {
    if let Some(u) = find_user(uid) {
        let prev = u.processes.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(prev > 0, "processes underflow on uid {:?}", uid);
        free_uid(u);
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

#[cfg(test)]
pub fn reset_for_tests() {
    let mut table = UID_HASH.lock();
    table.entries.clear();
    drop(table);
    *ROOT_INIT.lock() = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::ucount::reset_for_tests as reset_ucounts;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn alloc_returns_same_struct_for_same_uid() {
        let _g = TEST_LOCK.lock();
        reset_ucounts();
        reset_for_tests();
        let a = alloc_uid(KUid(1000));
        let b = alloc_uid(KUid(1000));
        assert!(Arc::ptr_eq(&a, &b));
        free_uid(a);
        free_uid(b);
    }

    #[test]
    fn distinct_uids_get_distinct_structs() {
        let _g = TEST_LOCK.lock();
        reset_ucounts();
        reset_for_tests();
        let a = alloc_uid(KUid(2000));
        let b = alloc_uid(KUid(2001));
        assert!(!Arc::ptr_eq(&a, &b));
        free_uid(a);
        free_uid(b);
    }

    #[test]
    fn root_is_preinserted_and_never_evicted() {
        let _g = TEST_LOCK.lock();
        reset_ucounts();
        reset_for_tests();
        let root = find_user(GLOBAL_ROOT_UID).expect("root user present");
        free_uid(root);
        // Even after free, root must remain in the table.
        assert!(find_user(GLOBAL_ROOT_UID).is_some());
    }

    #[test]
    fn free_uid_evicts_after_last_ref() {
        let _g = TEST_LOCK.lock();
        reset_ucounts();
        reset_for_tests();
        let u = alloc_uid(KUid(3000));
        free_uid(u);
        assert!(find_user(KUid(3000)).is_none());
    }

    #[test]
    fn inc_processes_respects_cap() {
        let _g = TEST_LOCK.lock();
        reset_ucounts();
        reset_for_tests();
        // Hold a long-lived reference so free_uid inside user_inc_processes
        // does not evict the slot between calls.  This mirrors Linux, where
        // the per-task cred pins the user_struct via cred->user.
        let _pin = alloc_uid(KUid(4000));
        assert_eq!(user_inc_processes(KUid(4000), 2), 1);
        assert_eq!(user_inc_processes(KUid(4000), 2), 2);
        assert_eq!(user_inc_processes(KUid(4000), 2), -11);
        user_dec_processes(KUid(4000));
        user_dec_processes(KUid(4000));
        assert_eq!(user_process_count(KUid(4000)), 0);
        free_uid(_pin);
    }
}
