//! linux-parity: partial
//! linux-source: vendor/linux/net/core/page_pool.c
//! Page-pool ABI exports for Linux-built network modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicI64, AtomicU32, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page::Page;
use crate::mm::page_alloc::{__free_pages, alloc_pages_noprof};
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

#[repr(C, align(64))]
struct LinuxPagePool {
    bytes: [u8; 1664],
}

struct PagePoolState {
    pool: usize,
    cached_pages: Vec<usize>,
}

static PAGE_POOLS: Mutex<Vec<PagePoolState>> = Mutex::new(Vec::new());

const PAGE_POOL_PARAMS_SLOW_OFFSET: usize = 48;
const PAGE_POOL_PARAMS_SLOW_SIZE: usize = 32;
const PAGE_POOL_SLOW_OFFSET: usize = 1584;
const PAGE_POOL_USER_COUNT_OFFSET: usize = 1572;
const PAGE_POOL_FLAGS_OFFSET: usize = 56;
const PAGE_POOL_HOLD_COUNT_OFFSET: usize = 52;
const PAGE_POOL_FRAG_USERS_OFFSET: usize = 64;
const PAGE_POOL_FRAG_PAGE_OFFSET: usize = 72;
const PAGE_POOL_FRAG_OFFSET_OFFSET: usize = 80;

const PP_FLAG_DMA_MAP: u32 = 1 << 0;
const PP_FLAG_DMA_SYNC_DEV: u32 = 1 << 1;
const PP_SIGNATURE: usize = 0xdead_0000_0000_0040;

const PAGE_PP_MAGIC_OFFSET: usize = 8;
const PAGE_PP_OFFSET: usize = 16;
const PAGE_DMA_ADDR_OFFSET: usize = 32;
const PAGE_PP_REF_COUNT_OFFSET: usize = 40;
const LINUX_PAGE_REF_COUNT_OFFSET: usize = 52;

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

    let raw = unsafe {
        crate::mm::slab::kzalloc_noprof(
            core::mem::size_of::<LinuxPagePool>(),
            crate::mm::page_flags::GFP_KERNEL,
        )
    };
    if raw.is_null() || (raw as usize) & 63 != 0 {
        if !raw.is_null() {
            unsafe { crate::mm::slab::kfree(raw) };
        }
        return err_ptr(crate::include::uapi::errno::ENOMEM);
    }
    let params = params.cast::<u8>();
    let flags = unsafe { params.add(60).cast::<u32>().read_unaligned() };
    let init_callback = unsafe { params.add(64).cast::<usize>().read_unaligned() };
    unsafe {
        core::ptr::copy_nonoverlapping(params, raw, 48);
        core::ptr::copy_nonoverlapping(
            params.add(PAGE_POOL_PARAMS_SLOW_OFFSET),
            raw.add(PAGE_POOL_SLOW_OFFSET),
            PAGE_POOL_PARAMS_SLOW_SIZE,
        );
        raw.add(48).cast::<i32>().write_unaligned(-1);
        raw.add(PAGE_POOL_FLAGS_OFFSET).write(
            u8::from(init_callback != 0)
                | (u8::from(flags & PP_FLAG_DMA_MAP != 0) << 1)
                | (u8::from(flags & PP_FLAG_DMA_SYNC_DEV != 0) << 2),
        );
        raw.add(PAGE_POOL_FRAG_USERS_OFFSET)
            .cast::<i64>()
            .write_unaligned(0);
        raw.add(PAGE_POOL_FRAG_PAGE_OFFSET)
            .cast::<usize>()
            .write_unaligned(0);
        raw.add(PAGE_POOL_FRAG_OFFSET_OFFSET)
            .cast::<u32>()
            .write_unaligned(0);
        raw.add(PAGE_POOL_USER_COUNT_OFFSET)
            .cast::<i32>()
            .write_unaligned(1);
    }
    PAGE_POOLS.lock().push(PagePoolState {
        pool: raw as usize,
        cached_pages: Vec::new(),
    });
    raw.cast()
}

/// `page_pool_destroy` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_destroy")]
pub unsafe extern "C" fn page_pool_destroy(pool: *mut c_void) {
    if pool.is_null() || is_err_ptr(pool) {
        return;
    }
    let cached = {
        let mut pools = PAGE_POOLS.lock();
        let Some(index) = pools.iter().position(|state| state.pool == pool as usize) else {
            return;
        };
        pools.swap_remove(index).cached_pages
    };
    for page in cached {
        let page = page as *mut Page;
        unsafe {
            let raw = page.cast::<u8>();
            raw.add(PAGE_PP_MAGIC_OFFSET)
                .cast::<usize>()
                .write_unaligned(0);
            raw.add(PAGE_PP_OFFSET).cast::<usize>().write_unaligned(0);
            __free_pages(page, 0);
        }
    }
    unsafe { crate::mm::slab::kfree(pool.cast()) };
}

/// `page_pool_alloc_pages` — `vendor/linux/net/core/page_pool.c`.
#[unsafe(export_name = "page_pool_alloc_pages")]
pub unsafe extern "C" fn page_pool_alloc_pages(pool: *mut c_void, gfp: GfpFlags) -> *mut Page {
    if pool.is_null() || is_err_ptr(pool) {
        return core::ptr::null_mut();
    }
    let (order, dev, dma_dir, dma_map) = unsafe {
        let raw = pool.cast::<u8>();
        (
            raw.cast::<u32>().read_unaligned(),
            raw.add(16).cast::<*mut c_void>().read_unaligned(),
            raw.add(32).cast::<u32>().read_unaligned(),
            raw.add(PAGE_POOL_FLAGS_OFFSET).read() & (1 << 1) != 0,
        )
    };
    let cached = {
        let mut pools = PAGE_POOLS.lock();
        pools
            .iter_mut()
            .find(|state| state.pool == pool as usize)
            .and_then(|state| state.cached_pages.pop())
    };
    let page = cached
        .map(|page| page as *mut Page)
        .unwrap_or_else(|| alloc_pages_noprof(gfp, order));
    if page.is_null() {
        return page;
    }
    let direction = match dma_dir {
        0 => crate::kernel::dma::DmaDirection::Bidirectional,
        1 => crate::kernel::dma::DmaDirection::ToDevice,
        2 => crate::kernel::dma::DmaDirection::FromDevice,
        _ => {
            unsafe { __free_pages(page, order) };
            return core::ptr::null_mut();
        }
    };
    let dma = if dma_map {
        unsafe {
            crate::kernel::dma::dma_map_page_attrs(
                dev,
                page,
                0,
                crate::mm::frame::PAGE_SIZE << order,
                direction,
                0,
            )
        }
    } else {
        0
    };
    if dma_map && dma == crate::kernel::dma::DMA_MAPPING_ERROR {
        unsafe { __free_pages(page, order) };
        return core::ptr::null_mut();
    }
    unsafe {
        let raw = page.cast::<u8>();
        raw.add(PAGE_PP_MAGIC_OFFSET)
            .cast::<usize>()
            .write_unaligned(PP_SIGNATURE);
        raw.add(PAGE_PP_OFFSET)
            .cast::<usize>()
            .write_unaligned(pool as usize);
        raw.add(PAGE_DMA_ADDR_OFFSET)
            .cast::<u64>()
            .write_unaligned(dma);
        (&*raw.add(PAGE_PP_REF_COUNT_OFFSET).cast::<AtomicI64>()).store(1, Ordering::Release);
        (&*raw.add(LINUX_PAGE_REF_COUNT_OFFSET).cast::<AtomicU32>()).store(1, Ordering::Release);
        let hold = &*pool
            .cast::<u8>()
            .add(PAGE_POOL_HOLD_COUNT_OFFSET)
            .cast::<AtomicU32>();
        hold.fetch_add(1, Ordering::AcqRel);
    }
    page
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
    pool: *mut c_void,
    netmem: usize,
    _dma_sync_size: u32,
    _allow_direct: bool,
) {
    let page = (netmem & !3) as *mut Page;
    if pool.is_null() || page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
        return;
    }
    let pool_size = unsafe { pool.cast::<u8>().add(4).cast::<u32>().read_unaligned() } as usize;
    unsafe {
        (&*page
            .cast::<u8>()
            .add(PAGE_PP_REF_COUNT_OFFSET)
            .cast::<AtomicI64>())
            .store(1, Ordering::Release);
        (&*page
            .cast::<u8>()
            .add(LINUX_PAGE_REF_COUNT_OFFSET)
            .cast::<AtomicU32>())
            .store(1, Ordering::Release);
    }
    let mut pools = PAGE_POOLS.lock();
    if let Some(state) = pools.iter_mut().find(|state| state.pool == pool as usize) {
        if state.cached_pages.len() < pool_size && !state.cached_pages.contains(&(page as usize)) {
            state.cached_pages.push(page as usize);
            return;
        }
    }
    drop(pools);
    unsafe { __free_pages(page, 0) };
}

/// Return a page carried by a recyclable skb to its owning page pool.
pub unsafe fn recycle_skb_page(page: *mut Page) -> bool {
    if page.is_null() || !crate::mm::buddy::page_in_mem_map(page) {
        return false;
    }
    let raw = page.cast::<u8>();
    let magic = unsafe {
        raw.add(PAGE_PP_MAGIC_OFFSET)
            .cast::<usize>()
            .read_unaligned()
    };
    let pool = unsafe {
        raw.add(PAGE_PP_OFFSET)
            .cast::<*mut c_void>()
            .read_unaligned()
    };
    if magic != PP_SIGNATURE
        || pool.is_null()
        || !PAGE_POOLS
            .lock()
            .iter()
            .any(|state| state.pool == pool as usize)
    {
        return false;
    }
    unsafe { page_pool_put_unrefed_netmem(pool, page as usize, u32::MAX, true) };
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn page_pool_prefix_layout_matches_inline_users() {
        assert_eq!(size_of::<LinuxPagePoolParamsFast>(), 48);
        assert_eq!(size_of::<LinuxPagePool>(), 1664);
        assert_eq!(align_of::<LinuxPagePool>(), 64);
        assert_eq!(offset_of!(LinuxPagePool, bytes), 0);
    }
}
