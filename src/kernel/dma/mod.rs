//! linux-parity: partial
//! linux-source: vendor/linux/kernel/dma
//! test-origin: linux:vendor/linux/kernel/dma
//! DMA API — M55.
//!
//! Mirrors `kernel/dma/mapping.c` and `kernel/dma/direct.c`.
//! Provides `dma_alloc_coherent`, `dma_free_coherent`, `dma_map_single`,
//! `dma_unmap_single` with the same function signatures Linux exports.
//!
//! For M55 we implement the **direct** path: a kernel CPU address is translated
//! through the current page tables into the device-visible physical DMA
//! address. Bounce buffers (swiotlb) are a deferred milestone.
//!
//! References:
//!   - `kernel/dma/mapping.c:631`  — `dma_alloc_attrs`
//!   - `kernel/dma/mapping.c:191`  — `dma_map_page_attrs`
//!   - `kernel/dma/direct.c`       — coherent + streaming direct ops

extern crate alloc;

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::ffi::c_void;

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, EIO, ENXIO};
use crate::kernel::locking::qspinlock::QSpinLock;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::scatterlist::{LinuxScatterList, LinuxSgTable, SG_END, SG_PAGE_LINK_MASK};
use crate::mm::buddy::{is_buddy_ready, page_in_mem_map, page_to_pfn, with_global_buddy};
use crate::mm::frame::PAGE_SIZE;
use crate::mm::mm_types::VmAreaStruct;
use crate::mm::page::Page;
use crate::mm::page_flags::{__GFP_COMP, __GFP_DMA, __GFP_DMA32, __GFP_HIGHMEM, GfpFlags};

pub mod buf;
pub mod dummy;
pub mod fence;
pub mod ops_helpers;
pub mod remap;
pub mod resv;
pub mod sync_file;

/// DMA direction, mirroring `enum dma_data_direction` in
/// `include/linux/dma-direction.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DmaDirection {
    Bidirectional = 0,
    ToDevice = 1,
    FromDevice = 2,
    None = 3,
}

/// Device-visible DMA address (bus address in Linux terminology).
pub type DmaAddr = u64;

/// `DMA_MAPPING_ERROR` for the staged x86_64 ABI.
///
/// Source: `vendor/linux/include/linux/dma-mapping.h`.
pub const DMA_MAPPING_ERROR: DmaAddr = DmaAddr::MAX;
const MAX_DMA_CHANNELS: usize = 8;
const DMA_PAGE_SHIFT: u32 = 12;
const DMA_ATTR_ALLOC_SINGLE_PAGES: usize = 1 << 7;

static DMA_SPIN_LOCK: QSpinLock = QSpinLock::new();
static DMA_CHANNEL_BUSY: Mutex<[bool; MAX_DMA_CHANNELS]> =
    Mutex::new([false, false, false, false, true, false, false, false]);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    buf::register_module_exports();
    fence::register_module_exports();
    resv::register_module_exports();
    sync_file::register_module_exports();
    export_symbol_once("dma_set_mask", dma_set_mask as usize, false);
    export_symbol_once(
        "dma_set_coherent_mask",
        dma_set_coherent_mask as usize,
        false,
    );
    export_symbol_once(
        "__dma_sync_single_for_cpu",
        __dma_sync_single_for_cpu as usize,
        false,
    );
    export_symbol_once(
        "__dma_sync_single_for_device",
        __dma_sync_single_for_device as usize,
        false,
    );
    export_symbol_once(
        "__dma_sync_sg_for_cpu",
        __dma_sync_sg_for_cpu as usize,
        false,
    );
    export_symbol_once(
        "__dma_sync_sg_for_device",
        __dma_sync_sg_for_device as usize,
        false,
    );
    export_symbol_once("__dma_need_sync", __dma_need_sync as usize, true);
    export_symbol_once("dma_alloc_attrs", dma_alloc_attrs as usize, false);
    export_symbol_once("dma_free_attrs", dma_free_attrs as usize, false);
    export_symbol_once("dma_alloc_pages", dma_alloc_pages as usize, true);
    export_symbol_once("dma_free_pages", dma_free_pages as usize, true);
    export_symbol_once("dma_map_page_attrs", dma_map_page_attrs as usize, false);
    export_symbol_once("dma_unmap_page_attrs", dma_unmap_page_attrs as usize, false);
    export_symbol_once("dma_map_resource", dma_map_resource as usize, false);
    export_symbol_once("dma_unmap_resource", dma_unmap_resource as usize, false);
    export_symbol_once("dma_map_sgtable", dma_map_sgtable as usize, true);
    export_symbol_once("dma_unmap_sg_attrs", dma_unmap_sg_attrs as usize, false);
    export_symbol_once(
        "dma_alloc_noncontiguous",
        dma_alloc_noncontiguous as usize,
        true,
    );
    export_symbol_once(
        "dma_free_noncontiguous",
        dma_free_noncontiguous as usize,
        true,
    );
    export_symbol_once(
        "dma_vmap_noncontiguous",
        dma_vmap_noncontiguous as usize,
        true,
    );
    export_symbol_once(
        "dma_vunmap_noncontiguous",
        dma_vunmap_noncontiguous as usize,
        true,
    );
    export_symbol_once("dma_mmap_attrs", dma_mmap_attrs as usize, false);
    export_symbol_once("dma_mmap_pages", dma_mmap_pages as usize, true);
    export_symbol_once(
        "dma_mmap_noncontiguous",
        dma_mmap_noncontiguous as usize,
        true,
    );
    export_symbol_once("dma_can_mmap", dma_can_mmap as usize, true);
    export_symbol_once(
        "dma_spin_lock",
        core::ptr::addr_of!(DMA_SPIN_LOCK) as usize,
        false,
    );
    export_symbol_once("request_dma", request_dma as usize, false);
    export_symbol_once("free_dma", free_dma as usize, false);
}

/// `dma_vunmap_noncontiguous` - `vendor/linux/kernel/dma/mapping.c:857`.
pub unsafe extern "C" fn dma_vunmap_noncontiguous(_dev: *mut c_void, _vaddr: *mut c_void) {}

const LINUX_DEVICE_DMA_MASK_OFFSET: usize = 584;
const LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET: usize = 592;

unsafe fn linux_device_streaming_dma_mask(dev: *const c_void) -> Option<u64> {
    if dev.is_null() {
        return None;
    }
    let mask_ptr = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_DMA_MASK_OFFSET)
            .cast::<*mut u64>()
            .read()
    };
    if mask_ptr.is_null() {
        None
    } else {
        Some(unsafe { mask_ptr.read() })
    }
}

unsafe fn linux_device_coherent_dma_mask(dev: *const c_void) -> Option<u64> {
    if dev.is_null() {
        return None;
    }
    let mask = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET)
            .cast::<u64>()
            .read()
    };
    (mask != 0).then_some(mask)
}

fn dma_range_fits_mask(addr: DmaAddr, size: usize, mask: u64) -> bool {
    size != 0
        && addr != DMA_MAPPING_ERROR
        && u64::try_from(size)
            .ok()
            .and_then(|size| addr.checked_add(size - 1))
            .is_some_and(|end| end <= mask)
}

fn direct_dma_required_mask() -> Option<u64> {
    #[cfg(test)]
    {
        // Host allocations are not backed by Lupos's physical allocator.
        None
    }

    #[cfg(not(test))]
    {
        if !is_buddy_ready() {
            return None;
        }

        let end_pfn = with_global_buddy(|buddy| {
            buddy
                .zones
                .iter()
                .filter_map(|zone| zone.zone_start_pfn.checked_add(zone.spanned_pages))
                .max()
                .unwrap_or(0)
        });
        if end_pfn == 0 {
            return None;
        }

        u64::try_from(end_pfn)
            .ok()?
            .checked_mul(PAGE_SIZE as u64)?
            .checked_sub(1)
    }
}

fn direct_dma_mask_covers(mask: u64, required_mask: Option<u64>) -> bool {
    // An all-ones dma_addr_t covers every address this ABI can represent even
    // before the physical allocator is online. Narrower masks are safe only
    // once the complete buddy-managed physical range is known. Lupos does not
    // yet have Linux's ZONE_DMA32, SWIOTLB, or IOMMU fallback, so accepting a
    // conventional 32-bit mask unconditionally would let later mappings escape
    // the mask the driver was promised.
    mask == u64::MAX || required_mask.is_some_and(|required| mask >= required)
}

fn direct_dma_mask_supported(mask: u64) -> bool {
    direct_dma_mask_covers(mask, direct_dma_required_mask())
}

/// `dma_set_mask` - `vendor/linux/kernel/dma/mapping.c:917`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_set_mask(dev: *mut c_void, mask: u64) -> i32 {
    if dev.is_null() || !direct_dma_mask_supported(mask) {
        return -EIO;
    }
    let mask_ptr = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_DMA_MASK_OFFSET)
            .cast::<*mut u64>()
            .read()
    };
    if mask_ptr.is_null() {
        return -EIO;
    }
    unsafe {
        mask_ptr.write(mask);
    }
    0
}

/// `dma_set_coherent_mask` - `vendor/linux/kernel/dma/mapping.c:936`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_set_coherent_mask(dev: *mut c_void, mask: u64) -> i32 {
    if dev.is_null() || !direct_dma_mask_supported(mask) {
        return -EIO;
    }
    unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET)
            .cast::<u64>()
            .write(mask);
    }
    0
}

/// `__dma_sync_single_for_cpu` - `vendor/linux/kernel/dma/mapping.c:378`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dma_sync_single_for_cpu(
    _dev: *mut c_void,
    _addr: DmaAddr,
    _size: usize,
    _dir: DmaDirection,
) {
}

/// `__dma_sync_single_for_device` - `vendor/linux/kernel/dma/mapping.c:396`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dma_sync_single_for_device(
    _dev: *mut c_void,
    _addr: DmaAddr,
    _size: usize,
    _dir: DmaDirection,
) {
}

/// `__dma_sync_sg_for_cpu` - `vendor/linux/kernel/dma/mapping.c:415`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dma_sync_sg_for_cpu(
    _dev: *mut c_void,
    _sg: *mut LinuxScatterList,
    _nelems: i32,
    _dir: DmaDirection,
) {
}

/// `__dma_sync_sg_for_device` - `vendor/linux/kernel/dma/mapping.c:431`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dma_sync_sg_for_device(
    _dev: *mut c_void,
    _sg: *mut LinuxScatterList,
    _nelems: i32,
    _dir: DmaDirection,
) {
}

/// `__dma_need_sync` - `vendor/linux/kernel/dma/mapping.c:446`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dma_need_sync(_dev: *mut c_void, _dma_addr: DmaAddr) -> bool {
    false
}

/// `dma_alloc_attrs` - `vendor/linux/kernel/dma/mapping.c:631`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_alloc_attrs(
    dev: *mut c_void,
    size: usize,
    dma_handle: *mut DmaAddr,
    _flag: u32,
    _attrs: usize,
) -> *mut c_void {
    let Some(mask) = (unsafe { linux_device_coherent_dma_mask(dev) }) else {
        return core::ptr::null_mut();
    };
    let Some((ptr, dma)) = dma_alloc_coherent(size) else {
        return core::ptr::null_mut();
    };
    if !dma_range_fits_mask(dma, size, mask) {
        unsafe {
            dma_free_coherent(ptr, size);
        }
        if !dma_handle.is_null() {
            unsafe {
                dma_handle.write(DMA_MAPPING_ERROR);
            }
        }
        return core::ptr::null_mut();
    }
    if !dma_handle.is_null() {
        unsafe { dma_handle.write(dma) };
    }
    ptr.cast()
}

/// `dma_free_attrs` - `vendor/linux/kernel/dma/mapping.c:684`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_free_attrs(
    _dev: *mut c_void,
    size: usize,
    cpu_addr: *mut c_void,
    _dma_handle: DmaAddr,
    _attrs: usize,
) {
    unsafe { dma_free_coherent(cpu_addr.cast(), size) };
}

fn dma_order_for_size(size: usize) -> Option<u32> {
    let aligned = dma_page_align(size)?;
    if aligned == 0 {
        return None;
    }
    let pages = aligned >> DMA_PAGE_SHIFT;
    Some(usize::BITS - pages.max(1).saturating_sub(1).leading_zeros())
}

fn dma_page_virt(page: *mut Page) -> *mut u8 {
    crate::arch::x86::mm::paging::pfn_to_virt(page_to_pfn(page))
}

fn dma_page_dma_addr(page: *mut Page) -> Option<DmaAddr> {
    let pfn = page_to_pfn(page);
    pfn.checked_mul(PAGE_SIZE)
        .and_then(|addr| DmaAddr::try_from(addr).ok())
}

fn dma_alloc_pages_gfp_valid(gfp: GfpFlags) -> bool {
    gfp & (__GFP_DMA | __GFP_DMA32 | __GFP_HIGHMEM | __GFP_COMP) == 0
}

/// `dma_alloc_pages` - `vendor/linux/kernel/dma/mapping.c:722`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_alloc_pages(
    dev: *mut c_void,
    size: usize,
    dma_handle: *mut DmaAddr,
    _dir: DmaDirection,
    gfp: GfpFlags,
) -> *mut Page {
    if !dma_alloc_pages_gfp_valid(gfp) {
        return core::ptr::null_mut();
    }
    let Some(order) = dma_order_for_size(size) else {
        return core::ptr::null_mut();
    };
    let Some(aligned_size) = dma_page_align(size) else {
        return core::ptr::null_mut();
    };
    let Some(mask) = (unsafe { linux_device_coherent_dma_mask(dev) }) else {
        return core::ptr::null_mut();
    };

    let page = crate::mm::page_alloc::alloc_pages_noprof(gfp, order);
    if page.is_null() || !page_in_mem_map(page) {
        return core::ptr::null_mut();
    }
    let Some(dma) = dma_page_dma_addr(page) else {
        crate::mm::page_alloc::__free_pages(page, order);
        return core::ptr::null_mut();
    };
    if !dma_range_fits_mask(dma, aligned_size, mask) {
        crate::mm::page_alloc::__free_pages(page, order);
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::write_bytes(dma_page_virt(page), 0, aligned_size);
    }
    if !dma_handle.is_null() {
        unsafe {
            dma_handle.write(dma);
        }
    }
    page
}

/// `dma_free_pages` - `vendor/linux/kernel/dma/mapping.c:752`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_free_pages(
    _dev: *mut c_void,
    size: usize,
    page: *mut Page,
    _dma_handle: DmaAddr,
    _dir: DmaDirection,
) {
    if page.is_null() || !page_in_mem_map(page) {
        return;
    }
    if let Some(order) = dma_order_for_size(size) {
        crate::mm::page_alloc::__free_pages(page, order);
    }
}

/// `dma_map_page_attrs` - `vendor/linux/kernel/dma/mapping.c:191`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_map_page_attrs(
    dev: *mut c_void,
    page: *mut Page,
    offset: usize,
    size: usize,
    dir: DmaDirection,
    _attrs: usize,
) -> DmaAddr {
    if page.is_null() || !page_in_mem_map(page) || dir == DmaDirection::None {
        return DMA_MAPPING_ERROR;
    }
    let ptr = crate::arch::x86::mm::paging::pfn_to_virt(page_to_pfn(page)).wrapping_add(offset);
    unsafe { dma_map_single_for_device(dev, ptr, size, dir) }
}

/// `dma_unmap_page_attrs` - `vendor/linux/kernel/dma/mapping.c:242`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_unmap_page_attrs(
    _dev: *mut c_void,
    dma_addr: DmaAddr,
    size: usize,
    dir: DmaDirection,
    _attrs: usize,
) {
    dma_unmap_single(dma_addr, size, dir);
}

/// `dma_map_resource` - `vendor/linux/kernel/dma/mapping.c:363`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_map_resource(
    _dev: *mut c_void,
    phys_addr: u64,
    _size: usize,
    _dir: DmaDirection,
    _attrs: usize,
) -> DmaAddr {
    phys_addr
}

/// `dma_unmap_resource` - `vendor/linux/kernel/dma/mapping.c:370`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_unmap_resource(
    _dev: *mut c_void,
    _dma_addr: DmaAddr,
    _size: usize,
    _dir: DmaDirection,
    _attrs: usize,
) {
}

unsafe fn linux_sg_page(sg: *const LinuxScatterList) -> *mut Page {
    if sg.is_null() {
        return core::ptr::null_mut();
    }
    (unsafe { (*sg).page_link } & !SG_PAGE_LINK_MASK) as *mut Page
}

/// `dma_map_sgtable` - `vendor/linux/kernel/dma/mapping.c:331`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_map_sgtable(
    dev: *mut c_void,
    sgt: *mut LinuxSgTable,
    dir: DmaDirection,
    _attrs: usize,
) -> i32 {
    if sgt.is_null() || dir == DmaDirection::None {
        return -EINVAL;
    }
    let nents = unsafe { (*sgt).orig_nents };
    let sgl = unsafe { (*sgt).sgl };
    if nents == 0 || sgl.is_null() {
        return -EINVAL;
    }
    let Some(mask) = (unsafe { linux_device_streaming_dma_mask(dev) }) else {
        return -EIO;
    };

    for idx in 0..nents as usize {
        let sg = unsafe { sgl.add(idx) };
        let page = unsafe { linux_sg_page(sg) };
        if page.is_null() || !page_in_mem_map(page) {
            return -EIO;
        }
        let Some(base) = dma_page_dma_addr(page) else {
            return -EIO;
        };
        let offset = unsafe { (*sg).offset } as u64;
        let length = unsafe { (*sg).length } as usize;
        let Some(dma) = base.checked_add(offset) else {
            return -EIO;
        };
        if !dma_range_fits_mask(dma, length, mask) {
            return -EIO;
        }
        unsafe {
            (*sg).dma_address = dma as usize;
            (*sg).dma_length = (*sg).length;
        }
    }

    unsafe {
        (*sgt).nents = nents;
    }
    0
}

/// `dma_unmap_sg_attrs` - `vendor/linux/kernel/dma/mapping.c:344`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_unmap_sg_attrs(
    _dev: *mut c_void,
    _sg: *mut LinuxScatterList,
    _nents: i32,
    _dir: DmaDirection,
    _attrs: usize,
) {
}

/// `dma_alloc_noncontiguous` - `vendor/linux/kernel/dma/mapping.c:798`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_alloc_noncontiguous(
    dev: *mut c_void,
    size: usize,
    dir: DmaDirection,
    gfp: GfpFlags,
    attrs: usize,
) -> *mut LinuxSgTable {
    if attrs & !DMA_ATTR_ALLOC_SINGLE_PAGES != 0 || gfp & __GFP_COMP != 0 {
        return core::ptr::null_mut();
    }

    let sgt = unsafe {
        crate::mm::slab::kmalloc(core::mem::size_of::<LinuxSgTable>(), gfp).cast::<LinuxSgTable>()
    };
    if sgt.is_null() {
        return core::ptr::null_mut();
    }
    if unsafe { crate::lib::scatterlist::linux_sg_alloc_table(sgt, 1, gfp) } != 0 {
        unsafe { crate::mm::slab::kfree(sgt.cast()) };
        return core::ptr::null_mut();
    }

    let mut dma = DMA_MAPPING_ERROR;
    let page = unsafe { dma_alloc_pages(dev, size, &mut dma, dir, gfp) };
    if page.is_null() {
        unsafe {
            crate::lib::scatterlist::linux_sg_free_table(sgt);
            crate::mm::slab::kfree(sgt.cast());
        }
        return core::ptr::null_mut();
    }

    let Some(aligned_size) = dma_page_align(size) else {
        unsafe {
            dma_free_pages(dev, size, page, dma, dir);
            crate::lib::scatterlist::linux_sg_free_table(sgt);
            crate::mm::slab::kfree(sgt.cast());
        }
        return core::ptr::null_mut();
    };
    let Ok(length) = u32::try_from(aligned_size) else {
        unsafe {
            dma_free_pages(dev, size, page, dma, dir);
            crate::lib::scatterlist::linux_sg_free_table(sgt);
            crate::mm::slab::kfree(sgt.cast());
        }
        return core::ptr::null_mut();
    };

    let sg = unsafe { (*sgt).sgl };
    unsafe {
        (*sg).page_link = (page as usize & !SG_PAGE_LINK_MASK) | SG_END;
        (*sg).offset = 0;
        (*sg).length = length;
        (*sg).dma_address = dma as usize;
        (*sg).dma_length = length;
        (*sgt).nents = 1;
    }
    sgt
}

/// `dma_free_noncontiguous` - `vendor/linux/kernel/dma/mapping.c:833`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_free_noncontiguous(
    dev: *mut c_void,
    size: usize,
    sgt: *mut LinuxSgTable,
    dir: DmaDirection,
) {
    if sgt.is_null() {
        return;
    }
    let sg = unsafe { (*sgt).sgl };
    if !sg.is_null() {
        let page = unsafe { linux_sg_page(sg) };
        if !page.is_null() {
            unsafe { dma_free_pages(dev, size, page, (*sg).dma_address as DmaAddr, dir) };
        }
        unsafe {
            crate::lib::scatterlist::linux_sg_free_table(sgt);
        }
    }
    unsafe {
        crate::mm::slab::kfree(sgt.cast());
    }
}

/// `dma_vmap_noncontiguous` - `vendor/linux/kernel/dma/mapping.c:846`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dma_vmap_noncontiguous(
    _dev: *mut c_void,
    _size: usize,
    sgt: *mut LinuxSgTable,
) -> *mut c_void {
    if sgt.is_null() {
        return core::ptr::null_mut();
    }
    let sg = unsafe { (*sgt).sgl };
    let page = unsafe { linux_sg_page(sg) };
    if page.is_null() || !page_in_mem_map(page) {
        return core::ptr::null_mut();
    }
    dma_page_virt(page).cast()
}

fn dma_page_align(size: usize) -> Option<usize> {
    size.checked_add(PAGE_SIZE - 1)
        .map(|size| size & !(PAGE_SIZE - 1))
}

unsafe fn dma_vma_pages(vma: *const VmAreaStruct) -> Option<u64> {
    if vma.is_null() {
        return None;
    }
    let start = unsafe { (*vma).vm_start };
    let end = unsafe { (*vma).vm_end };
    (end >= start).then_some((end - start) >> DMA_PAGE_SHIFT)
}

fn dma_pfn_from_dma_addr(dma_addr: DmaAddr) -> u64 {
    dma_addr >> DMA_PAGE_SHIFT
}

/// `dma_mmap_attrs` - `vendor/linux/kernel/dma/mapping.c:582`.
#[unsafe(export_name = "dma_mmap_attrs")]
pub unsafe extern "C" fn dma_mmap_attrs(
    _dev: *mut c_void,
    vma: *mut VmAreaStruct,
    _cpu_addr: *mut c_void,
    dma_addr: DmaAddr,
    size: usize,
    _attrs: usize,
) -> i32 {
    let Some(user_count) = (unsafe { dma_vma_pages(vma) }) else {
        return -EINVAL;
    };
    let Some(count) = dma_page_align(size).map(|size| (size >> DMA_PAGE_SHIFT) as u64) else {
        return -ENXIO;
    };
    let pgoff = unsafe { (*vma).vm_pgoff };
    if pgoff >= count || user_count > count.saturating_sub(pgoff) {
        return -ENXIO;
    }

    let Some(bytes) = user_count.checked_shl(DMA_PAGE_SHIFT) else {
        return -ENXIO;
    };
    crate::mm::mm_public::remap_pfn_range(
        vma,
        unsafe { (*vma).vm_start },
        dma_pfn_from_dma_addr(dma_addr) + pgoff,
        bytes,
        unsafe { (*vma).vm_page_prot },
    )
}

/// `dma_mmap_pages` - `vendor/linux/kernel/dma/mapping.c:761`.
#[unsafe(export_name = "dma_mmap_pages")]
pub unsafe extern "C" fn dma_mmap_pages(
    _dev: *mut c_void,
    vma: *mut VmAreaStruct,
    size: usize,
    page: *mut Page,
) -> i32 {
    if page.is_null() || !page_in_mem_map(page) {
        return -ENXIO;
    }
    let Some(user_count) = (unsafe { dma_vma_pages(vma) }) else {
        return -EINVAL;
    };
    let Some(count) = dma_page_align(size).map(|size| (size >> DMA_PAGE_SHIFT) as u64) else {
        return -ENXIO;
    };
    let pgoff = unsafe { (*vma).vm_pgoff };
    if pgoff >= count || user_count > count.saturating_sub(pgoff) {
        return -ENXIO;
    }

    let Some(bytes) = user_count.checked_shl(DMA_PAGE_SHIFT) else {
        return -ENXIO;
    };
    crate::mm::mm_public::remap_pfn_range(
        vma,
        unsafe { (*vma).vm_start },
        page_to_pfn(page) as u64 + pgoff,
        bytes,
        unsafe { (*vma).vm_page_prot },
    )
}

/// `dma_mmap_noncontiguous` - `vendor/linux/kernel/dma/mapping.c:864`.
#[unsafe(export_name = "dma_mmap_noncontiguous")]
pub unsafe extern "C" fn dma_mmap_noncontiguous(
    dev: *mut c_void,
    vma: *mut VmAreaStruct,
    size: usize,
    sgt: *mut LinuxSgTable,
) -> i32 {
    if sgt.is_null() {
        return -ENXIO;
    }
    let sg = unsafe { (*sgt).sgl };
    if sg.is_null() {
        return -ENXIO;
    }
    let page = unsafe { linux_sg_page(sg) };
    unsafe { dma_mmap_pages(dev, vma, size, page) }
}

/// `dma_can_mmap` - `vendor/linux/kernel/dma/mapping.c:557`.
#[unsafe(export_name = "dma_can_mmap")]
pub unsafe extern "C" fn dma_can_mmap(_dev: *mut c_void) -> bool {
    true
}

/// `request_dma` - `vendor/linux/kernel/dma.c:70`.
pub unsafe extern "C" fn request_dma(dmanr: u32, _device_id: *const i8) -> i32 {
    let channel = dmanr as usize;
    if channel >= MAX_DMA_CHANNELS {
        return -EINVAL;
    }

    let mut busy = DMA_CHANNEL_BUSY.lock();
    if busy[channel] {
        return -EBUSY;
    }
    busy[channel] = true;
    0
}

/// `free_dma` - `vendor/linux/kernel/dma.c:88`.
pub unsafe extern "C" fn free_dma(dmanr: u32) {
    let channel = dmanr as usize;
    if channel >= MAX_DMA_CHANNELS || channel == 4 {
        return;
    }
    DMA_CHANNEL_BUSY.lock()[channel] = false;
}

/// `dma_alloc_coherent(dev, size, dma_handle, gfp)` — `kernel/dma/mapping.c:631`.
///
/// Allocates a contiguous DMA-coherent buffer and returns both the kernel
/// virtual address and the device-visible DMA address.
///
/// Returns `None` if allocation fails.
pub fn dma_alloc_coherent(size: usize) -> Option<(*mut u8, DmaAddr)> {
    if size == 0 {
        return None;
    }
    let layout = Layout::from_size_align(size, 4096).ok()?;
    // SAFETY: alloc_zeroed panics on null on most allocators; we check.
    let ptr = unsafe { alloc_zeroed(layout) };
    if ptr.is_null() {
        return None;
    }
    let Some(dma_addr) = dma_addr_from_cpu_addr(ptr) else {
        unsafe {
            dealloc(ptr, layout);
        }
        return None;
    };
    Some((ptr, dma_addr))
}

/// `dma_free_coherent` — mirror of `dma_free_attrs`.
///
/// # Safety
/// `ptr` must be the pointer returned by a previous `dma_alloc_coherent`
/// call with the same `size`.
pub unsafe fn dma_free_coherent(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    let layout = Layout::from_size_align(size, 4096).expect("dma_free_coherent layout");
    unsafe {
        dealloc(ptr, layout);
    }
}

/// `dma_map_single` — `kernel/dma/mapping.c:191` (`dma_map_page_attrs`).
///
/// On the direct path the DMA address is the physical address backing the
/// kernel virtual address. A real implementation would flush/invalidate caches
/// and check IOMMU mapping; we elide both for M55.
#[inline]
pub fn dma_map_single(ptr: *const u8, _size: usize, _dir: DmaDirection) -> DmaAddr {
    dma_addr_from_cpu_addr(ptr).unwrap_or(0)
}

/// Direct-map a CPU address for a specific Linux device.
///
/// This is the no-IOMMU/no-SWIOTLB branch of
/// `vendor/linux/kernel/dma/direct.h:dma_direct_map_phys()`: the active
/// streaming mask is checked after translating the CPU address.  Lupos does
/// not claim Linux's bounce-buffer fallback, so an address outside the mask is
/// rejected with `DMA_MAPPING_ERROR`.
pub unsafe fn dma_map_single_for_device(
    dev: *mut c_void,
    ptr: *const u8,
    size: usize,
    dir: DmaDirection,
) -> DmaAddr {
    if dir == DmaDirection::None {
        return DMA_MAPPING_ERROR;
    }
    let Some(mask) = (unsafe { linux_device_streaming_dma_mask(dev) }) else {
        return DMA_MAPPING_ERROR;
    };
    let Some(dma) = dma_addr_from_cpu_addr(ptr) else {
        return DMA_MAPPING_ERROR;
    };
    if dma_range_fits_mask(dma, size, mask) {
        dma
    } else {
        DMA_MAPPING_ERROR
    }
}

/// `dma_unmap_single`.
#[inline]
pub fn dma_unmap_single(_dma: DmaAddr, _size: usize, _dir: DmaDirection) {
    // On the direct path no invalidation is needed.
}

/// `dma_map_sg` (scatter-gather) — `kernel/dma/mapping.c:333` (`dma_map_sgtable`).
///
/// Maps a list of `(ptr, len)` pairs.  Returns the DMA address of each.
pub fn dma_map_sg(segments: &[(*const u8, usize)], dir: DmaDirection) -> alloc::vec::Vec<DmaAddr> {
    segments
        .iter()
        .map(|&(ptr, size)| dma_map_single(ptr, size, dir))
        .collect()
}

/// Convert a kernel CPU address into the device-visible direct DMA address.
///
/// Linux's direct DMA ops use `virt_to_phys()` on x86. Host-side unit tests do
/// not run with Lupos page tables installed, so they keep the test harness's
/// identity convention.
pub fn dma_addr_from_cpu_addr(ptr: *const u8) -> Option<DmaAddr> {
    if ptr.is_null() {
        return None;
    }

    #[cfg(test)]
    {
        Some(ptr as DmaAddr)
    }

    #[cfg(not(test))]
    {
        let addr = ptr as u64;
        crate::arch::x86::mm::paging::virt_to_phys(addr)
            .or_else(|| (addr < crate::arch::x86::mm::paging::PAGE_OFFSET).then_some(addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_coherent_returns_non_null() {
        let Some((ptr, dma)) = dma_alloc_coherent(4096) else {
            panic!("dma_alloc_coherent returned None");
        };
        assert!(!ptr.is_null());
        assert_eq!(dma_addr_from_cpu_addr(ptr), Some(dma));
        unsafe { dma_free_coherent(ptr, 4096) };
    }

    #[test]
    fn dma_mask_exports_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("dma_set_mask"),
            Some(dma_set_mask as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_set_coherent_mask"),
            Some(dma_set_coherent_mask as usize)
        );
        assert_eq!(
            unsafe { dma_set_mask(core::ptr::null_mut(), u64::MAX) },
            -EIO
        );
        assert_eq!(
            unsafe { dma_set_coherent_mask(core::ptr::null_mut(), u64::MAX) },
            -EIO
        );
    }

    #[test]
    fn dma_mask_helpers_update_target_config_device_fields() {
        #[repr(C, align(8))]
        struct DeviceStorage([u8; 760]);

        let mut storage = DeviceStorage([0; 760]);
        let mut streaming_mask = u32::MAX as u64;
        unsafe {
            storage
                .0
                .as_mut_ptr()
                .add(LINUX_DEVICE_DMA_MASK_OFFSET)
                .cast::<*mut u64>()
                .write(core::ptr::addr_of_mut!(streaming_mask));

            assert_eq!(dma_set_mask(storage.0.as_mut_ptr().cast(), u64::MAX), 0);
            assert_eq!(streaming_mask, u64::MAX);
            assert_eq!(
                dma_set_coherent_mask(storage.0.as_mut_ptr().cast(), u64::MAX),
                0
            );
            assert_eq!(
                storage
                    .0
                    .as_ptr()
                    .add(LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET)
                    .cast::<u64>()
                    .read(),
                u64::MAX
            );
            assert_eq!(dma_set_mask(storage.0.as_mut_ptr().cast(), 0), -EIO);
        }
    }

    #[test]
    fn direct_dma_masks_fail_closed_without_a_known_address_ceiling() {
        assert!(direct_dma_mask_covers(u64::MAX, None));
        assert!(!direct_dma_mask_covers(u32::MAX as u64, None));
        assert!(direct_dma_mask_covers(0x1_ffff_ffff, Some(0x1_ffff_ffff)));
        assert!(!direct_dma_mask_covers(0xffff_ffff, Some(0x1_0000_0000)));
    }

    #[test]
    fn alloc_coherent_zero_size_returns_none() {
        assert!(dma_alloc_coherent(0).is_none());
    }

    #[test]
    fn map_single_identity() {
        let buf = [0u8; 16];
        let dma = dma_map_single(buf.as_ptr(), buf.len(), DmaDirection::ToDevice);
        assert_eq!(dma_addr_from_cpu_addr(buf.as_ptr()), Some(dma));
        dma_unmap_single(dma, buf.len(), DmaDirection::ToDevice);
    }

    #[test]
    fn map_sg_returns_one_per_segment() {
        let a = [0u8; 64];
        let b = [0u8; 64];
        let segs: &[(*const u8, usize)] = &[(a.as_ptr(), 64), (b.as_ptr(), 64)];
        let addrs = dma_map_sg(segs, DmaDirection::Bidirectional);
        assert_eq!(addrs.len(), 2);
        assert_eq!(dma_addr_from_cpu_addr(a.as_ptr()), Some(addrs[0]));
        assert_eq!(dma_addr_from_cpu_addr(b.as_ptr()), Some(addrs[1]));
    }
}
