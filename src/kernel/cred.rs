//! linux-parity: complete
//! linux-source: vendor/linux/kernel/cred.c
//! test-origin: linux:vendor/linux/kernel/cred.c
//! Task credentials (`struct cred`) — Milestone 27.
//!
//! Implements:
//!   - `Cred` — the per-task credential block (uid/gid/caps/securebits).
//!   - `INIT_CRED` — the boot-time credential singleton.
//!   - `prepare_creds`, `commit_creds`, `override_creds`, `revert_creds` —
//!     the canonical Linux COW credential-update protocol.
//!   - `current_cred` — read the calling task's effective cred.
//!
//! # COW protocol
//!
//! 1. `prepare_creds()` allocates a fresh `Cred` initialised from the current
//!    cred.  The caller mutates the new cred (e.g. drops a capability).
//! 2. `commit_creds(new)` swaps `current.cred = new`, refcount-decrementing
//!    the old cred.  Linux uses RCU to guarantee readers never see a torn
//!    pointer; M27 relies on the cooperative scheduler — full RCU lands in M34.
//! 3. `override_creds(new)` saves+swaps in one step and returns the saved
//!    cred so `revert_creds(old)` can restore.  Used at security boundaries.
//!
//! Reference: Linux `include/linux/cred.h`, `kernel/cred.c`.

extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::kernel::capability::KernelCapT;

// ── User identifier types ────────────────────────────────────────────────────

/// Kernel user-ID.  Linux: `kuid_t`.  Currently identity-mapped (no user-NS
/// translation until M28).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct KUid(pub u32);

/// Kernel group-ID.  Linux: `kgid_t`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct KGid(pub u32);

/// Sentinel "invalid" user ID — Linux `INVALID_UID`.
pub const INVALID_UID: KUid = KUid(u32::MAX);
/// Sentinel "invalid" group ID — Linux `INVALID_GID`.
pub const INVALID_GID: KGid = KGid(u32::MAX);

/// `securebits` flags — Linux `include/uapi/linux/securebits.h`.
pub mod securebits {
    pub const SECURE_NOROOT: u32 = 0;
    pub const SECURE_NO_SETUID_FIXUP: u32 = 2;
    pub const SECURE_KEEP_CAPS: u32 = 4;
    pub const SECURE_NO_CAP_AMBIENT_RAISE: u32 = 6;
}

// ── Group info ───────────────────────────────────────────────────────────────

pub const NGROUPS_MAX_INLINE: usize = 32;

/// Supplementary group list.  Linux `struct group_info` uses a flexible array;
/// M27 caps the count at `NGROUPS_MAX_INLINE` which is sufficient for our
/// in-kernel users.  Full dynamic sizing arrives with the user-namespace
/// gid_map work in M28.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GroupInfo {
    pub usage: u32,
    pub ngroups: u32,
    pub gid: [KGid; NGROUPS_MAX_INLINE],
}

impl Default for GroupInfo {
    fn default() -> Self {
        Self {
            usage: 1,
            ngroups: 0,
            gid: [KGid(0); NGROUPS_MAX_INLINE],
        }
    }
}

// ── Cred ─────────────────────────────────────────────────────────────────────

/// Per-task credential block.
///
/// Linux `struct cred` from `include/linux/cred.h`.  We reproduce the
/// observable fields; alignment-only fields (`subscribers`, `magic`,
/// `non_rcu`, `rcu`) are omitted as they have no ABI consumer in M27.  The
/// `usage` refcount provides correct ownership semantics under the
/// cooperative scheduler.
///
/// # Refcounting
///
/// `usage == 0` means freed; the value is incremented by `get_cred` and
/// decremented by `put_cred`.  When `put_cred` brings `usage` to 0 it
/// deallocates the box.  M34 will replace the swap in `commit_creds` with
/// `rcu_assign_pointer` so concurrent readers see consistent snapshots.
#[repr(C)]
pub struct Cred {
    pub usage: AtomicUsize,
    /// Real UID — the UID the task was created with.
    pub uid: KUid,
    /// Real GID.
    pub gid: KGid,
    /// Saved set-user-ID.
    pub suid: KUid,
    /// Saved set-group-ID.
    pub sgid: KGid,
    /// Effective UID — the one used for permission checks.
    pub euid: KUid,
    /// Effective GID.
    pub egid: KGid,
    /// File-system UID — used for filesystem accesses (separated from euid).
    pub fsuid: KUid,
    /// File-system GID.
    pub fsgid: KGid,
    /// Inheritable capability mask.
    pub cap_inheritable: KernelCapT,
    /// Permitted capability mask.
    pub cap_permitted: KernelCapT,
    /// Effective capability mask.
    pub cap_effective: KernelCapT,
    /// Bounding capability set.
    pub cap_bset: KernelCapT,
    /// Ambient capability set (Linux 4.3+).
    pub cap_ambient: KernelCapT,
    /// `securebits` flags.
    pub securebits: u32,
    /// Supplementary groups.
    pub group_info: GroupInfo,
    /// Owning user namespace pointer (raw — type defined in M28).
    pub user_ns: *const core::ffi::c_void,
}

// SAFETY: Cred is shared between tasks via refcount; raw pointer fields are
// either null (in M27) or point to long-lived statics.
unsafe impl Send for Cred {}
unsafe impl Sync for Cred {}

impl Cred {
    /// Bump the refcount and return self.
    #[inline]
    pub fn get(&self) -> &Self {
        self.usage.fetch_add(1, Ordering::Relaxed);
        self
    }

    /// Drop a reference; if this was the last one, deallocate.
    ///
    /// # Safety
    /// `cred` must have been obtained from `prepare_creds`/`get_cred` and
    /// not yet released.
    pub unsafe fn put(cred: *const Cred) {
        if cred.is_null() {
            return;
        }
        // INIT_CRED is reference-counted but never freed — its pointer is
        // identity-stable for the lifetime of the kernel.
        if core::ptr::eq(cred, &raw const INIT_CRED as *const Cred) {
            unsafe {
                (*cred).usage.fetch_sub(1, Ordering::Release);
            }
            return;
        }
        let prev = unsafe { (*cred).usage.fetch_sub(1, Ordering::Release) };
        if prev == 1 {
            unsafe { drop(Box::from_raw(cred as *mut Cred)) };
        }
    }
}

// ── INIT_CRED (boot singleton) ───────────────────────────────────────────────

/// The init task's credential block.
///
/// Root (uid=0, gid=0) with the full capability set raised, owning a static
/// init_user_ns pointer (NULL for M27 — populated by M28's `INIT_USER_NS`).
pub static INIT_CRED: Cred = Cred {
    usage: AtomicUsize::new(usize::MAX / 2), // sticky — never freed
    uid: KUid(0),
    gid: KGid(0),
    suid: KUid(0),
    sgid: KGid(0),
    euid: KUid(0),
    egid: KGid(0),
    fsuid: KUid(0),
    fsgid: KGid(0),
    cap_inheritable: KernelCapT::empty(),
    cap_permitted: KernelCapT::full(),
    cap_effective: KernelCapT::full(),
    cap_bset: KernelCapT::full(),
    cap_ambient: KernelCapT::empty(),
    securebits: 0,
    group_info: GroupInfo {
        usage: 1,
        ngroups: 0,
        gid: [KGid(0); NGROUPS_MAX_INLINE],
    },
    user_ns: core::ptr::null(),
};

// ── current_cred / prepare_creds / commit_creds / override_creds ─────────────

/// Read the calling task's effective cred.
///
/// Returns `&INIT_CRED` if the scheduler is not yet running or the current
/// task has a null cred pointer (kernel-thread bring-up before
/// `commit_creds` ran).
pub fn current_cred() -> *const Cred {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return &raw const INIT_CRED;
    }
    let p = unsafe { (*task).cred };
    if p.is_null() { &raw const INIT_CRED } else { p }
}

/// Allocate a new `Cred` initialised from `current_cred()` with `usage == 1`.
///
/// Mirrors Linux `prepare_creds()` — the start of every COW credential change.
pub fn prepare_creds() -> Option<*mut Cred> {
    let cur = current_cred();
    if cur.is_null() {
        return None;
    }
    // Box-allocate and copy.
    let new = unsafe {
        let mut c: Box<Cred> = Box::new(core::mem::zeroed());
        *c = Cred {
            usage: AtomicUsize::new(1),
            uid: (*cur).uid,
            gid: (*cur).gid,
            suid: (*cur).suid,
            sgid: (*cur).sgid,
            euid: (*cur).euid,
            egid: (*cur).egid,
            fsuid: (*cur).fsuid,
            fsgid: (*cur).fsgid,
            cap_inheritable: (*cur).cap_inheritable,
            cap_permitted: (*cur).cap_permitted,
            cap_effective: (*cur).cap_effective,
            cap_bset: (*cur).cap_bset,
            cap_ambient: (*cur).cap_ambient,
            securebits: (*cur).securebits,
            group_info: (*cur).group_info,
            user_ns: (*cur).user_ns,
        };
        Box::into_raw(c)
    };
    Some(new)
}

/// Commit `new` as the calling task's credential, releasing the old one.
///
/// Linux semantics: both `cred` and `real_cred` are updated to `new`
/// (separating real from effective is reserved for `setresuid` / file
/// capabilities — neither implemented in M27).
///
/// # Safety
/// `new` must be a unique cred pointer with `usage >= 1`.
pub fn commit_creds(new: *mut Cred) {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        // Pre-init: just drop the new cred — there's no task to update.
        unsafe { Cred::put(new) };
        return;
    }
    let old = unsafe { (*task).cred };
    let old_real = unsafe { (*task).m27.real_cred };
    // Bump refcount once more so cred and real_cred each own a reference.
    unsafe { (*new).usage.fetch_add(1, Ordering::Relaxed) };
    unsafe {
        (*task).cred = new as *const Cred;
        (*task).m27.real_cred = new as *const Cred;
    }
    // M34: replace with rcu_assign_pointer + synchronize_rcu before the puts.
    unsafe {
        Cred::put(old);
        if !old_real.is_null() && !core::ptr::eq(old_real, old) {
            Cred::put(old_real);
        }
    }
}

/// Atomically swap in `new` and return the previous cred so it can be
/// restored later via `revert_creds`.
///
/// Mirrors Linux `override_creds()` — used by callers that need to elevate
/// or de-privilege themselves for a single operation.
pub fn override_creds(new: *const Cred) -> *const Cred {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return core::ptr::null();
    }
    let old = unsafe { (*task).cred };
    if !new.is_null() {
        unsafe { (*new).usage.fetch_add(1, Ordering::Relaxed) };
    }
    unsafe { (*task).cred = new };
    old
}

/// Restore a cred previously saved by `override_creds`.
pub fn revert_creds(old: *const Cred) {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return;
    }
    let cur = unsafe { (*task).cred };
    unsafe { (*task).cred = old };
    unsafe { Cred::put(cur) };
}

// ── copy_creds (called by copy_process) ──────────────────────────────────────

/// Initialise `child.cred` and `child.real_cred` from the parent.
///
/// Linux `copy_creds(p, clone_flags)` from `kernel/cred.c`:
/// - With `CLONE_THREAD`: the child shares the parent's cred (no copy).
/// - Without `CLONE_THREAD`: a fresh COW copy is allocated.
///
/// # Safety
/// `child` and `parent` must be valid TaskStruct pointers.
pub unsafe fn copy_creds(
    child: *mut crate::kernel::task::TaskStruct,
    parent: *mut crate::kernel::task::TaskStruct,
    clone_flags: u64,
) -> Result<(), i32> {
    use crate::kernel::clone::{CLONE_NEWUSER, CLONE_THREAD};

    let parent_cred = unsafe { (*parent).cred };
    let cred_to_use: *const Cred = if parent_cred.is_null() {
        &raw const INIT_CRED
    } else {
        parent_cred
    };

    if clone_flags & CLONE_THREAD != 0 {
        // Share — bump refcount twice (once for cred, once for real_cred).
        unsafe {
            (*cred_to_use).usage.fetch_add(2, Ordering::Relaxed);
            (*child).cred = cred_to_use;
            (*child).m27.real_cred = cred_to_use;
        }
    } else {
        // COW — allocate a private copy.
        if clone_flags & CLONE_NEWUSER != 0 {
            // Defense in depth for callers that bypass kernel_clone(): do not
            // create user namespaces while capable()/ns_capable() are still
            // global bitmask checks.
            return Err(-1);
        }
        let new = unsafe {
            let mut c: Box<Cred> = Box::new(core::mem::zeroed());
            *c = Cred {
                usage: AtomicUsize::new(2), // cred + real_cred
                uid: (*cred_to_use).uid,
                gid: (*cred_to_use).gid,
                suid: (*cred_to_use).suid,
                sgid: (*cred_to_use).sgid,
                euid: (*cred_to_use).euid,
                egid: (*cred_to_use).egid,
                fsuid: (*cred_to_use).fsuid,
                fsgid: (*cred_to_use).fsgid,
                cap_inheritable: (*cred_to_use).cap_inheritable,
                cap_permitted: (*cred_to_use).cap_permitted,
                cap_effective: (*cred_to_use).cap_effective,
                cap_bset: (*cred_to_use).cap_bset,
                cap_ambient: (*cred_to_use).cap_ambient,
                securebits: (*cred_to_use).securebits,
                group_info: (*cred_to_use).group_info,
                user_ns: (*cred_to_use).user_ns,
            };
            Box::into_raw(c)
        };
        unsafe {
            (*child).cred = new as *const Cred;
            (*child).m27.real_cred = new as *const Cred;
        }
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::capability::CAP_SYS_ADMIN;

    #[test]
    fn init_cred_is_root_with_full_caps() {
        assert_eq!(INIT_CRED.uid, KUid(0));
        assert_eq!(INIT_CRED.gid, KGid(0));
        assert!(INIT_CRED.cap_effective.raised(CAP_SYS_ADMIN));
        assert!(INIT_CRED.cap_permitted.raised(CAP_SYS_ADMIN));
        assert!(!INIT_CRED.cap_inheritable.raised(CAP_SYS_ADMIN));
    }

    #[test]
    fn invalid_uid_is_uint_max() {
        assert_eq!(INVALID_UID.0, u32::MAX);
        assert_eq!(INVALID_GID.0, u32::MAX);
    }

    #[test]
    fn group_info_default_is_empty() {
        let gi = GroupInfo::default();
        assert_eq!(gi.ngroups, 0);
        assert_eq!(gi.usage, 1);
    }
}
