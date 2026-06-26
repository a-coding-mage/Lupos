//! linux-parity: complete
//! linux-source: vendor/linux/mm/memremap.c
//! test-origin: linux:vendor/linux/mm/memremap.c
//! Device-memory remap visible ABI surface for the configured x86_64 target.

use crate::include::uapi::errno::ENXIO;
use crate::mm::page::Page;

#[inline]
fn err_ptr(errno: i32) -> *mut u8 {
    (-(errno as isize)) as *mut u8
}

pub fn memremap_compat_align() -> usize {
    crate::mm::frame::PAGE_SIZE
}

pub fn memremap_pages(_pgmap: *mut u8, _nid: i32) -> *mut u8 {
    err_ptr(ENXIO)
}

pub fn memunmap_pages(_pgmap: *mut u8) {}

pub fn devm_memremap_pages(_dev: *mut u8, pgmap: *mut u8) -> *mut u8 {
    memremap_pages(pgmap, 0)
}

pub fn devm_memunmap_pages(_dev: *mut u8, pgmap: *mut u8) {
    memunmap_pages(pgmap)
}

pub fn get_dev_pagemap(_pfn: u64, pgmap: *mut u8) -> *mut u8 {
    let _ = pgmap;
    core::ptr::null_mut()
}

pub fn put_dev_pagemap(_pgmap: *mut u8) {}

pub fn pgmap_altmap(_pgmap: *mut u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub fn pgmap_pfn_valid(_pgmap: *mut u8, _pfn: u64) -> bool {
    false
}

pub fn pgmap_vmemmap_nr(_pgmap: *mut u8) -> usize {
    0
}

pub fn pgmap_has_memory_failure(_pgmap: *mut u8) -> bool {
    false
}

pub fn zone_device_page_init(_page: *mut Page) {}

pub fn zone_device_folio_init(_folio: *mut Page) {}

pub fn zone_device_private_split_cb(_page: *mut Page, _nr_pages: usize) {}

pub fn folio_set_zone_device_data(folio: *mut Page, data: *mut u8) {
    if !folio.is_null() {
        unsafe { (*folio).private = data as usize };
    }
}

pub fn folio_zone_device_data(folio: *const Page) -> *mut u8 {
    if folio.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { (*folio).private as *mut u8 }
    }
}

pub fn is_device_private_page(_page: *const Page) -> bool {
    false
}
pub fn is_device_coherent_page(_page: *const Page) -> bool {
    false
}
pub fn is_pci_p2pdma_page(_page: *const Page) -> bool {
    false
}
pub fn is_fsdax_page(_page: *const Page) -> bool {
    false
}
pub fn folio_is_device_private(folio: *const Page) -> bool {
    is_device_private_page(folio)
}
pub fn folio_is_device_coherent(folio: *const Page) -> bool {
    is_device_coherent_page(folio)
}
pub fn folio_is_pci_p2pdma(folio: *const Page) -> bool {
    is_pci_p2pdma_page(folio)
}
pub fn folio_is_fsdax(folio: *const Page) -> bool {
    is_fsdax_page(folio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::frame::PAGE_SIZE;

    #[test]
    fn zone_device_disabled_returns_linux_disabled_shapes() {
        let ret = memremap_pages(core::ptr::null_mut(), 0);
        assert_eq!(ret as isize, -(ENXIO as isize));
        assert_eq!(
            devm_memremap_pages(core::ptr::null_mut(), core::ptr::null_mut()) as isize,
            -(ENXIO as isize)
        );
        assert!(get_dev_pagemap(42, 0x1234usize as *mut u8).is_null());
        assert!(!pgmap_pfn_valid(0x1234usize as *mut u8, 42));
        assert_eq!(memremap_compat_align(), PAGE_SIZE);
    }

    #[test]
    fn zone_device_page_type_helpers_match_disabled_config() {
        let mut page = Page::new();
        let ptr = &raw mut page;

        assert!(!is_device_private_page(ptr));
        assert!(!is_device_coherent_page(ptr));
        assert!(!is_pci_p2pdma_page(ptr));
        assert!(!is_fsdax_page(ptr));
        assert!(!folio_is_device_private(ptr));
        assert!(!folio_is_device_coherent(ptr));
        assert!(!folio_is_pci_p2pdma(ptr));
        assert!(!folio_is_fsdax(ptr));

        folio_set_zone_device_data(ptr, 0xfeedusize as *mut u8);
        assert_eq!(folio_zone_device_data(ptr) as usize, 0xfeed);
        zone_device_page_init(ptr);
        zone_device_folio_init(ptr);
        zone_device_private_split_cb(ptr, 1);
        memunmap_pages(core::ptr::null_mut());
        devm_memunmap_pages(core::ptr::null_mut(), core::ptr::null_mut());
        put_dev_pagemap(core::ptr::null_mut());
    }
}
