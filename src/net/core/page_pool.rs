//! linux-parity: partial
//! linux-source: vendor/linux/net/core/page_pool.c
//! Page-pool ABI exports for Linux-built network modules.

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::c_void;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page::Page;
use crate::mm::page_alloc::alloc_pages_noprof;
use crate::mm::page_flags::GfpFlags;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxPagePoolParamsFast {
    order: u32,
    pool_size: u32,
    nid: i32,
    dev: *mut c_void,
    napi: *mut c_void,
    dma_dir: i32,
    max_len: u32,
    offset: u32,
}

#[repr(C, align(32))]
struct LinuxPagePool {
    p: LinuxPagePoolParamsFast,
    cpuid: i32,
    pages_state_hold_cnt: u32,
    flags: u8,
    _pad_to_frag: [u8; 7],
    frag_users: isize,
    frag_page: usize,
    frag_offset: u32,
}

const MAX_ERRNO: usize = 4095;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

fn is_err_ptr(ptr: *mut c_void) -> bool {
    (ptr as usize) >= usize::MAX - MAX_ERRNO + 1
}

pub fn register_module_exports() {
    export_symbol_once("page_pool_create", page_pool_create as usize, true);
    export_symbol_once("page_pool_destroy", page_pool_destroy as usize, true);
    export_symbol_once(
        "page_pool_alloc_pages",
        page_pool_alloc_pages as usize,
        true,
    );
    export_symbol_once(
        "page_pool_alloc_netmems",
        page_pool_alloc_netmems as usize,
        true,
    );
    export_symbol_once(
        "page_pool_alloc_frag_netmem",
        page_pool_alloc_frag_netmem as usize,
        true,
    );
    export_symbol_once(
        "page_pool_put_unrefed_netmem",
        page_pool_put_unrefed_netmem as usize,
        true,
    );
}

/// `page_pool_create` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_create")]
pub unsafe extern "C" fn page_pool_create(params: *const c_void) -> *mut c_void {
    if params.is_null() {
        return err_ptr(EINVAL);
    }

    let fast = unsafe { *(params.cast::<LinuxPagePoolParamsFast>()) };
    Box::into_raw(Box::new(LinuxPagePool {
        p: fast,
        cpuid: -1,
        pages_state_hold_cnt: 0,
        flags: 0,
        _pad_to_frag: [0; 7],
        frag_users: 0,
        frag_page: 0,
        frag_offset: 0,
    }))
    .cast()
}

/// `page_pool_destroy` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_destroy")]
pub unsafe extern "C" fn page_pool_destroy(pool: *mut c_void) {
    if pool.is_null() || is_err_ptr(pool) {
        return;
    }
    unsafe {
        drop(Box::from_raw(pool.cast::<LinuxPagePool>()));
    }
}

/// `page_pool_alloc_pages` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_alloc_pages")]
pub unsafe extern "C" fn page_pool_alloc_pages(_pool: *mut c_void, gfp: GfpFlags) -> *mut Page {
    alloc_pages_noprof(gfp, 0)
}

/// `page_pool_alloc_netmems` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_alloc_netmems")]
pub unsafe extern "C" fn page_pool_alloc_netmems(_pool: *mut c_void, gfp: GfpFlags) -> usize {
    page_pool_alloc_pages(_pool, gfp) as usize
}

/// `page_pool_alloc_frag_netmem` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_alloc_frag_netmem")]
pub unsafe extern "C" fn page_pool_alloc_frag_netmem(
    pool: *mut c_void,
    offset: *mut u32,
    _size: u32,
    gfp: GfpFlags,
) -> usize {
    if !offset.is_null() {
        unsafe {
            *offset = 0;
        }
    }
    unsafe { page_pool_alloc_netmems(pool, gfp) }
}

/// `page_pool_put_unrefed_netmem` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_put_unrefed_netmem")]
pub unsafe extern "C" fn page_pool_put_unrefed_netmem(
    _pool: *mut c_void,
    _netmem: usize,
    _dma_sync_size: u32,
    _allow_direct: bool,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn page_pool_prefix_layout_matches_inline_users() {
        assert_eq!(offset_of!(LinuxPagePool, p), 0);
        assert_eq!(offset_of!(LinuxPagePool, cpuid), 48);
        assert_eq!(offset_of!(LinuxPagePool, pages_state_hold_cnt), 52);
        assert_eq!(offset_of!(LinuxPagePool, flags), 56);
        assert_eq!(offset_of!(LinuxPagePool, frag_users), 64);
        assert_eq!(offset_of!(LinuxPagePool, frag_page), 72);
        assert_eq!(offset_of!(LinuxPagePool, frag_offset), 80);
        assert_eq!(size_of::<LinuxPagePoolParamsFast>(), 48);
        assert_eq!(align_of::<LinuxPagePool>(), 32);
    }
}
