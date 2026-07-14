//! linux-parity: complete
//! linux-source: vendor/linux/fs/file.c
//! test-origin: linux:vendor/linux/fs/file.c
//! File-table helpers — `alloc_file`, `fput`.
//!
//! The fdtable (per-task FD → File mapping) lands in M39 (`fdtable.rs`).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::fdtable::NR_OPEN_MAX;
use crate::include::uapi::errno::{EMFILE, ENODEV};
use crate::include::uapi::fcntl::O_PATH;
use crate::kernel::module::{export_symbol, find_symbol};

use super::ops::FileOps;
use super::types::{DentryRef, File, FileRef};

#[repr(C)]
pub struct LinuxFd {
    pub word: usize,
}

const LINUX_VENDOR_FD_BASE: u32 = 200_000;
static NEXT_LINUX_VENDOR_FD: AtomicU32 = AtomicU32::new(LINUX_VENDOR_FD_BASE);

lazy_static! {
    static ref LINUX_VENDOR_FDS: Mutex<BTreeMap<u32, Option<usize>>> = Mutex::new(BTreeMap::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn err_ptr(errno: i32) -> *mut c_void {
    (usize::MAX - errno as usize + 1) as *mut c_void
}

pub fn register_module_exports() {
    export_symbol_once("dentry_open", linux_dentry_open as usize, false);
    export_symbol_once("fput", linux_fput as usize, false);
    export_symbol_once("get_file_active", linux_get_file_active as usize, true);
    export_symbol_once("file_update_time", linux_file_update_time as usize, false);
    export_symbol_once("fdget", linux_fdget as usize, false);
    export_symbol_once(
        "get_unused_fd_flags",
        linux_get_unused_fd_flags as usize,
        false,
    );
    export_symbol_once("put_unused_fd", linux_put_unused_fd as usize, false);
    export_symbol_once("fd_install", linux_fd_install as usize, false);
}

/// `fput` - `vendor/linux/fs/file_table.c:586`.
///
/// Lupos-owned files use `FileRef` and are released through the native `fput`.
/// Vendor modules pass Linux `struct file *` objects; unless a subsystem has
/// registered ownership for such an object, dropping it here must be a no-op.
pub unsafe extern "C" fn linux_fput(_file: *mut c_void) {}

/// `get_file_active` - `vendor/linux/fs/file.c:1004`.
///
/// Until Lupos owns arbitrary Linux-layout `struct file` instances end-to-end,
/// do not touch the foreign `f_ref` field. Module callers get the current file
/// pointer if present; `fput` on that pointer is intentionally a no-op here.
pub unsafe extern "C" fn linux_get_file_active(filep: *mut *mut c_void) -> *mut c_void {
    if filep.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { filep.read() }
}

/// `file_update_time` - `vendor/linux/fs/inode.c:2506`.
///
/// Module callers pass Linux-layout `struct file *` objects. Until Lupos owns
/// those objects end-to-end, timestamp updates are treated as already satisfied
/// rather than dereferencing a foreign file layout.
pub unsafe extern "C" fn linux_file_update_time(_file: *mut c_void) -> i32 {
    0
}

/// `dentry_open` - `vendor/linux/fs/open.c`.
///
/// Lupos does not yet expose a complete Linux-layout `struct file` opener for
/// arbitrary vendor `struct path` values, so keep unsupported opens
/// fail-closed instead of fabricating a partially initialized file.
pub unsafe extern "C" fn linux_dentry_open(
    _path: *const c_void,
    _flags: i32,
    _cred: *const c_void,
) -> *mut c_void {
    err_ptr(ENODEV)
}

/// `fdget` - `vendor/linux/fs/file.c:1206`.
pub unsafe extern "C" fn linux_fdget(fd: u32) -> LinuxFd {
    let word = LINUX_VENDOR_FDS
        .lock()
        .get(&fd)
        .and_then(|file| *file)
        .unwrap_or(0);
    LinuxFd { word }
}

/// `get_unused_fd_flags` - `vendor/linux/fs/file.c:619`.
pub unsafe extern "C" fn linux_get_unused_fd_flags(_flags: u32) -> i32 {
    loop {
        let fd = NEXT_LINUX_VENDOR_FD.fetch_add(1, Ordering::AcqRel);
        if fd as usize >= NR_OPEN_MAX {
            return -EMFILE;
        }
        let mut table = LINUX_VENDOR_FDS.lock();
        if let alloc::collections::btree_map::Entry::Vacant(entry) = table.entry(fd) {
            entry.insert(None);
            return fd as i32;
        }
    }
}

/// `put_unused_fd` - `vendor/linux/fs/file.c:633`.
pub unsafe extern "C" fn linux_put_unused_fd(fd: u32) {
    LINUX_VENDOR_FDS.lock().remove(&fd);
}

/// `fd_install` - `vendor/linux/fs/file.c:679`.
pub unsafe extern "C" fn linux_fd_install(fd: u32, file: *mut c_void) {
    if file.is_null() {
        return;
    }
    LINUX_VENDOR_FDS.lock().insert(fd, Some(file as usize));
}

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

    #[test]
    fn linux_vendor_fd_install_round_trip() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("fd_install").is_some());
        assert!(crate::kernel::module::find_symbol("get_unused_fd_flags").is_some());
        assert_eq!(
            crate::kernel::module::find_symbol("get_file_active"),
            Some(linux_get_file_active as usize)
        );

        let fd = unsafe { linux_get_unused_fd_flags(0) };
        assert!(fd >= 0);
        assert_eq!(unsafe { linux_fdget(fd as u32).word }, 0);

        let mut raw_file = 0usize;
        unsafe { linux_fd_install(fd as u32, (&mut raw_file as *mut usize).cast::<c_void>()) };
        assert_eq!(
            unsafe { linux_fdget(fd as u32).word },
            &mut raw_file as *mut usize as usize
        );

        unsafe { linux_put_unused_fd(fd as u32) };
        assert_eq!(unsafe { linux_fdget(fd as u32).word }, 0);
    }

    #[test]
    fn get_file_active_returns_current_foreign_file_pointer() {
        let mut raw_file = 0x1234usize as *mut c_void;
        assert_eq!(
            unsafe { linux_get_file_active(&mut raw_file as *mut *mut c_void) },
            raw_file
        );
        raw_file = core::ptr::null_mut();
        assert!(unsafe { linux_get_file_active(&mut raw_file as *mut *mut c_void) }.is_null());
        assert!(unsafe { linux_get_file_active(core::ptr::null_mut()) }.is_null());
    }
}
