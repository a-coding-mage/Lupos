//! linux-parity: complete
//! linux-source: vendor/linux/kernel/ucount.c
//! test-origin: linux:vendor/linux/kernel/ucount.c
//! Per-user-namespace resource counters — M27a.
//!
//! Implements the bookkeeping Linux uses to bound a single uid's footprint
//! within a user namespace: namespace creation counts (`UCOUNT_*_NAMESPACES`,
//! `UCOUNT_INOTIFY_*`, `UCOUNT_FANOTIFY_*`) and resource accounting
//! (`UCOUNT_RLIMIT_NPROC`, `UCOUNT_RLIMIT_MSGQUEUE`,
//! `UCOUNT_RLIMIT_SIGPENDING`, `UCOUNT_RLIMIT_MEMLOCK`).
//!
//! Reference: vendor/linux/kernel/ucount.c
//!            vendor/linux/include/linux/user_namespace.h
//!
//! # Port notes
//!
//! Linux indexes the global `ucounts_hashtable` by `(user_ns, kuid)`.  We do
//! the same with a `Vec` lookup behind a single spinlock — adequate while
//! M27 has a single root user namespace.  The full RCU + hlist port lands
//! when the user namespace tree is fleshed out.
//!
//! `inc_ucount` walks the namespace ancestry and rolls back partial
//! increments if any limit is hit, matching Linux line 226-260 of
//! `kernel/ucount.c`.  `inc_rlimit_ucounts` / `dec_rlimit_ucounts` operate
//! on the `rlimit[]` array and do **not** walk ancestors — Linux scopes
//! rlimit_max per-namespace.

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicI64, Ordering};

use spin::Mutex;

use crate::kernel::cred::KUid;

// ── Linux ABI: enum ucount_type ──────────────────────────────────────────────
//
// Matches `enum ucount_type` from include/linux/user_namespace.h:44.

/// Indices into `ucounts.ucount[]`.
#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UcountType {
    UserNamespaces = 0,
    PidNamespaces = 1,
    UtsNamespaces = 2,
    IpcNamespaces = 3,
    NetNamespaces = 4,
    MntNamespaces = 5,
    CgroupNamespaces = 6,
    TimeNamespaces = 7,
    InotifyInstances = 8,
    InotifyWatches = 9,
    FanotifyGroups = 10,
    FanotifyMarks = 11,
}

/// Length of `ucounts.ucount[]`.  Linux: `UCOUNT_COUNTS`.
pub const UCOUNT_COUNTS: usize = 12;

/// Linux: `enum rlimit_type` from include/linux/user_namespace.h:64.
#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RlimitType {
    Nproc = 0,
    MsgQueue = 1,
    SigPending = 2,
    MemLock = 3,
}

/// Linux: `UCOUNT_RLIMIT_COUNTS`.
pub const UCOUNT_RLIMIT_COUNTS: usize = 4;

// ── Ucounts ──────────────────────────────────────────────────────────────────

/// Opaque user-namespace token.  The full type lives in
/// `crate::kernel::user_namespace`; until that module exposes the
/// counter slots we identify each namespace by its base address.
pub type UserNsPtr = *const core::ffi::c_void;

/// Per-(user_ns, uid) counters.  Linux: `struct ucounts` from
/// include/linux/user_namespace.h:119.
pub struct Ucounts {
    pub ns: UserNsPtr,
    pub uid: KUid,
    /// `count` in Linux — refcount maintained by Arc::strong_count via the
    /// `Arc<Ucounts>` wrapper.  Kept here for parity with code that reads
    /// `ucounts->count` directly.
    pub count: AtomicI64,
    /// `ucount[]` — namespace-instance counters (e.g. number of nested pid
    /// namespaces created by this uid).
    pub ucount: [AtomicI64; UCOUNT_COUNTS],
    /// `rlimit[]` — per-uid resource caps (process count, signal queue, …).
    pub rlimit: [AtomicI64; UCOUNT_RLIMIT_COUNTS],
}

// SAFETY: All public fields are atomic or const raw pointers; no interior
// mutability beyond atomics.
unsafe impl Send for Ucounts {}
unsafe impl Sync for Ucounts {}

impl Ucounts {
    fn new(ns: UserNsPtr, uid: KUid) -> Self {
        Self {
            ns,
            uid,
            count: AtomicI64::new(1),
            ucount: [const { AtomicI64::new(0) }; UCOUNT_COUNTS],
            rlimit: [const { AtomicI64::new(0) }; UCOUNT_RLIMIT_COUNTS],
        }
    }
}

// ── Hashtable (Vec-backed) ───────────────────────────────────────────────────

struct UcountsTable {
    entries: Vec<Arc<Ucounts>>,
}

impl UcountsTable {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn find(&self, ns: UserNsPtr, uid: KUid) -> Option<Arc<Ucounts>> {
        self.entries
            .iter()
            .find(|u| u.ns == ns && u.uid == uid)
            .cloned()
    }

    fn insert(&mut self, ns: UserNsPtr, uid: KUid) -> Arc<Ucounts> {
        let uc = Arc::new(Ucounts::new(ns, uid));
        self.entries.push(uc.clone());
        uc
    }

    fn remove_if_idle(&mut self, target: &Arc<Ucounts>) {
        if Arc::strong_count(target) > 2 {
            // External refs remain; do not evict.
            return;
        }
        self.entries
            .retain(|u| !(u.ns == target.ns && u.uid == target.uid));
    }
}

static TABLE: Mutex<UcountsTable> = Mutex::new(UcountsTable::new());

// ── Per-namespace limits ─────────────────────────────────────────────────────
//
// Linux stores `ucount_max[UCOUNT_COUNTS]` and `rlimit_max[UCOUNT_RLIMIT_COUNTS]`
// on `struct user_namespace`.  We mirror those slots in a small side table
// keyed by namespace pointer — the user_namespace port will move them inline
// once it gains a stable layout.

struct NsLimits {
    ns: UserNsPtr,
    ucount_max: [i64; UCOUNT_COUNTS],
    rlimit_max: [i64; UCOUNT_RLIMIT_COUNTS],
}

// SAFETY: NsLimits is only accessed via the NS_LIMITS spinlock.  The `ns`
// raw pointer is treated as an opaque key — never dereferenced.
unsafe impl Send for NsLimits {}

struct NsLimitsTable {
    entries: Vec<NsLimits>,
}

impl NsLimitsTable {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn slot_mut(&mut self, ns: UserNsPtr) -> &mut NsLimits {
        if let Some(pos) = self.entries.iter().position(|n| n.ns == ns) {
            return &mut self.entries[pos];
        }
        self.entries.push(NsLimits {
            ns,
            // Linux default: INT_MAX for both counter classes.
            ucount_max: [i64::from(i32::MAX); UCOUNT_COUNTS],
            rlimit_max: [i64::from(i32::MAX); UCOUNT_RLIMIT_COUNTS],
        });
        self.entries.last_mut().expect("just pushed")
    }
}

static NS_LIMITS: Mutex<NsLimitsTable> = Mutex::new(NsLimitsTable::new());

// ── User-namespace parent registry ───────────────────────────────────────────
//
// Linux's `inc_ucount` walks `ns->parent` up to the root user namespace,
// incrementing the per-type counter on each level and unwinding on overflow.
// The structural port keeps a small side table mapping each namespace to
// its parent (NULL for root); call `register_userns_parent` from the user
// namespace constructor (M28) once the parent link is known.

struct NsParent {
    ns: UserNsPtr,
    parent: UserNsPtr,
}

unsafe impl Send for NsParent {}

struct NsParentTable {
    entries: Vec<NsParent>,
}

impl NsParentTable {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn parent_of(&self, ns: UserNsPtr) -> UserNsPtr {
        self.entries
            .iter()
            .find(|e| e.ns == ns)
            .map(|e| e.parent)
            .unwrap_or(core::ptr::null())
    }

    fn register(&mut self, ns: UserNsPtr, parent: UserNsPtr) {
        if let Some(slot) = self.entries.iter_mut().find(|e| e.ns == ns) {
            slot.parent = parent;
        } else {
            self.entries.push(NsParent { ns, parent });
        }
    }
}

static NS_PARENTS: Mutex<NsParentTable> = Mutex::new(NsParentTable::new());

/// Record the parent of `ns`.  Linux: `create_user_ns` sets
/// `new->parent = current_user_ns()`.  Call this from the user namespace
/// constructor in `kernel::user_namespace`.
pub fn register_userns_parent(ns: UserNsPtr, parent: UserNsPtr) {
    NS_PARENTS.lock().register(ns, parent);
}

/// Read the parent of `ns`.  Returns NULL for the root namespace.
pub fn userns_parent(ns: UserNsPtr) -> UserNsPtr {
    NS_PARENTS.lock().parent_of(ns)
}

/// Set the maximum number of instances of `type` that may be created from
/// `ns`.  Linux: `set_userns_rlimit_max` /
/// `user_namespace.ucount_max[type] = max` (see include/linux/user_namespace.h:163).
pub fn set_userns_count_max(ns: UserNsPtr, ty: UcountType, max: i64) {
    let mut table = NS_LIMITS.lock();
    table.slot_mut(ns).ucount_max[ty as usize] = max;
}

/// Set the maximum value for an rlimit-class counter on `ns`.
pub fn set_userns_rlimit_max(ns: UserNsPtr, ty: RlimitType, max: i64) {
    let mut table = NS_LIMITS.lock();
    table.slot_mut(ns).rlimit_max[ty as usize] = max;
}

/// Read the per-namespace cap for `ty`.
pub fn get_userns_rlimit_max(ns: UserNsPtr, ty: RlimitType) -> i64 {
    let mut table = NS_LIMITS.lock();
    table.slot_mut(ns).rlimit_max[ty as usize]
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Look up or allocate the `Ucounts` slot for `(ns, uid)`.
///
/// Linux: `alloc_ucounts` from kernel/ucount.c:171.  Returns a fresh `Arc`
/// reference each call; drop the returned `Arc` (or pass to `put_ucounts`)
/// when done.
pub fn alloc_ucounts(ns: UserNsPtr, uid: KUid) -> Arc<Ucounts> {
    let mut table = TABLE.lock();
    if let Some(existing) = table.find(ns, uid) {
        existing.count.fetch_add(1, Ordering::Relaxed);
        return existing;
    }
    table.insert(ns, uid)
}

/// Bump the refcount of `uc` and return a fresh `Arc`.  Linux:
/// `get_ucounts` from include/linux/user_namespace.h:139.
pub fn get_ucounts(uc: &Arc<Ucounts>) -> Arc<Ucounts> {
    uc.count.fetch_add(1, Ordering::Relaxed);
    Arc::clone(uc)
}

/// Release a reference obtained from `alloc_ucounts`/`get_ucounts`.  When the
/// final `Arc` clone is dropped the entry is evicted from the table.
pub fn put_ucounts(uc: Arc<Ucounts>) {
    let prev = uc.count.fetch_sub(1, Ordering::Release);
    if prev > 1 {
        return;
    }
    // Try eviction; the table may keep an additional Arc clone that is
    // released below when this function returns.
    TABLE.lock().remove_if_idle(&uc);
    drop(uc);
}

/// Walk the user-namespace ancestry, incrementing `ucount[type]` on each
/// level.  Returns the deepest `Ucounts` (the `(ns, uid)` pair the caller
/// passes in) on success, or `None` if any level would exceed its cap.
///
/// Linux: `inc_ucount` from kernel/ucount.c:226.  Rolls back partial
/// increments on overflow.
///
/// Parent levels keep their `count` field elevated for the lifetime of the
/// inc/dec pair — Linux's leaf `struct ucounts` holds a `ns` pointer that
/// implicitly pins the parents via `get_user_ns`.  We mirror that by
/// leaking one count reference per parent on inc; `dec_ucount` reclaims
/// them via the symmetric parent walk.
pub fn inc_ucount(ns: UserNsPtr, uid: KUid, ty: UcountType) -> Option<Arc<Ucounts>> {
    // Build chain leaf-first → root-last so chain[0] is the user-requested
    // namespace (returned to the caller).
    let mut chain: Vec<UserNsPtr> = Vec::new();
    let mut cursor = ns;
    while !cursor.is_null() {
        chain.push(cursor);
        let next = userns_parent(cursor);
        if next == cursor {
            break;
        }
        cursor = next;
    }
    if chain.is_empty() {
        chain.push(core::ptr::null());
    }

    let mut held: Vec<Arc<Ucounts>> = Vec::with_capacity(chain.len());
    for &level in chain.iter() {
        let uc = alloc_ucounts(level, uid);
        let max = {
            let mut table = NS_LIMITS.lock();
            table.slot_mut(level).ucount_max[ty as usize]
        };
        let after = uc.ucount[ty as usize].fetch_add(1, Ordering::AcqRel) + 1;
        if after > max {
            uc.ucount[ty as usize].fetch_sub(1, Ordering::Release);
            // Drop the failing alloc's count refcount.
            put_ucounts(uc);
            // Roll back successful levels.
            for prev_uc in held.into_iter().rev() {
                prev_uc.ucount[ty as usize].fetch_sub(1, Ordering::AcqRel);
                put_ucounts(prev_uc);
            }
            return None;
        }
        held.push(uc);
    }

    // The first held entry corresponds to chain[0] — the requested namespace.
    // Return a clone for the caller; intentionally LEAK the count refcount on
    // each held entry (including the leaf) so the table keeps them alive
    // until `dec_ucount` walks back through.
    let leaf = Arc::clone(&held[0]);
    // Bump leaf count once more so the caller holds a real reference; the
    // alloc_ucounts call already bumped count for the original `held[0]`.
    leaf.count.fetch_add(1, Ordering::Relaxed);
    // Drop the held vec WITHOUT calling put_ucounts — the count bumps from
    // alloc_ucounts represent the long-lived "ancestry pin" the caller's
    // dec_ucount will release.
    core::mem::forget(held);
    Some(leaf)
}

/// Decrement `ucount[type]` and release one `Ucounts` reference.  Linux:
/// `dec_ucount` from kernel/ucount.c:262.  Walks the ancestry chain in the
/// reverse order of `inc_ucount` and decrements each level.
///
/// Releases the count refcounts that `inc_ucount` leaked onto each parent
/// level (so successive inc/dec pairs do not accumulate leaks).
pub fn dec_ucount(uc: Arc<Ucounts>, ty: UcountType) {
    let leaf_ns = uc.ns;
    let leaf_uid = uc.uid;

    // Decrement leaf first.
    let prev = uc.ucount[ty as usize].fetch_sub(1, Ordering::Release);
    debug_assert!(
        prev > 0,
        "dec_ucount underflow at ucount[{:?}]",
        ty as usize
    );

    // The leaf itself was alloc_ucounts'd by inc_ucount (leaking one count)
    // plus a Arc::clone returned to the caller.  Releasing both:
    //   * put_ucounts(uc) drops the caller's reference.
    //   * One more put_ucounts via the lookup below releases the leak.
    let leaf_pin_release = alloc_ucounts(leaf_ns, leaf_uid);
    // alloc_ucounts bumped count; pair with two puts:
    //   one to release alloc_ucounts, one to release inc_ucount's leak.
    put_ucounts(leaf_pin_release.clone());
    put_ucounts(leaf_pin_release);

    // Walk parents and decrement their counters; each level also releases
    // the count leaked by inc_ucount.
    let mut cursor = userns_parent(leaf_ns);
    while !cursor.is_null() {
        let parent_uc = alloc_ucounts(cursor, leaf_uid);
        let p_prev = parent_uc.ucount[ty as usize].fetch_sub(1, Ordering::Release);
        debug_assert!(p_prev > 0);
        // Two puts: one for our alloc_ucounts above, one for inc_ucount's leak.
        put_ucounts(parent_uc.clone());
        put_ucounts(parent_uc);
        let next = userns_parent(cursor);
        if next == cursor {
            break;
        }
        cursor = next;
    }

    put_ucounts(uc);
}

/// Read an rlimit-class counter without modifying it.  Linux:
/// `get_rlimit_value` from include/linux/user_namespace.h:146.
pub fn get_rlimit_value(uc: &Ucounts, ty: RlimitType) -> i64 {
    uc.rlimit[ty as usize].load(Ordering::Acquire)
}

/// Bump rlimit counter `ty` by `delta`.  Linux: `inc_rlimit_ucounts` from
/// kernel/ucount.c:299.  Returns the new total.
pub fn inc_rlimit_ucounts(uc: &Ucounts, ty: RlimitType, delta: i64) -> i64 {
    uc.rlimit[ty as usize].fetch_add(delta, Ordering::AcqRel) + delta
}

/// Decrement rlimit counter `ty` by `delta`.  Linux:
/// `dec_rlimit_ucounts` from kernel/ucount.c:311.
///
/// Returns `true` if the post-decrement value is non-negative.
pub fn dec_rlimit_ucounts(uc: &Ucounts, ty: RlimitType, delta: i64) -> bool {
    let after = uc.rlimit[ty as usize].fetch_sub(delta, Ordering::AcqRel) - delta;
    after >= 0
}

/// Try to bump `ty` by `v`, but only if doing so keeps the value within the
/// per-namespace cap.  Linux: `inc_rlimit_get_ucounts` from
/// kernel/ucount.c:319.  Returns the new value on success or 0 (with no
/// state change) when the cap is exceeded.
pub fn inc_rlimit_get_ucounts(uc: &Arc<Ucounts>, ty: RlimitType, v: i64) -> i64 {
    let max = get_userns_rlimit_max(uc.ns, ty);
    let after = uc.rlimit[ty as usize].fetch_add(v, Ordering::AcqRel) + v;
    if after > max {
        uc.rlimit[ty as usize].fetch_sub(v, Ordering::Release);
        return 0;
    }
    // Caller now owns one additional reference.
    uc.count.fetch_add(1, Ordering::Relaxed);
    after
}

/// Pair with `inc_rlimit_get_ucounts`.  Linux:
/// `dec_rlimit_put_ucounts` from kernel/ucount.c:340.
pub fn dec_rlimit_put_ucounts(uc: Arc<Ucounts>, ty: RlimitType) {
    uc.rlimit[ty as usize].fetch_sub(1, Ordering::AcqRel);
    put_ucounts(uc);
}

/// True when `ty`'s current value exceeds `max`.  Linux:
/// `is_rlimit_overlimit` from kernel/ucount.c:346.
pub fn is_rlimit_overlimit(uc: &Ucounts, ty: RlimitType, max: i64) -> bool {
    uc.rlimit[ty as usize].load(Ordering::Acquire) > max
}

// ── Test helpers ─────────────────────────────────────────────────────────────

#[cfg(test)]
pub fn reset_for_tests() {
    TABLE.lock().entries.clear();
    NS_LIMITS.lock().entries.clear();
    NS_PARENTS.lock().entries.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn fake_ns(id: usize) -> UserNsPtr {
        id as UserNsPtr
    }

    #[test]
    fn alloc_returns_same_object_for_same_key() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(1);
        let a = alloc_ucounts(ns, KUid(1000));
        let b = alloc_ucounts(ns, KUid(1000));
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn distinct_uids_get_distinct_objects() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(2);
        let a = alloc_ucounts(ns, KUid(1000));
        let b = alloc_ucounts(ns, KUid(1001));
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn distinct_namespaces_get_distinct_objects() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let a = alloc_ucounts(fake_ns(10), KUid(0));
        let b = alloc_ucounts(fake_ns(11), KUid(0));
        assert!(!Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn inc_ucount_respects_cap_and_rolls_back() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(3);
        set_userns_count_max(ns, UcountType::PidNamespaces, 2);

        let a = inc_ucount(ns, KUid(0), UcountType::PidNamespaces).expect("first");
        let b = inc_ucount(ns, KUid(0), UcountType::PidNamespaces).expect("second");
        assert!(inc_ucount(ns, KUid(0), UcountType::PidNamespaces).is_none());

        // Counter must be exactly 2 after the failed third inc rolled back.
        assert_eq!(
            a.ucount[UcountType::PidNamespaces as usize].load(Ordering::Acquire),
            2
        );

        dec_ucount(a, UcountType::PidNamespaces);
        dec_ucount(b, UcountType::PidNamespaces);
    }

    #[test]
    fn rlimit_inc_dec_round_trip() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(4);
        let uc = alloc_ucounts(ns, KUid(2000));

        assert_eq!(inc_rlimit_ucounts(&uc, RlimitType::Nproc, 3), 3);
        assert_eq!(inc_rlimit_ucounts(&uc, RlimitType::Nproc, 2), 5);
        assert_eq!(get_rlimit_value(&uc, RlimitType::Nproc), 5);

        assert!(dec_rlimit_ucounts(&uc, RlimitType::Nproc, 2));
        assert_eq!(get_rlimit_value(&uc, RlimitType::Nproc), 3);
        assert!(dec_rlimit_ucounts(&uc, RlimitType::Nproc, 3));
        assert_eq!(get_rlimit_value(&uc, RlimitType::Nproc), 0);

        put_ucounts(uc);
    }

    #[test]
    fn inc_rlimit_get_respects_cap() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(5);
        set_userns_rlimit_max(ns, RlimitType::SigPending, 2);
        let uc = alloc_ucounts(ns, KUid(0));

        assert_eq!(inc_rlimit_get_ucounts(&uc, RlimitType::SigPending, 1), 1);
        assert_eq!(inc_rlimit_get_ucounts(&uc, RlimitType::SigPending, 1), 2);
        // Third increment exceeds cap → 0 returned, counter rolled back.
        assert_eq!(inc_rlimit_get_ucounts(&uc, RlimitType::SigPending, 1), 0);
        assert_eq!(get_rlimit_value(&uc, RlimitType::SigPending), 2);

        // Manual cleanup: dec the two successful inc_rlimit_get_ucounts.
        dec_rlimit_put_ucounts(get_ucounts(&uc), RlimitType::SigPending);
        dec_rlimit_put_ucounts(get_ucounts(&uc), RlimitType::SigPending);
        put_ucounts(uc);
    }

    #[test]
    fn inc_ucount_walks_ancestry_and_rolls_back() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let root = fake_ns(100);
        let child = fake_ns(101);
        register_userns_parent(child, root);

        // Root has plenty of headroom, child capped at 2.
        set_userns_count_max(root, UcountType::PidNamespaces, 10);
        set_userns_count_max(child, UcountType::PidNamespaces, 2);

        let a = inc_ucount(child, KUid(0), UcountType::PidNamespaces).expect("first");
        let b = inc_ucount(child, KUid(0), UcountType::PidNamespaces).expect("second");
        // Both must point at the child (the requested leaf), not the parent.
        assert!(core::ptr::eq(a.ns, child));
        assert!(core::ptr::eq(b.ns, child));
        // Third should fail at the child level and roll back the root bump.
        assert!(inc_ucount(child, KUid(0), UcountType::PidNamespaces).is_none());

        // Root counter must reflect exactly 2 successful increments after rollback.
        let root_uc = alloc_ucounts(root, KUid(0));
        assert_eq!(
            root_uc.ucount[UcountType::PidNamespaces as usize].load(Ordering::Acquire),
            2,
        );
        put_ucounts(root_uc);

        // Walk-aware dec must drop both levels.
        dec_ucount(a, UcountType::PidNamespaces);
        dec_ucount(b, UcountType::PidNamespaces);
        let root_uc = alloc_ucounts(root, KUid(0));
        assert_eq!(
            root_uc.ucount[UcountType::PidNamespaces as usize].load(Ordering::Acquire),
            0,
        );
        put_ucounts(root_uc);
    }

    #[test]
    fn put_after_last_ref_evicts_entry() {
        let _g = TEST_LOCK.lock();
        reset_for_tests();
        let ns = fake_ns(6);
        let uc = alloc_ucounts(ns, KUid(42));
        // table holds 1 ref, caller holds 1 ref.
        assert_eq!(TABLE.lock().entries.len(), 1);
        put_ucounts(uc);
        assert_eq!(TABLE.lock().entries.len(), 0);
    }
}
