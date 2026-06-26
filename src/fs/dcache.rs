//! linux-parity: complete
//! linux-source: vendor/linux/fs/dcache.c
//! test-origin: linux:vendor/linux/fs/dcache.c
//! Dentry cache — `d_alloc`, `d_lookup`, `d_instantiate`, `dput`.
//!
//! Lupos keeps the cache as per-parent BTreeMap children inside each `Dentry`
//! (see `types.rs::Dentry::children`).  A global hash is unnecessary at this
//! scale — every lookup descends from the SuperBlock root, and parent locks
//! serialize concurrent inserts.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::Ordering;

use super::types::{Dentry, DentryRef, InodeRef};

/// Allocate a negative dentry not linked to any parent.
pub fn d_alloc(name: &str) -> DentryRef {
    Dentry::new_negative(name)
}

/// Allocate a dentry as a child of `parent`, link it into the parent's child
/// table, and return the new dentry.  No inode bound yet — caller fills it
/// via `d_instantiate`, or leaves it negative to mirror Linux `d_add(NULL)`.
pub fn d_alloc_child(parent: &DentryRef, name: &str) -> DentryRef {
    let d = d_alloc(name);
    *d.parent.lock() = Some(parent.clone());
    parent
        .children
        .write()
        .insert(alloc::string::String::from(name), d.clone());
    d
}

/// Bind `inode` to `dentry`, clearing the negative flag.
pub fn d_instantiate(dentry: &DentryRef, inode: InodeRef) {
    dentry.instantiate(inode);
}

/// Look up `name` in `parent`'s dcache.  Returns positive and negative
/// dentries, matching Linux `d_lookup`; callers that require an inode must
/// check `dentry.inode()`.
pub fn d_lookup(parent: &DentryRef, name: &str) -> Option<DentryRef> {
    let children = parent.children.read();
    children
        .iter()
        .find(|(child_name, _)| names_eq(child_name.as_str(), name))
        .map(|(_, dentry)| dentry.clone())
}

/// Cache a negative child dentry after the filesystem lookup reports ENOENT.
/// This mirrors Linux's slow lookup path: allocate the dentry first, then keep
/// it hashed as a negative dentry when `i_op->lookup` finds no inode.
pub fn d_cache_negative(parent: &DentryRef, name: &str) -> DentryRef {
    if let Some(existing) = d_lookup(parent, name) {
        return existing;
    }
    d_alloc_child(parent, name)
}

/// Drop a reference to `dentry` (matches Linux `dput`).  Arc handles the real
/// deallocation; we only update the diagnostic counter.
pub fn dput(dentry: DentryRef) {
    dentry.d_count.fetch_sub(1, Ordering::AcqRel);
    drop(dentry);
}

/// Acquire a fresh reference (matches Linux `dget`).
pub fn dget(dentry: &DentryRef) -> DentryRef {
    dentry.d_count.fetch_add(1, Ordering::AcqRel);
    dentry.clone()
}

/// Detach `name` from `parent`'s child table.  Caller must have already
/// torn down the inode side via `unlink`/`rmdir`.
pub fn d_drop(parent: &DentryRef, name: &str) {
    let mut children = parent.children.write();
    let key = children
        .keys()
        .find(|child_name| names_eq(child_name.as_str(), name))
        .cloned();
    if let Some(key) = key {
        children.remove(&key);
    }
}

fn names_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Walk parent → child along the slash-separated `path`, starting at `root`.
/// Returns the resolved dentry or `None` on the first miss.  M38 helper —
/// M39 replaces this with `path_lookupat`.
pub fn d_walk(root: &DentryRef, path: &str) -> Option<DentryRef> {
    let mut cur = root.clone();
    for comp in path.split('/').filter(|c| !c.is_empty() && *c != ".") {
        let next = if comp == ".." {
            cur.parent.lock().clone().unwrap_or_else(|| cur.clone())
        } else if let Some(d) = d_lookup(&cur, comp) {
            d.inode()?;
            d
        } else {
            // Try inode_ops.lookup as a fallback, then cache the result.
            let dir_inode = cur.inode()?;
            let lookup = dir_inode.ops.lookup?;
            let child_inode = match lookup(&dir_inode, comp) {
                Ok(inode) => inode,
                Err(crate::include::uapi::errno::ENOENT) => {
                    d_cache_negative(&cur, comp);
                    return None;
                }
                Err(_) => return None,
            };
            let new_d = d_alloc_child(&cur, comp);
            d_instantiate(&new_d, child_inode);
            new_d
        };
        cur = next;
    }
    Some(cur)
}

/// Diagnostic — Arc strong count.  Used by acceptance tests to verify
/// dput returns to baseline.
pub fn d_strong_count(d: &DentryRef) -> usize {
    Arc::strong_count(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d_alloc_creates_negative() {
        let d = d_alloc("x");
        assert!(d.is_negative());
        assert_eq!(d.d_count.load(Ordering::Acquire), 1);
    }
}
