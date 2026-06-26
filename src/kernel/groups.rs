//! linux-parity: complete
//! linux-source: vendor/linux/kernel/groups.c
//! test-origin: linux:vendor/linux/kernel/groups.c
//! Supplementary group ID list management — M27a.
//!
//! Implements `groups_alloc` / `groups_sort` / `groups_search` / `set_groups`
//! / `set_current_groups` plus the `getgroups(2)` / `setgroups(2)` syscall
//! entry points.  Operates on the `GroupInfo` type already defined in
//! `crate::kernel::cred`; the COW update path here mirrors Linux exactly:
//! `prepare_creds → set_groups → commit_creds`.
//!
//! Reference: vendor/linux/kernel/groups.c
//!            vendor/linux/include/linux/cred.h
//!
//! # Port notes
//!
//! Linux uses a flexible array (`gid[]`) sized at allocation.  Our
//! `GroupInfo` (defined in cred.rs:65) caps the list at
//! `NGROUPS_MAX_INLINE` = 32, which exceeds Lupos's current users.  The
//! `groups_alloc` shim here checks against that cap up front.

extern crate alloc;

use alloc::boxed::Box;
use core::ptr;

use crate::kernel::cred::{
    Cred, GroupInfo, KGid, NGROUPS_MAX_INLINE, commit_creds, current_cred, prepare_creds,
};

/// Linux: `NGROUPS_MAX`.  The supplementary-group cap exposed to userspace.
pub const NGROUPS_MAX: i32 = NGROUPS_MAX_INLINE as i32;

// errno parity
const ENOMEM: i32 = -12;
const EINVAL: i32 = -22;
const EPERM: i32 = -1;
const EFAULT: i32 = -14;

// ── groups_alloc / groups_free ───────────────────────────────────────────────

/// Allocate a heap-backed `GroupInfo` capable of holding `gidsetsize`
/// entries.  Linux: `groups_alloc` from kernel/groups.c:15.
///
/// Returns a raw pointer with refcount 1.  Free via `groups_free` or
/// `put_group_info`.
pub fn groups_alloc(gidsetsize: i32) -> *mut GroupInfo {
    if gidsetsize < 0 || gidsetsize as usize > NGROUPS_MAX_INLINE {
        return ptr::null_mut();
    }
    let mut gi = Box::new(GroupInfo::default());
    gi.ngroups = gidsetsize as u32;
    gi.usage = 1;
    Box::into_raw(gi)
}

/// Free a `GroupInfo` previously returned by `groups_alloc`.  Linux:
/// `groups_free` from kernel/groups.c:29.
///
/// # Safety
/// `gi` must have been produced by `groups_alloc` and have refcount 1.
pub unsafe fn groups_free(gi: *mut GroupInfo) {
    if gi.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(gi)) };
}

/// Increment the reference count.  Linux: `get_group_info` from
/// include/linux/cred.h.
///
/// # Safety
/// `gi` must be non-null and valid.
pub unsafe fn get_group_info(gi: *mut GroupInfo) {
    if gi.is_null() {
        return;
    }
    unsafe { (*gi).usage = (*gi).usage.saturating_add(1) };
}

/// Drop a reference and free when it hits zero.  Linux:
/// `put_group_info` from include/linux/cred.h.
///
/// # Safety
/// `gi` must have been produced by `groups_alloc`.
pub unsafe fn put_group_info(gi: *mut GroupInfo) {
    if gi.is_null() {
        return;
    }
    let prev = unsafe { (*gi).usage };
    unsafe { (*gi).usage = prev.saturating_sub(1) };
    if prev == 1 {
        unsafe { drop(Box::from_raw(gi)) };
    }
}

// ── groups_sort / groups_search ──────────────────────────────────────────────

/// Sort the supplementary group list in ascending order.  Linux:
/// `groups_sort` from kernel/groups.c:84.
pub fn groups_sort(gi: &mut GroupInfo) {
    let n = (gi.ngroups as usize).min(NGROUPS_MAX_INLINE);
    let slice = &mut gi.gid[..n];
    slice.sort_by_key(|kgid| kgid.0);
}

/// Binary-search for `grp`.  Returns 1 (Linux-style) when found, 0
/// otherwise.  Linux: `groups_search` from kernel/groups.c:92.
pub fn groups_search(gi: &GroupInfo, grp: KGid) -> i32 {
    let mut left = 0usize;
    let mut right = (gi.ngroups as usize).min(NGROUPS_MAX_INLINE);
    while left < right {
        let mid = (left + right) / 2;
        let entry = gi.gid[mid];
        if grp.0 > entry.0 {
            left = mid + 1;
        } else if grp.0 < entry.0 {
            right = mid;
        } else {
            return 1;
        }
    }
    0
}

// ── set_groups / set_current_groups ──────────────────────────────────────────

/// Install `group_info` on the freshly prepared cred `new`.  Linux:
/// `set_groups` from kernel/groups.c:118.
///
/// # Safety
/// `new` must be a unique cred pointer returned by `prepare_creds`.
pub unsafe fn set_groups(new: *mut Cred, group_info: &GroupInfo) {
    if new.is_null() {
        return;
    }
    unsafe {
        (*new).group_info = *group_info;
    }
}

/// Replace the current task's supplementary groups by COW.  Linux:
/// `set_current_groups` from kernel/groups.c:134.  Returns 0 on success or
/// `-ENOMEM` if `prepare_creds` fails.
pub fn set_current_groups(group_info: &GroupInfo) -> i32 {
    let Some(new) = prepare_creds() else {
        return ENOMEM;
    };
    unsafe { set_groups(new, group_info) };
    commit_creds(new);
    0
}

// ── Syscall entry points ─────────────────────────────────────────────────────

/// `getgroups(int gidsetsize, gid_t __user *grouplist)`.  Linux:
/// `SYSCALL_DEFINE2(getgroups, ...)` at kernel/groups.c:161.
///
/// When `gidsetsize == 0` the call returns the number of supplementary
/// groups without copying.  Otherwise the supplementary group ids are
/// written into `grouplist` and the count returned.
///
/// # Safety
/// `grouplist` must be a valid pointer to an array large enough for
/// `gidsetsize` entries when `gidsetsize > 0`.
pub unsafe fn sys_getgroups(gidsetsize: i32, grouplist: *mut u32) -> i64 {
    if gidsetsize < 0 {
        return EINVAL as i64;
    }
    let cred = current_cred();
    if cred.is_null() {
        return EFAULT as i64;
    }
    let n = unsafe { (*cred).group_info.ngroups } as i32;
    if gidsetsize == 0 {
        return n as i64;
    }
    if n > gidsetsize {
        return EINVAL as i64;
    }
    if grouplist.is_null() {
        return EFAULT as i64;
    }
    unsafe {
        let gi = &(*cred).group_info;
        for i in 0..(n as usize) {
            *grouplist.add(i) = gi.gid[i].0;
        }
    }
    n as i64
}

/// `setgroups(int gidsetsize, const gid_t __user *grouplist)`.  Linux:
/// `SYSCALL_DEFINE2(setgroups, ...)` at kernel/groups.c:198.
///
/// # Safety
/// `grouplist` must be a valid pointer to an array of `gidsetsize`
/// `gid_t` entries when `gidsetsize > 0`.
pub unsafe fn sys_setgroups(gidsetsize: i32, grouplist: *const u32) -> i64 {
    if gidsetsize < 0 || gidsetsize > NGROUPS_MAX {
        return EINVAL as i64;
    }
    // CAP_SETGID gate — root-only by default.
    let cred = current_cred();
    if cred.is_null() {
        return EFAULT as i64;
    }
    let euid = unsafe { (*cred).euid.0 };
    if euid != 0 {
        return EPERM as i64;
    }

    let mut gi = GroupInfo::default();
    gi.ngroups = gidsetsize as u32;
    if gidsetsize > 0 {
        if grouplist.is_null() {
            return EFAULT as i64;
        }
        unsafe {
            for i in 0..(gidsetsize as usize) {
                gi.gid[i] = KGid(*grouplist.add(i));
            }
        }
        groups_sort(&mut gi);
    }
    set_current_groups(&gi) as i64
}

// ── in_group_p / in_egroup_p ─────────────────────────────────────────────────

/// True if `grp` is the current task's fsgid or appears in the supplementary
/// list.  Linux: `in_group_p` from kernel/groups.c:227.
pub fn in_group_p(grp: KGid) -> bool {
    let cred = current_cred();
    if cred.is_null() {
        return false;
    }
    unsafe {
        if (*cred).fsgid == grp {
            return true;
        }
        groups_search(&(*cred).group_info, grp) == 1
    }
}

/// True if `grp` is the current task's egid or appears in the supplementary
/// list.  Linux: `in_egroup_p` from kernel/groups.c:239.
pub fn in_egroup_p(grp: KGid) -> bool {
    let cred = current_cred();
    if cred.is_null() {
        return false;
    }
    unsafe {
        if (*cred).egid == grp {
            return true;
        }
        groups_search(&(*cred).group_info, grp) == 1
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::cred::{INIT_CRED, NGROUPS_MAX_INLINE};

    #[test]
    fn groups_alloc_respects_cap() {
        let gi = groups_alloc(8);
        assert!(!gi.is_null());
        unsafe {
            assert_eq!((*gi).ngroups, 8);
            assert_eq!((*gi).usage, 1);
            put_group_info(gi);
        }

        assert!(groups_alloc(-1).is_null());
        assert!(groups_alloc((NGROUPS_MAX_INLINE + 1) as i32).is_null());
    }

    #[test]
    fn sort_then_search_finds_member() {
        let mut gi = GroupInfo::default();
        gi.ngroups = 4;
        gi.gid[0] = KGid(50);
        gi.gid[1] = KGid(10);
        gi.gid[2] = KGid(30);
        gi.gid[3] = KGid(20);
        groups_sort(&mut gi);
        assert_eq!(gi.gid[0].0, 10);
        assert_eq!(gi.gid[3].0, 50);
        assert_eq!(groups_search(&gi, KGid(30)), 1);
        assert_eq!(groups_search(&gi, KGid(99)), 0);
    }

    #[test]
    fn empty_group_info_search_returns_zero() {
        let gi = GroupInfo::default();
        assert_eq!(groups_search(&gi, KGid(0)), 0);
    }

    #[test]
    fn refcount_round_trip() {
        let gi = groups_alloc(3);
        unsafe {
            assert_eq!((*gi).usage, 1);
            get_group_info(gi);
            assert_eq!((*gi).usage, 2);
            put_group_info(gi);
            assert_eq!((*gi).usage, 1);
            put_group_info(gi); // last ref → freed
        }
    }

    #[test]
    fn in_group_p_with_init_cred() {
        // INIT_CRED has fsgid=0, egid=0, empty supplementary list.
        // Without a current task, current_cred() falls back to INIT_CRED.
        assert!(in_group_p(KGid(0)));
        assert!(in_egroup_p(KGid(0)));
        // Non-zero gid not in supplementary list returns false.
        assert!(!in_group_p(KGid(1234)));
        assert!(!in_egroup_p(KGid(1234)));
        // Reference the import so the linker keeps INIT_CRED visible for
        // future cred-injected tests.
        let _ = &raw const INIT_CRED;
    }

    #[test]
    fn ngroups_max_matches_inline_cap() {
        assert_eq!(NGROUPS_MAX as usize, NGROUPS_MAX_INLINE);
    }
}
