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

use crate::include::uapi::errno::EIO;
use crate::kernel::module::{export_symbol, find_symbol};
#[cfg(not(test))]
use crate::mm::buddy::{is_buddy_ready, with_global_buddy};
use crate::mm::buddy::{page_in_mem_map, page_to_pfn};
#[cfg(not(test))]
use crate::mm::frame::PAGE_SIZE;
use crate::mm::page::Page;

pub mod dummy;
pub mod ops_helpers;
pub mod remap;

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

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
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
    export_symbol_once("dma_alloc_attrs", dma_alloc_attrs as usize, false);
    export_symbol_once("dma_free_attrs", dma_free_attrs as usize, false);
    export_symbol_once("dma_map_page_attrs", dma_map_page_attrs as usize, false);
    export_symbol_once("dma_unmap_page_attrs", dma_unmap_page_attrs as usize, false);
}

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
