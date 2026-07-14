//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sysctl.c
//! Sysctl handler ABI exports used by Linux-built modules.

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::{c_char, c_void};

use crate::kernel::module::{export_symbol, find_symbol};

pub const DEFAULT_OVERFLOWUID: i32 = 65_534;
pub const DEFAULT_OVERFLOWGID: i32 = 65_534;
pub const DEFAULT_FS_OVERFLOWUID: i32 = 65_534;
pub const DEFAULT_FS_OVERFLOWGID: i32 = 65_534;

static mut LINUX_OVERFLOWUID: i32 = DEFAULT_OVERFLOWUID;
static mut LINUX_OVERFLOWGID: i32 = DEFAULT_OVERFLOWGID;
static mut LINUX_FS_OVERFLOWUID: i32 = DEFAULT_FS_OVERFLOWUID;
static mut LINUX_FS_OVERFLOWGID: i32 = DEFAULT_FS_OVERFLOWGID;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

#[repr(C)]
struct SysctlHeader {
    path: *const c_char,
    table: *const c_void,
    table_size: usize,
}

const SYSCTL_VALS: [i32; 12] = [0, 1, 2, 3, 4, 100, 200, 1000, 3000, i32::MAX, 65535, -1];

pub fn register_module_exports() {
    unsafe {
        export_symbol_once(
            "overflowuid",
            core::ptr::addr_of_mut!(LINUX_OVERFLOWUID) as usize,
            false,
        );
        export_symbol_once(
            "overflowgid",
            core::ptr::addr_of_mut!(LINUX_OVERFLOWGID) as usize,
            false,
        );
        export_symbol_once(
            "fs_overflowuid",
            core::ptr::addr_of_mut!(LINUX_FS_OVERFLOWUID) as usize,
            false,
        );
        export_symbol_once(
            "fs_overflowgid",
            core::ptr::addr_of_mut!(LINUX_FS_OVERFLOWGID) as usize,
            false,
        );
    }
    export_symbol_once(
        "proc_dointvec_minmax",
        linux_proc_dointvec_minmax as usize,
        false,
    );
    export_symbol_once("proc_dointvec", linux_proc_dointvec as usize, false);
    export_symbol_once("proc_dostring", linux_proc_dostring as usize, false);
    export_symbol_once(
        "register_sysctl_sz",
        linux_register_sysctl_sz as usize,
        false,
    );
    export_symbol_once(
        "unregister_sysctl_table",
        linux_unregister_sysctl_table as usize,
        false,
    );
    export_symbol_once("sysctl_vals", SYSCTL_VALS.as_ptr() as usize, false);
}

pub fn overflowuid() -> i32 {
    unsafe { LINUX_OVERFLOWUID }
}

pub fn overflowgid() -> i32 {
    unsafe { LINUX_OVERFLOWGID }
}

/// `proc_dointvec_minmax` - `vendor/linux/kernel/sysctl.c`.
pub unsafe extern "C" fn linux_proc_dointvec_minmax(
    _table: *const c_void,
    _write: i32,
    _buffer: *mut c_void,
    _lenp: *mut usize,
    _ppos: *mut i64,
) -> i32 {
    0
}

/// `proc_dointvec` - `vendor/linux/kernel/sysctl.c`.
pub unsafe extern "C" fn linux_proc_dointvec(
    table: *const c_void,
    write: i32,
    buffer: *mut c_void,
    lenp: *mut usize,
    ppos: *mut i64,
) -> i32 {
    unsafe { linux_proc_dointvec_minmax(table, write, buffer, lenp, ppos) }
}

/// `proc_dostring` - `vendor/linux/kernel/sysctl.c`.
pub unsafe extern "C" fn linux_proc_dostring(
    _table: *const c_void,
    _write: i32,
    _buffer: *mut c_void,
    _lenp: *mut usize,
    _ppos: *mut i64,
) -> i32 {
    0
}

/// `register_sysctl_sz` - `vendor/linux/fs/proc/proc_sysctl.c`.
pub unsafe extern "C" fn linux_register_sysctl_sz(
    path: *const c_char,
    table: *const c_void,
    table_size: usize,
) -> *mut c_void {
    Box::into_raw(Box::new(SysctlHeader {
        path,
        table,
        table_size,
    }))
    .cast::<c_void>()
}

/// `unregister_sysctl_table` - `vendor/linux/fs/proc/proc_sysctl.c`.
pub unsafe extern "C" fn linux_unregister_sysctl_table(header: *mut c_void) {
    if !header.is_null() {
        unsafe {
            drop(Box::from_raw(header.cast::<SysctlHeader>()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysctl_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("overflowuid"),
            Some(core::ptr::addr_of!(LINUX_OVERFLOWUID) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("overflowgid"),
            Some(core::ptr::addr_of!(LINUX_OVERFLOWGID) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("fs_overflowuid"),
            Some(core::ptr::addr_of!(LINUX_FS_OVERFLOWUID) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("fs_overflowgid"),
            Some(core::ptr::addr_of!(LINUX_FS_OVERFLOWGID) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("proc_dointvec_minmax"),
            Some(linux_proc_dointvec_minmax as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("proc_dointvec"),
            Some(linux_proc_dointvec as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("proc_dostring"),
            Some(linux_proc_dostring as usize)
        );
        assert!(crate::kernel::module::find_symbol("register_sysctl_sz").is_some());
        assert!(crate::kernel::module::find_symbol("sysctl_vals").is_some());
    }

    #[test]
    fn overflow_exports_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/sys.c"
        ));
        assert!(source.contains("int overflowuid = DEFAULT_OVERFLOWUID;"));
        assert!(source.contains("int overflowgid = DEFAULT_OVERFLOWGID;"));
        assert!(source.contains("EXPORT_SYMBOL(overflowuid);"));
        assert!(source.contains("EXPORT_SYMBOL(overflowgid);"));
        assert!(source.contains("int fs_overflowuid = DEFAULT_FS_OVERFLOWUID;"));
        assert!(source.contains("int fs_overflowgid = DEFAULT_FS_OVERFLOWGID;"));
        assert!(source.contains("EXPORT_SYMBOL(fs_overflowuid);"));
        assert!(source.contains("EXPORT_SYMBOL(fs_overflowgid);"));

        assert_eq!(core::mem::size_of_val(&DEFAULT_OVERFLOWUID), 4);
        assert_eq!(overflowuid(), 65_534);
        assert_eq!(overflowgid(), 65_534);
    }
}
