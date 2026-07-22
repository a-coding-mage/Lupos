//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/file.c
//! test-origin: linux:vendor/linux/fs/sysfs/file.c
//! sysfs file helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/file.c`

use alloc::sync::Arc;
use core::ffi::{c_char, c_void};

use crate::fs::kernfs::{KernfsNode, ShowFn, StoreFn, add_child};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::frame::PAGE_SIZE;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sysfs_emit", linux_sysfs_emit as usize, true);
    export_symbol_once("sysfs_emit_at", linux_sysfs_emit_at as usize, true);
    export_symbol_once(
        "sysfs_create_file_ns",
        linux_sysfs_create_file_ns as usize,
        true,
    );
    export_symbol_once(
        "sysfs_remove_file_ns",
        linux_sysfs_remove_file_ns as usize,
        true,
    );
    export_symbol_once("sysfs_chmod_file", linux_sysfs_chmod_file as usize, true);
    export_symbol_once(
        "sysfs_create_files",
        linux_sysfs_create_files as usize,
        true,
    );
    export_symbol_once(
        "sysfs_create_bin_file",
        linux_sysfs_create_bin_file as usize,
        true,
    );
    export_symbol_once(
        "sysfs_remove_bin_file",
        linux_sysfs_remove_bin_file as usize,
        true,
    );
    export_symbol_once(
        "sysfs_add_file_to_group",
        linux_sysfs_add_file_to_group as usize,
        true,
    );
    export_symbol_once(
        "sysfs_remove_file_from_group",
        linux_sysfs_remove_file_from_group as usize,
        true,
    );
    export_symbol_once(
        "sysfs_break_active_protection",
        linux_sysfs_break_active_protection as usize,
        true,
    );
    export_symbol_once(
        "sysfs_unbreak_active_protection",
        linux_sysfs_unbreak_active_protection as usize,
        true,
    );
}

/// `sysfs_emit` — `vendor/linux/fs/sysfs/file.c:751`.
///
/// The assembly wrapper preserves the native x86-64 C-varargs ordering and
/// passes it to the shared vendor-module formatter. As in Linux, `buf` must be
/// the start of a page and output is `vscnprintf(..., PAGE_SIZE, ...)`.
#[unsafe(naked)]
#[unsafe(export_name = "sysfs_emit")]
pub unsafe extern "C" fn linux_sysfs_emit() {
    core::arch::naked_asm!(
        "sub rsp, 40",
        "mov qword ptr [rsp], rdx",
        "mov qword ptr [rsp + 8], rcx",
        "mov qword ptr [rsp + 16], r8",
        "mov qword ptr [rsp + 24], r9",
        "lea rdx, [rsp]",
        "lea rcx, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym linux_sysfs_emit_helper,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_sysfs_emit_helper(
    buf: *mut c_char,
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> i32 {
    if buf.is_null() || (buf as usize & (PAGE_SIZE - 1)) != 0 {
        crate::log_warn!("sysfs", "invalid sysfs_emit: buf:{:p}", buf);
        return 0;
    }

    unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf(
            buf.cast::<u8>(),
            PAGE_SIZE,
            fmt,
            register_args,
            stack_args,
        ) as i32
    }
}

/// `sysfs_emit_at` — `vendor/linux/fs/sysfs/file.c:779`.
#[unsafe(naked)]
#[unsafe(export_name = "sysfs_emit_at")]
pub unsafe extern "C" fn linux_sysfs_emit_at() {
    core::arch::naked_asm!(
        "sub rsp, 40",
        "mov qword ptr [rsp], rcx",
        "mov qword ptr [rsp + 8], r8",
        "mov qword ptr [rsp + 16], r9",
        "lea rcx, [rsp]",
        "lea r8, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym linux_sysfs_emit_at_helper,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_sysfs_emit_at_helper(
    buf: *mut c_char,
    at: i32,
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> i32 {
    if buf.is_null() || (buf as usize & (PAGE_SIZE - 1)) != 0 || at < 0 || at >= PAGE_SIZE as i32 {
        crate::log_warn!("sysfs", "invalid sysfs_emit_at: buf:{:p} at:{}", buf, at);
        return 0;
    }

    let at = at as usize;
    unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf(
            buf.add(at).cast::<u8>(),
            PAGE_SIZE - at,
            fmt,
            register_args,
            stack_args,
        ) as i32
    }
}

/// `sysfs_create_file_ns` - `vendor/linux/fs/sysfs/file.c:367`.
pub unsafe extern "C" fn linux_sysfs_create_file_ns(
    kobj: *mut c_void,
    attr: *const c_void,
    _ns: *const c_void,
) -> i32 {
    if kobj.is_null() || attr.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `sysfs_remove_file_ns` - `vendor/linux/fs/sysfs/file.c:510`.
#[unsafe(export_name = "sysfs_remove_file_ns")]
pub unsafe extern "C" fn linux_sysfs_remove_file_ns(
    _kobj: *mut c_void,
    _attr: *const c_void,
    _ns: *const c_void,
) {
}

/// `sysfs_chmod_file` - `vendor/linux/fs/sysfs/file.c:435`.
pub unsafe extern "C" fn linux_sysfs_chmod_file(
    kobj: *mut c_void,
    attr: *const c_void,
    _mode: u16,
) -> i32 {
    if kobj.is_null() || attr.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `sysfs_create_files` - `vendor/linux/fs/sysfs/file.c:381`.
pub unsafe extern "C" fn linux_sysfs_create_files(
    kobj: *mut c_void,
    ptr: *const *const c_void,
) -> i32 {
    if kobj.is_null() || ptr.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `sysfs_create_bin_file` - `vendor/linux/fs/sysfs/file.c:582`.
pub unsafe extern "C" fn linux_sysfs_create_bin_file(
    kobj: *mut c_void,
    attr: *const c_void,
) -> i32 {
    if kobj.is_null() || attr.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `sysfs_remove_bin_file` - `vendor/linux/fs/sysfs/file.c:602`.
pub unsafe extern "C" fn linux_sysfs_remove_bin_file(_kobj: *mut c_void, _attr: *const c_void) {}

/// `sysfs_add_file_to_group` - `vendor/linux/fs/sysfs/file.c:401`.
///
/// Lupos does not expose Linux-built driver attribute methods through kernfs
/// yet.  Accept well-formed add requests so probe paths can continue, but do
/// not publish callable vendor attribute callbacks.
#[unsafe(export_name = "sysfs_add_file_to_group")]
pub unsafe extern "C" fn linux_sysfs_add_file_to_group(
    kobj: *mut c_void,
    attr: *const c_void,
    _group: *const c_char,
) -> i32 {
    if kobj.is_null() || attr.is_null() {
        -crate::include::uapi::errno::EINVAL
    } else {
        0
    }
}

/// `sysfs_remove_file_from_group` - `vendor/linux/fs/sysfs/file.c:558`.
#[unsafe(export_name = "sysfs_remove_file_from_group")]
pub unsafe extern "C" fn linux_sysfs_remove_file_from_group(
    _kobj: *mut c_void,
    _attr: *const c_void,
    _group: *const c_char,
) {
}

pub fn create_file(
    parent: &Arc<KernfsNode>,
    name: &str,
    mode: u32,
    show: Option<ShowFn>,
    store: Option<StoreFn>,
) -> Arc<KernfsNode> {
    let file = KernfsNode::new_file(name, mode, show, store);
    add_child(parent, file.clone());
    file
}

/// `sysfs_break_active_protection` - `vendor/linux/fs/sysfs/file.c`.
///
/// Lupos does not yet keep per-attribute kernfs active references for
/// Linux-built modules, so return an opaque non-null cookie only when both
/// inputs are present.  The paired unbreak helper accepts and drops that cookie.
pub unsafe extern "C" fn linux_sysfs_break_active_protection(
    kobj: *mut c_void,
    attr: *const c_void,
) -> *mut c_void {
    if kobj.is_null() || attr.is_null() {
        core::ptr::null_mut()
    } else {
        attr.cast_mut()
    }
}

/// `sysfs_unbreak_active_protection` - `vendor/linux/fs/sysfs/file.c`.
pub unsafe extern "C" fn linux_sysfs_unbreak_active_protection(_kn: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysfs_file_exports_track_vendor_symbols() {
        let source = include_str!("../../../vendor/linux/fs/sysfs/file.c");
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_create_file_ns);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(sysfs_remove_file_ns);"));
        register_module_exports();
        assert_eq!(
            find_symbol("sysfs_create_file_ns"),
            Some(linux_sysfs_create_file_ns as usize)
        );
        assert_eq!(
            find_symbol("sysfs_remove_file_ns"),
            Some(linux_sysfs_remove_file_ns as usize)
        );
    }
}
