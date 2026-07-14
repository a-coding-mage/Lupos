//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/group.c
//! sysfs attribute group helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/group.c`

use alloc::sync::Arc;
use core::ffi::c_void;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "sysfs_create_group",
        linux_sysfs_create_group as usize,
        true,
    );
    export_symbol_once(
        "sysfs_remove_group",
        linux_sysfs_remove_group as usize,
        true,
    );
    export_symbol_once(
        "sysfs_update_group",
        linux_sysfs_update_group as usize,
        true,
    );
    export_symbol_once("sysfs_merge_group", linux_sysfs_merge_group as usize, true);
}

pub fn create_group(parent: &Arc<KernfsNode>, name: &str) -> Arc<KernfsNode> {
    super::dir::create_dir(parent, name)
}

/// `sysfs_create_group` - `vendor/linux/fs/sysfs/group.c`.
///
/// Lupos does not yet publish Linux-built attribute callbacks through kernfs.
/// Accept well-formed groups so driver probe/sysfs setup paths can continue.
#[unsafe(export_name = "sysfs_create_group")]
pub unsafe extern "C" fn linux_sysfs_create_group(kobj: *mut c_void, group: *const c_void) -> i32 {
    if kobj.is_null() || group.is_null() {
        -EINVAL
    } else {
        0
    }
}

/// `sysfs_remove_group` - `vendor/linux/fs/sysfs/group.c`.
#[unsafe(export_name = "sysfs_remove_group")]
pub unsafe extern "C" fn linux_sysfs_remove_group(_kobj: *mut c_void, _group: *const c_void) {}

/// `sysfs_update_group` - `vendor/linux/fs/sysfs/group.c:295`.
pub unsafe extern "C" fn linux_sysfs_update_group(kobj: *mut c_void, group: *const c_void) -> i32 {
    unsafe { linux_sysfs_create_group(kobj, group) }
}

/// `sysfs_merge_group` - `vendor/linux/fs/sysfs/group.c:365`.
pub unsafe extern "C" fn linux_sysfs_merge_group(kobj: *mut c_void, group: *const c_void) -> i32 {
    if kobj.is_null() || group.is_null() {
        -EINVAL
    } else {
        0
    }
}
