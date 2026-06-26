//! linux-parity: complete
//! linux-source: vendor/linux/fs/file.c
//! test-origin: linux:vendor/linux/fs/file.c
//! File-table helpers — `alloc_file`, `fput`.
//!
//! The fdtable (per-task FD → File mapping) lands in M39 (`fdtable.rs`).

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::include::uapi::fcntl::O_PATH;

use super::ops::FileOps;
use super::types::{DentryRef, File, FileRef};

/// Allocate a `File` for an opened dentry.
pub fn alloc_file(dentry: DentryRef, flags: u32, mode: u32, fops: &'static FileOps) -> FileRef {
    super::file_table::account_allocated_file();
    File::new(dentry, flags, mode, fops)
}

pub fn set_path_hint(file: &FileRef, path: String) {
    *file.path_hint.lock() = Some(normalize_path_hint(path));
}

pub fn path_hint(file: &FileRef) -> Option<String> {
    file.path_hint.lock().clone()
}

fn note_file_access_for_integrity_hook(
    hook: crate::security::integrity::ima::ImaHook,
    path: Option<&str>,
    file: &FileRef,
) {
    if file.flags.load(Ordering::Acquire) & O_PATH != 0 {
        return;
    }
    let Some(inode) = file.inode() else {
        return;
    };
    if !inode.is_reg() {
        return;
    }
    let path = path
        .map(String::from)
        .or_else(|| path_hint(file))
        .unwrap_or_else(|| file_path(file));
    let _ = match hook {
        crate::security::integrity::ima::ImaHook::MmapCheck => {
            crate::security::integrity::ima::measure_mapped_inode(&path, &inode)
        }
        _ => crate::security::integrity::ima::measure_inode_private_for_hook(
            hook,
            &path,
            &inode.private,
        ),
    };
}

pub fn note_file_access_for_integrity(path: Option<&str>, file: &FileRef) {
    note_file_access_for_integrity_hook(
        crate::security::integrity::ima::ImaHook::FileCheck,
        path,
        file,
    );
}

pub fn note_file_mmap_for_integrity(path: Option<&str>, file: &FileRef) {
    note_file_access_for_integrity_hook(
        crate::security::integrity::ima::ImaHook::MmapCheck,
        path,
        file,
    );
}

fn normalize_path_hint(mut path: String) -> String {
    while path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    path
}

pub fn fget(f: &FileRef) -> FileRef {
    f.f_count.fetch_add(1, Ordering::AcqRel);
    f.clone()
}

pub fn fput(f: FileRef) {
    f.f_count.fetch_sub(1, Ordering::AcqRel);
    if let Some(release) = f.fops.release {
        if Arc::strong_count(&f) == 1 {
            super::file_table::account_released_file();
            release(f);
            return;
        }
    }
    if Arc::strong_count(&f) == 1 {
        super::file_table::account_released_file();
    }
    drop(f);
}

pub fn f_strong_count(f: &FileRef) -> usize {
    Arc::strong_count(f)
}

pub fn file_path(f: &FileRef) -> String {
    dentry_path(&f.dentry)
}

pub fn dentry_path(dentry: &DentryRef) -> String {
    let mut components = Vec::new();
    let mut cur = Some(dentry.clone());

    while let Some(node) = cur {
        let parent = node.parent.lock().clone();
        let is_root = parent.is_none();
        if !is_root && node.name != "/" && !node.name.is_empty() {
            components.push(node.name.clone());
        }
        cur = parent;
    }

    if components.is_empty() {
        return String::from("/");
    }

    let mut path = String::new();
    for component in components.iter().rev() {
        path.push('/');
        path.push_str(component);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::{d_alloc, d_alloc_child};
    use crate::fs::ops::NOOP_FILE_OPS;

    #[test]
    fn file_path_uses_dentry_parent_chain() {
        let root = d_alloc("/");
        let tmp = d_alloc_child(&root, "tmp");
        let file = d_alloc_child(&tmp, "x");
        let f = alloc_file(file, 0, 0, &NOOP_FILE_OPS);
        assert_eq!(file_path(&f), "/tmp/x");
    }
}
