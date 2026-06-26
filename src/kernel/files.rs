//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! Task file table glue (`task_struct.files`).
//!
//! The ABI-facing `TaskStruct.files` field is an opaque pointer in
//! `kernel/task.rs` to preserve the Linux layout. The real implementation lives
//! in `fs/fdtable.rs` as an `Arc<FilesStruct>`.
//!
//! This module provides the unsafe cast + lifetime management helpers that
//! bridge the two.

extern crate alloc;

use alloc::sync::Arc;

use crate::fs::fdtable::{FilesStruct as FdTable, dup_fd};
use crate::kernel::task::{FilesStruct as OpaqueFiles, TaskStruct};

#[inline]
unsafe fn raw_ptr(tsk: *mut TaskStruct) -> *const FdTable {
    unsafe { (*tsk).files as *const FdTable }
}

/// Install a new `files_struct` into `tsk`, consuming one strong reference.
///
/// # Safety
/// `tsk` must be a valid `TaskStruct` pointer.
pub unsafe fn set_task_files(tsk: *mut TaskStruct, files: Arc<FdTable>) {
    let ptr = Arc::into_raw(files) as *mut FdTable;
    unsafe {
        (*tsk).files = ptr as *mut OpaqueFiles;
    }
}

/// Borrow `tsk->files` as an owning `Arc`.
///
/// This function increments the strong count and returns a new `Arc`.
///
/// # Safety
/// `tsk` must be valid. The caller must not mutate `tsk->files` concurrently.
pub unsafe fn get_task_files(tsk: *mut TaskStruct) -> Option<Arc<FdTable>> {
    if tsk.is_null() {
        return None;
    }
    let ptr = unsafe { raw_ptr(tsk) };
    if ptr.is_null() {
        return None;
    }
    unsafe {
        Arc::increment_strong_count(ptr);
        Some(Arc::from_raw(ptr))
    }
}

/// Drop one reference to `tsk->files` and NULL the pointer.
///
/// # Safety
/// `tsk` must be valid. After this call, `tsk->files` is NULL.
pub unsafe fn drop_task_files(tsk: *mut TaskStruct) {
    if tsk.is_null() {
        return;
    }
    let ptr = unsafe { raw_ptr(tsk) };
    if ptr.is_null() {
        return;
    }
    unsafe {
        (*tsk).files = core::ptr::null_mut();
        drop(Arc::from_raw(ptr));
    }
}

/// Duplicate a parent's `files_struct` for a newly created child.
///
/// Mirrors Linux's `copy_files()`: `CLONE_FILES` shares the table; otherwise
/// clone an independent copy.
///
/// # Safety
/// `child` and `parent` must be valid pointers.
pub unsafe fn copy_files(child: *mut TaskStruct, parent: *mut TaskStruct, clone_files: bool) {
    unsafe {
        (*child).files = core::ptr::null_mut();
    }
    let Some(parent_arc) = (unsafe { get_task_files(parent) }) else {
        return;
    };
    let new = dup_fd(&parent_arc, clone_files);
    unsafe {
        set_task_files(child, new);
    }
}
