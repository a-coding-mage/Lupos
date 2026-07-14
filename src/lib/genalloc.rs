//! linux-parity: partial
//! linux-source: vendor/linux/lib/genalloc.c
//! Generic allocator exports used by vendor modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

#[derive(Clone, Copy)]
struct GenPoolAllocation {
    pool: usize,
    addr: usize,
    base: usize,
    size: usize,
    dma: u64,
}

lazy_static! {
    static ref GEN_POOL_ALLOCATIONS: Mutex<Vec<GenPoolAllocation>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "gen_pool_dma_alloc_align",
        linux_gen_pool_dma_alloc_align as usize,
        false,
    );
    export_symbol_once(
        "gen_pool_dma_alloc",
        linux_gen_pool_dma_alloc as usize,
        false,
    );
    export_symbol_once(
        "gen_pool_dma_zalloc_align",
        linux_gen_pool_dma_zalloc_align as usize,
        false,
    );
    export_symbol_once(
        "gen_pool_free_owner",
        linux_gen_pool_free_owner as usize,
        false,
    );
    export_symbol_once(
        "gen_pool_virt_to_phys",
        linux_gen_pool_virt_to_phys as usize,
        false,
    );
}

fn align_up(value: usize, align: usize) -> Option<usize> {
    if align <= 1 {
        return Some(value);
    }
    let rem = value % align;
    if rem == 0 {
        Some(value)
    } else {
        value.checked_add(align - rem)
    }
}

fn dma_for_addr(addr: usize) -> u64 {
    crate::arch::x86::mm::paging::virt_to_phys(addr as u64).unwrap_or(addr as u64)
}

unsafe fn gen_pool_alloc_aligned(
    pool: *mut c_void,
    size: usize,
    dma: *mut u64,
    align: usize,
    zero: bool,
) -> *mut c_void {
    if pool.is_null() || size == 0 {
        return core::ptr::null_mut();
    }
    let align = align.max(core::mem::size_of::<usize>());
    let Some(extra) = align.checked_sub(1) else {
        return core::ptr::null_mut();
    };
    let Some(bytes) = size.checked_add(extra) else {
        return core::ptr::null_mut();
    };
    let flags = if zero {
        GFP_KERNEL | __GFP_ZERO
    } else {
        GFP_KERNEL
    };
    let base = unsafe { crate::mm::slab::kmalloc(bytes, flags) };
    if base.is_null() {
        return core::ptr::null_mut();
    }
    let Some(addr) = align_up(base as usize, align) else {
        unsafe { crate::mm::slab::kfree(base) };
        return core::ptr::null_mut();
    };
    if zero && addr != base as usize {
        unsafe {
            core::ptr::write_bytes(addr as *mut u8, 0, size);
        }
    }
    let dma_addr = dma_for_addr(addr);
    if !dma.is_null() {
        unsafe {
            *dma = dma_addr;
        }
    }
    GEN_POOL_ALLOCATIONS.lock().push(GenPoolAllocation {
        pool: pool as usize,
        addr,
        base: base as usize,
        size,
        dma: dma_addr,
    });
    addr as *mut c_void
}

/// `gen_pool_dma_alloc_align` - `vendor/linux/lib/genalloc.c`.
#[unsafe(export_name = "gen_pool_dma_alloc_align")]
unsafe extern "C" fn linux_gen_pool_dma_alloc_align(
    pool: *mut c_void,
    size: usize,
    dma: *mut u64,
    align: i32,
) -> *mut c_void {
    let align = if align <= 0 { 1 } else { align as usize };
    unsafe { gen_pool_alloc_aligned(pool, size, dma, align, false) }
}

/// `gen_pool_dma_alloc` - `vendor/linux/lib/genalloc.c`.
#[unsafe(export_name = "gen_pool_dma_alloc")]
unsafe extern "C" fn linux_gen_pool_dma_alloc(
    pool: *mut c_void,
    size: usize,
    dma: *mut u64,
) -> *mut c_void {
    unsafe { gen_pool_alloc_aligned(pool, size, dma, core::mem::size_of::<usize>(), false) }
}

/// `gen_pool_dma_zalloc_align` - `vendor/linux/lib/genalloc.c`.
#[unsafe(export_name = "gen_pool_dma_zalloc_align")]
unsafe extern "C" fn linux_gen_pool_dma_zalloc_align(
    pool: *mut c_void,
    size: usize,
    dma: *mut u64,
    align: i32,
) -> *mut c_void {
    let align = if align <= 0 { 1 } else { align as usize };
    unsafe { gen_pool_alloc_aligned(pool, size, dma, align, true) }
}

/// `gen_pool_free_owner` - `vendor/linux/lib/genalloc.c`.
#[unsafe(export_name = "gen_pool_free_owner")]
unsafe extern "C" fn linux_gen_pool_free_owner(
    pool: *mut c_void,
    addr: usize,
    _size: usize,
    owner: *mut *mut c_void,
) {
    if !owner.is_null() {
        unsafe {
            *owner = core::ptr::null_mut();
        }
    }
    if pool.is_null() || addr == 0 {
        return;
    }
    let mut allocations = GEN_POOL_ALLOCATIONS.lock();
    let Some(index) = allocations
        .iter()
        .position(|allocation| allocation.pool == pool as usize && allocation.addr == addr)
    else {
        return;
    };
    let allocation = allocations.swap_remove(index);
    drop(allocations);
    unsafe {
        crate::mm::slab::kfree(allocation.base as *mut u8);
    }
}

/// `gen_pool_virt_to_phys` - `vendor/linux/lib/genalloc.c`.
#[unsafe(export_name = "gen_pool_virt_to_phys")]
unsafe extern "C" fn linux_gen_pool_virt_to_phys(_pool: *mut c_void, addr: usize) -> u64 {
    if let Some(allocation) = GEN_POOL_ALLOCATIONS
        .lock()
        .iter()
        .find(|allocation| addr >= allocation.addr && addr < allocation.addr + allocation.size)
        .copied()
    {
        return allocation.dma + (addr - allocation.addr) as u64;
    }
    dma_for_addr(addr)
}
