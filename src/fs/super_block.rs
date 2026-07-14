//! linux-parity: partial
//! linux-source: vendor/linux/fs
//! Filesystem-type registry and `mount_fs`.
//!
//! Mirrors `vendor/linux/fs/super.c` `register_filesystem` /
//! `get_fs_type` / `mount_fs`.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_void;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

use super::types::SuperBlockRef;

/// Filesystem mount callback — `(source, flags, data) -> SuperBlock`.
///
/// `source` is e.g. a device path, `flags` is the bitset from `MS_*`,
/// `data` is the comma-separated mount options string ("size=64M,mode=755").
pub type MountFn = fn(source: &str, flags: u64, data: &str) -> Result<SuperBlockRef, i32>;

#[derive(Clone, Copy)]
pub struct FileSystemType {
    pub name: &'static str,
    pub mount: MountFn,
    pub fs_flags: u32,
}

pub const FS_REQUIRES_DEV: u32 = 1 << 0;
pub const FS_USERNS_MOUNT: u32 = 1 << 3;

lazy_static! {
    static ref REGISTRY: Mutex<BTreeMap<String, FileSystemType>> = Mutex::new(BTreeMap::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("kill_anon_super", linux_kill_anon_super as usize, false);
}

/// `kill_anon_super` - `vendor/linux/fs/super.c`.
pub unsafe extern "C" fn linux_kill_anon_super(_sb: *mut c_void) {}

pub fn init_registry() {
    // No-op — Mutex::new in the lazy_static initializer already runs lazily.
}

pub fn register_filesystem(fst: FileSystemType) -> Result<(), i32> {
    let mut reg = REGISTRY.lock();
    if reg.contains_key(fst.name) {
        return Err(crate::include::uapi::errno::EBUSY);
    }
    reg.insert(String::from(fst.name), fst);
    Ok(())
}

pub fn lookup_filesystem(name: &str) -> Option<FileSystemType> {
    REGISTRY.lock().get(name).copied()
}

pub fn registered_filesystems() -> Vec<FileSystemType> {
    REGISTRY.lock().values().copied().collect()
}

/// Look up the named filesystem and call its `mount` callback.
pub fn mount_fs(fs_name: &str, source: &str, flags: u64, data: &str) -> Result<SuperBlockRef, i32> {
    let fst = lookup_filesystem(fs_name).ok_or(crate::include::uapi::errno::ENODEV)?;
    (fst.mount)(source, flags, data)
}

/// Diagnostics — number of registered filesystems.
pub fn registered_count() -> usize {
    REGISTRY.lock().len()
}
