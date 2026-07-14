//! linux-parity: partial
//! linux-source: vendor/linux/mm/ioremap.c
//! test-origin: linux:vendor/linux/mm/ioremap.c
//! Generic IO-memory remap alignment and validation. The registered module ABI
//! currently exposes only `ioremap()`/`iounmap()` and inherits the x86
//! backend's incomplete memtype and VA-lifetime handling.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::mm::ioremap::{
    IoremapMapping as X86IoremapMapping, ioremap as x86_ioremap,
    ioremap_cachemode as x86_ioremap_cachemode, ioremap_wc as x86_ioremap_wc,
    iounmap as x86_iounmap,
};
use crate::arch::x86::mm::pat::cachemode::PageCacheMode;
use crate::kernel::module::{export_symbol, find_symbol};

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);
const MEMREMAP_WB: usize = 1 << 0;
const MEMREMAP_WT: usize = 1 << 1;
const MEMREMAP_WC: usize = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoremapMapping {
    pub aligned_phys: u64,
    pub aligned_size: u64,
    pub offset: u64,
}

#[derive(Clone, Copy)]
struct RegisteredIoremap {
    addr: usize,
    mapping: X86IoremapMapping,
}

lazy_static! {
    static ref IOREMAPS: Mutex<Vec<RegisteredIoremap>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ioremap", linux_ioremap as usize, false);
    export_symbol_once("ioremap_cache", linux_ioremap_cache as usize, false);
    export_symbol_once("ioremap_wc", linux_ioremap_wc as usize, false);
    export_symbol_once("devm_ioremap", linux_devm_ioremap as usize, false);
    export_symbol_once("devm_ioremap_wc", linux_devm_ioremap_wc as usize, false);
    export_symbol_once("iounmap", linux_iounmap as usize, false);
    export_symbol_once("memremap", linux_memremap as usize, false);
    export_symbol_once("memunmap", linux_memunmap as usize, false);
}

fn register_mapping(mapping: X86IoremapMapping) -> *mut c_void {
    let addr = mapping.virt as usize;
    IOREMAPS.lock().push(RegisteredIoremap { addr, mapping });
    addr as *mut c_void
}

unsafe extern "C" fn linux_ioremap(phys_addr: u64, size: usize) -> *mut c_void {
    let Ok(mapping) = (unsafe { x86_ioremap(phys_addr, size as u64) }) else {
        return core::ptr::null_mut();
    };
    register_mapping(mapping)
}

unsafe extern "C" fn linux_ioremap_cache(phys_addr: u64, size: usize) -> *mut c_void {
    let Ok(mapping) =
        (unsafe { x86_ioremap_cachemode(phys_addr, size as u64, PageCacheMode::WriteBack) })
    else {
        return core::ptr::null_mut();
    };
    register_mapping(mapping)
}

unsafe extern "C" fn linux_ioremap_wc(phys_addr: u64, size: usize) -> *mut c_void {
    let Ok(mapping) = (unsafe { x86_ioremap_wc(phys_addr, size as u64) }) else {
        return core::ptr::null_mut();
    };
    register_mapping(mapping)
}

unsafe extern "C" fn linux_devm_ioremap(
    _dev: *mut c_void,
    phys_addr: u64,
    size: usize,
) -> *mut c_void {
    unsafe { linux_ioremap(phys_addr, size) }
}

unsafe extern "C" fn linux_devm_ioremap_wc(
    _dev: *mut c_void,
    phys_addr: u64,
    size: usize,
) -> *mut c_void {
    unsafe { linux_ioremap_wc(phys_addr, size) }
}

unsafe extern "C" fn linux_memremap(phys_addr: u64, size: usize, flags: usize) -> *mut c_void {
    if flags == 0 {
        return core::ptr::null_mut();
    }
    if flags & MEMREMAP_WB != 0 {
        return unsafe { linux_ioremap_cache(phys_addr, size) };
    }
    if flags & MEMREMAP_WT != 0 {
        return unsafe { linux_ioremap_cache(phys_addr, size) };
    }
    if flags & MEMREMAP_WC != 0 {
        return unsafe { linux_ioremap_wc(phys_addr, size) };
    }
    core::ptr::null_mut()
}

unsafe extern "C" fn linux_iounmap(addr: *mut c_void) {
    if addr.is_null() {
        return;
    }
    let mut maps = IOREMAPS.lock();
    if let Some(index) = maps
        .iter()
        .position(|mapping| mapping.addr == addr as usize)
    {
        let mapping = maps.swap_remove(index).mapping;
        drop(maps);
        unsafe { x86_iounmap(mapping) };
    }
}

unsafe extern "C" fn linux_memunmap(addr: *mut c_void) {
    unsafe { linux_iounmap(addr) };
}

pub const fn page_align(size: u64) -> u64 {
    (size + PAGE_SIZE - 1) & PAGE_MASK
}

pub fn generic_ioremap_prot(
    phys_addr: u64,
    size: u64,
    slab_available: bool,
    vm_area_available: bool,
    page_range_ok: bool,
) -> Option<IoremapMapping> {
    if !slab_available || size == 0 {
        return None;
    }
    let last_addr = phys_addr.checked_add(size - 1)?;
    if last_addr < phys_addr || !vm_area_available {
        return None;
    }
    let offset = phys_addr & !PAGE_MASK;
    let aligned_phys = phys_addr - offset;
    let aligned_size = page_align(size + offset);
    if !page_range_ok {
        return None;
    }
    Some(IoremapMapping {
        aligned_phys,
        aligned_size,
        offset,
    })
}

pub const fn generic_iounmap_should_vunmap(is_ioremap_addr: bool) -> bool {
    is_ioremap_addr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_ioremap_validation_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/ioremap.c"
        ));
        assert!(source.contains("if (WARN_ON_ONCE(!slab_is_available()))"));
        assert!(source.contains("last_addr = phys_addr + size - 1;"));
        assert!(source.contains("if (!size || last_addr < phys_addr)"));
        assert!(source.contains("offset = phys_addr & (~PAGE_MASK);"));
        assert!(source.contains("size = PAGE_ALIGN(size + offset);"));
        assert!(source.contains("__get_vm_area_caller(size, VM_IOREMAP"));
        assert!(source.contains("ioremap_page_range(vaddr, vaddr + size, phys_addr, prot)"));
        assert!(source.contains("generic_iounmap"));
        assert!(source.contains("vunmap(vaddr);"));

        let mapping = generic_ioremap_prot(0x1234, 0x20, true, true, true).unwrap();
        assert_eq!(mapping.aligned_phys, 0x1000);
        assert_eq!(mapping.offset, 0x234);
        assert_eq!(mapping.aligned_size, PAGE_SIZE);
        assert!(generic_ioremap_prot(0x1000, 0, true, true, true).is_none());
        assert!(generic_ioremap_prot(0x1000, 0x20, false, true, true).is_none());
        assert!(generic_iounmap_should_vunmap(true));
    }

    #[test]
    fn registers_vendor_ioremap_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("ioremap_cache"),
            Some(linux_ioremap_cache as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("devm_ioremap"),
            Some(linux_devm_ioremap as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("memremap"),
            Some(linux_memremap as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("memunmap"),
            Some(linux_memunmap as usize)
        );
    }
}
