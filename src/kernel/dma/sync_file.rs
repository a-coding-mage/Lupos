//! linux-parity: partial
//! linux-source: vendor/linux/drivers/dma-buf/sync_file.c
//! test-origin: linux:vendor/linux/drivers/dma-buf/sync_file.c
//! Sync-file dma-fence ABI used by Linux-built dma-buf and DRM modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sync_file_create", linux_sync_file_create as usize, false);
    export_symbol_once(
        "sync_file_get_fence",
        linux_sync_file_get_fence as usize,
        false,
    );
}

/// `sync_file_create` - `vendor/linux/drivers/dma-buf/sync_file.c:64`.
///
/// Lupos does not yet model Linux sync-file file operations or fd ownership.
/// Returning `NULL` keeps callers on Linux's allocation-failure path instead of
/// manufacturing a file object that cannot be validated later.
pub unsafe extern "C" fn linux_sync_file_create(_fence: *mut c_void) -> *mut c_void {
    core::ptr::null_mut()
}

/// `sync_file_get_fence` - `vendor/linux/drivers/dma-buf/sync_file.c:101`.
///
/// Linux returns `NULL` when the fd is invalid or does not reference a
/// sync-file. Until Lupos can validate vendor `struct file` instances against
/// `sync_file_fops`, every fd is treated as unmodeled.
pub unsafe extern "C" fn linux_sync_file_get_fence(_fd: i32) -> *mut c_void {
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_vendor_sync_file_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("sync_file_create"),
            Some(linux_sync_file_create as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("sync_file_get_fence"),
            Some(linux_sync_file_get_fence as usize)
        );
    }

    #[test]
    fn unmodeled_sync_files_fail_closed() {
        assert!(unsafe { linux_sync_file_create(core::ptr::null_mut()) }.is_null());
        assert!(unsafe { linux_sync_file_get_fence(-1) }.is_null());
    }
}
