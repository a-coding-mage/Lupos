//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/file.c
//! sysfs file helpers.
//!
//! Ref: `vendor/linux/fs/sysfs/file.c`

use alloc::sync::Arc;
use core::ffi::c_char;

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
