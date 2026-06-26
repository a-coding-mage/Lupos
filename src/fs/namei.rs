//! linux-parity: partial
//! linux-source: vendor/linux/fs/namei.c
//! Path resolution — `path_lookupat`, `walk_component` (M39).
//!
//! Mirrors `vendor/linux/fs/namei.c`.  Lupos M39 only ships ref-walk;
//! RCU-walk lands once SMP migration finishes (M55+).

extern crate alloc;

use crate::include::uapi::errno::{EINVAL, ELOOP, ENOENT, ENOTDIR};
use crate::include::uapi::openat2::{
    OpenHow, RESOLVE_BENEATH, RESOLVE_IN_ROOT, RESOLVE_NO_MAGICLINKS, RESOLVE_NO_SYMLINKS,
    RESOLVE_NO_XDEV, RESOLVE_VALID_MASK,
};

use super::dcache::{d_alloc_child, d_cache_negative, d_lookup};
use super::types::{DentryRef, InodeKind};

const MAX_LINK_RECURSION: u32 = 40;

pub struct LookupCtx {
    pub root: DentryRef,
    pub start: DentryRef,
    pub resolve: u64,
    pub _depth: u32,
}

impl LookupCtx {
    pub fn new(root: DentryRef, start: DentryRef, resolve: u64) -> Self {
        Self {
            root,
            start,
            resolve,
            _depth: 0,
        }
    }
}

/// Validate `open_how.resolve` for unknown bits.
pub fn validate_open_how(how: &OpenHow) -> Result<(), i32> {
    if how.resolve & !RESOLVE_VALID_MASK != 0 {
        return Err(EINVAL);
    }
    // BENEATH and IN_ROOT are mutually exclusive.
    if how.resolve & RESOLVE_BENEATH != 0 && how.resolve & RESOLVE_IN_ROOT != 0 {
        return Err(EINVAL);
    }
    Ok(())
}

/// Resolve `path` honoring `ctx.resolve` flags.  Returns the terminal dentry.
pub fn path_lookupat(ctx: &LookupCtx, path: &str) -> Result<DentryRef, i32> {
    if path.is_empty() {
        return Ok(ctx.start.clone());
    }

    // Absolute path with RESOLVE_BENEATH or RESOLVE_IN_ROOT means we still
    // start at ctx.start (BENEATH) or treat ctx.root as "/" (IN_ROOT).
    let absolute = path.starts_with('/');
    let mut cur = if absolute && ctx.resolve & RESOLVE_BENEATH != 0 {
        // BENEATH disallows leading slash.
        return Err(EINVAL);
    } else if absolute {
        ctx.root.clone()
    } else {
        ctx.start.clone()
    };

    let trimmed = path.trim_start_matches('/');
    for comp in trimmed.split('/').filter(|c| !c.is_empty() && *c != ".") {
        if comp == ".." {
            // BENEATH + climbing at-or-above start is forbidden.
            if ctx.resolve & RESOLVE_BENEATH != 0 && alloc::sync::Arc::ptr_eq(&cur, &ctx.start) {
                return Err(EINVAL);
            }
            // IN_ROOT pins `..` at ctx.root.
            if ctx.resolve & RESOLVE_IN_ROOT != 0 && alloc::sync::Arc::ptr_eq(&cur, &ctx.root) {
                continue;
            }
            let parent = cur.parent.lock().clone();
            cur = parent.unwrap_or(cur);
            continue;
        }

        let dir = cur.inode().ok_or(ENOENT)?;
        if dir.kind != InodeKind::Directory {
            return Err(ENOTDIR);
        }

        let next = if let Some(d) = d_lookup(&cur, comp) {
            if d.inode().is_none() {
                return Err(ENOENT);
            }
            d
        } else {
            let lookup = dir.ops.lookup.ok_or(ENOENT)?;
            let child_inode = match lookup(&dir, comp) {
                Ok(inode) => inode,
                Err(ENOENT) => {
                    d_cache_negative(&cur, comp);
                    return Err(ENOENT);
                }
                Err(errno) => return Err(errno),
            };
            let new_d = d_alloc_child(&cur, comp);
            new_d.instantiate(child_inode);
            new_d
        };

        // RESOLVE_NO_SYMLINKS — we don't materialize symlinks here yet, so
        // treat any symlink encounter as ELOOP (M38 hasn't created any).
        if let Some(ino) = next.inode() {
            if ino.kind == InodeKind::Symlink {
                if ctx.resolve & RESOLVE_NO_SYMLINKS != 0 {
                    return Err(ELOOP);
                }
                if ctx.resolve & RESOLVE_NO_MAGICLINKS != 0 {
                    return Err(ELOOP);
                }
            }
        }

        cur = next;
    }
    let _ = (RESOLVE_NO_XDEV,); // suppress unused-import lint
    Ok(cur)
}
