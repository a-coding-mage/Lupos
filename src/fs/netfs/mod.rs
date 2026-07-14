//! linux-parity: partial
//! linux-source: vendor/linux/fs/netfs
//! Netfs and FS-Cache source coverage.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

pub mod fscache_main;
pub mod fscache_proc;
pub mod fscache_stats;
pub mod stats;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "netfs_write_subrequest_terminated",
        linux_netfs_write_subrequest_terminated as usize,
        false,
    );
}

/// `netfs_write_subrequest_terminated` - `vendor/linux/fs/netfs/write_collect.c`.
pub unsafe extern "C" fn linux_netfs_write_subrequest_terminated(
    _op: *mut c_void,
    _transferred_or_error: isize,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netfs_write_subrequest_terminated_export_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/netfs/write_collect.c"
        ));
        assert!(source.contains("void netfs_write_subrequest_terminated"));
        assert!(source.contains("EXPORT_SYMBOL(netfs_write_subrequest_terminated);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("netfs_write_subrequest_terminated"),
            Some(linux_netfs_write_subrequest_terminated as usize)
        );
    }
}
