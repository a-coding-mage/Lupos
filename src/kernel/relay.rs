//! linux-parity: partial
//! linux-source: vendor/linux/kernel/relay.c
//! test-origin: linux:vendor/linux/kernel/relay.c
//! Relay channel ABI used by Linux-built modules for debugfs trace buffers.
//!
//! Lupos does not yet implement relay buffer allocation or VFS dispatch for
//! raw Linux `struct file_operations`.  The exported fops object is present so
//! vendor modules can relocate and pass it through debugfs; channel creation
//! returns NULL so runtime relay use follows the caller's normal error path.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::ENODEV;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "relay_file_operations",
        core::ptr::addr_of!(RELAY_FILE_OPERATIONS) as usize,
        true,
    );
    export_symbol_once("relay_open", linux_relay_open as usize, true);
    export_symbol_once("relay_close", linux_relay_close as usize, true);
    export_symbol_once("relay_flush", linux_relay_flush as usize, true);
    export_symbol_once("relay_reset", linux_relay_reset as usize, true);
    export_symbol_once("relay_stats", linux_relay_stats as usize, true);
    export_symbol_once(
        "relay_subbufs_consumed",
        linux_relay_subbufs_consumed as usize,
        true,
    );
    export_symbol_once("relay_buf_full", linux_relay_buf_full as usize, true);
    export_symbol_once(
        "relay_switch_subbuf",
        linux_relay_switch_subbuf as usize,
        true,
    );
}

#[repr(C)]
struct LinuxFileOperations {
    owner: usize,
    fop_flags: u32,
    _pad: u32,
    llseek: usize,
    read: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut i64) -> isize>,
    write: usize,
    read_iter: usize,
    write_iter: usize,
    iopoll: usize,
    iterate_shared: usize,
    poll: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32>,
    unlocked_ioctl: usize,
    compat_ioctl: usize,
    mmap: usize,
    open: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    flush: usize,
    release: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    fsync: usize,
    fasync: usize,
    lock: usize,
    get_unmapped_area: usize,
    check_flags: usize,
    flock: usize,
    splice_write: usize,
    splice_read: usize,
    splice_eof: usize,
    setlease: usize,
    fallocate: usize,
    show_fdinfo: usize,
    copy_file_range: usize,
    remap_file_range: usize,
    fadvise: usize,
    uring_cmd: usize,
    uring_cmd_iopoll: usize,
    mmap_prepare: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
}

static RELAY_FILE_OPERATIONS: LinuxFileOperations = LinuxFileOperations {
    owner: 0,
    fop_flags: 0,
    _pad: 0,
    llseek: 0,
    read: Some(linux_relay_file_read),
    write: 0,
    read_iter: 0,
    write_iter: 0,
    iopoll: 0,
    iterate_shared: 0,
    poll: Some(linux_relay_file_poll),
    unlocked_ioctl: 0,
    compat_ioctl: 0,
    mmap: 0,
    open: Some(linux_relay_file_open),
    flush: 0,
    release: Some(linux_relay_file_release),
    fsync: 0,
    fasync: 0,
    lock: 0,
    get_unmapped_area: 0,
    check_flags: 0,
    flock: 0,
    splice_write: 0,
    splice_read: 0,
    splice_eof: 0,
    setlease: 0,
    fallocate: 0,
    show_fdinfo: 0,
    copy_file_range: 0,
    remap_file_range: 0,
    fadvise: 0,
    uring_cmd: 0,
    uring_cmd_iopoll: 0,
    mmap_prepare: Some(linux_relay_file_mmap_prepare),
};

unsafe extern "C" fn linux_relay_file_open(_inode: *mut c_void, _file: *mut c_void) -> i32 {
    -ENODEV
}

unsafe extern "C" fn linux_relay_file_release(_inode: *mut c_void, _file: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_relay_file_poll(_file: *mut c_void, _wait: *mut c_void) -> u32 {
    0
}

unsafe extern "C" fn linux_relay_file_mmap_prepare(_desc: *mut c_void) -> i32 {
    -ENODEV
}

unsafe extern "C" fn linux_relay_file_read(
    _file: *mut c_void,
    _buffer: *mut c_void,
    _count: usize,
    _ppos: *mut i64,
) -> isize {
    0
}

/// `relay_open` - `vendor/linux/kernel/relay.c:474`.
unsafe extern "C" fn linux_relay_open(
    _base_filename: *const c_char,
    _parent: *mut c_void,
    _subbuf_size: usize,
    _n_subbufs: usize,
    _cb: *const c_void,
    _private_data: *mut c_void,
) -> *mut c_void {
    core::ptr::null_mut()
}

/// `relay_close` - `vendor/linux/kernel/relay.c:649`.
unsafe extern "C" fn linux_relay_close(_chan: *mut c_void) {}

/// `relay_flush` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_flush(_chan: *mut c_void) {}

/// `relay_reset` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_reset(_chan: *mut c_void) {}

/// `relay_stats` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_stats(_chan: *mut c_void, _flags: i32) -> usize {
    0
}

/// `relay_subbufs_consumed` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_subbufs_consumed(_chan: *mut c_void, _cpu: u32, _consumed: usize) {
}

/// `relay_buf_full` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_buf_full(_buf: *mut c_void) -> i32 {
    1
}

/// `relay_switch_subbuf` - `vendor/linux/kernel/relay.c`.
unsafe extern "C" fn linux_relay_switch_subbuf(_buf: *mut c_void, _length: usize) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_module_exports_register() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("relay_file_operations"),
            Some(core::ptr::addr_of!(RELAY_FILE_OPERATIONS) as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("relay_open"),
            Some(linux_relay_open as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("relay_switch_subbuf"),
            Some(linux_relay_switch_subbuf as usize)
        );
    }

    #[test]
    fn relay_runtime_paths_fail_closed() {
        unsafe {
            assert!(
                linux_relay_open(
                    core::ptr::null(),
                    core::ptr::null_mut(),
                    4096,
                    2,
                    core::ptr::null(),
                    core::ptr::null_mut()
                )
                .is_null()
            );
            assert_eq!(linux_relay_switch_subbuf(core::ptr::null_mut(), 16), 0);
            assert_eq!(linux_relay_buf_full(core::ptr::null_mut()), 1);
        }
    }
}
