//! linux-parity: partial
//! linux-source: vendor/linux/lib/scatterlist.c
//! test-origin: linux:vendor/linux/lib/scatterlist.c
//! Scatterlist exports used by Linux-built modules.

use core::ffi::c_void;

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::buddy::page_to_pfn;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::page::Page;
use crate::mm::page_flags::GfpFlags;

pub const SG_CHAIN: usize = 0x01;
pub const SG_END: usize = 0x02;
pub const SG_PAGE_LINK_MASK: usize = SG_CHAIN | SG_END;

#[repr(C)]
pub struct LinuxScatterList {
    pub page_link: usize,
    pub offset: u32,
    pub length: u32,
    pub dma_address: usize,
    pub dma_length: u32,
    /// Present under the selected vendor `CONFIG_NEED_SG_DMA_FLAGS=y` ABI.
    pub dma_flags: u32,
}

#[repr(C)]
pub struct LinuxSgTable {
    pub sgl: *mut LinuxScatterList,
    pub nents: u32,
    pub orig_nents: u32,
}

#[repr(C)]
pub struct LinuxSgPageIter {
    pub sg: *mut LinuxScatterList,
    pub sg_pgoffset: u32,
    pub nents: u32,
    pub pg_advance: i32,
}

#[repr(C)]
pub struct LinuxSgDmaPageIter {
    pub base: LinuxSgPageIter,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sg_init_table", linux_sg_init_table as usize, true);
    export_symbol_once("sg_init_one", linux_sg_init_one as usize, true);
    export_symbol_once("sg_nents", linux_sg_nents as usize, false);
    export_symbol_once(
        "sg_alloc_table_chained",
        linux_sg_alloc_table_chained as usize,
        true,
    );
    export_symbol_once(
        "sg_free_table_chained",
        linux_sg_free_table_chained as usize,
        true,
    );
    export_symbol_once("__sg_alloc_table", linux___sg_alloc_table as usize, true);
    export_symbol_once("__sg_free_table", linux___sg_free_table as usize, true);
    export_symbol_once("sg_alloc_table", linux_sg_alloc_table as usize, true);
    export_symbol_once("sg_free_table", linux_sg_free_table as usize, true);
    export_symbol_once(
        "sg_alloc_table_from_pages_segment",
        linux_sg_alloc_table_from_pages_segment as usize,
        false,
    );
    export_symbol_once(
        "__sg_page_iter_start",
        linux___sg_page_iter_start as usize,
        false,
    );
    export_symbol_once(
        "__sg_page_iter_next",
        linux___sg_page_iter_next as usize,
        false,
    );
    export_symbol_once(
        "__sg_page_iter_dma_next",
        linux___sg_page_iter_dma_next as usize,
        false,
    );
}

/// `sg_init_table` - `vendor/linux/lib/scatterlist.c:130`.
///
/// The selected vendor configuration has `CONFIG_DEBUG_SG=n`, so the marker
/// initialization is exactly a zero fill followed by `SG_END` on the final
/// entry.
pub unsafe extern "C" fn linux_sg_init_table(sgl: *mut LinuxScatterList, nents: u32) {
    if sgl.is_null() || nents == 0 {
        return;
    }
    unsafe {
        core::ptr::write_bytes(sgl, 0, nents as usize);
        (*sgl.add(nents as usize - 1)).page_link |= SG_END;
    }
}

/// `sg_init_one` - `vendor/linux/lib/scatterlist.c`.
pub unsafe extern "C" fn linux_sg_init_one(
    sg: *mut LinuxScatterList,
    buf: *const c_void,
    len: u32,
) {
    if sg.is_null() || buf.is_null() {
        return;
    }
    let Some(phys) = crate::arch::x86::mm::paging::virt_to_phys(buf as u64) else {
        crate::log_warn!(
            "scatterlist",
            "sg_init_one rejected unmapped kernel buffer {:p}",
            buf
        );
        return;
    };
    let page = crate::mm::buddy::pfn_to_page((phys as usize) >> 12);
    unsafe {
        linux_sg_init_table(sg, 1);
        (*sg).page_link = (page as usize & !SG_PAGE_LINK_MASK) | SG_END;
        (*sg).offset = (phys as usize & 0xfff) as u32;
        (*sg).length = len;
    }
}

/// `sg_nents` - `vendor/linux/lib/scatterlist.c:25`.
#[unsafe(export_name = "sg_nents")]
pub unsafe extern "C" fn linux_sg_nents(mut sg: *mut LinuxScatterList) -> i32 {
    let mut nents = 0i32;
    while !sg.is_null() {
        nents = nents.saturating_add(1);
        sg = unsafe { linux_sg_next(sg) };
    }
    nents
}

/// `sg_alloc_table_chained` - `vendor/linux/lib/sg_pool.c`.
pub unsafe extern "C" fn linux_sg_alloc_table_chained(
    table: *mut LinuxSgTable,
    nents: u32,
    first_chunk: *mut LinuxScatterList,
    _nents_first_chunk: u32,
) -> i32 {
    if table.is_null() {
        return -22;
    }
    unsafe {
        (*table).sgl = first_chunk;
        (*table).nents = nents;
        (*table).orig_nents = nents;
    }
    0
}

unsafe fn alloc_sg_entries(nents: u32, gfp: GfpFlags) -> *mut LinuxScatterList {
    let Some(bytes) = (nents as usize).checked_mul(core::mem::size_of::<LinuxScatterList>()) else {
        return core::ptr::null_mut();
    };
    unsafe { crate::mm::slab::kmalloc(bytes, gfp).cast::<LinuxScatterList>() }
}

/// `__sg_alloc_table` - `vendor/linux/lib/scatterlist.c`.
#[unsafe(export_name = "__sg_alloc_table")]
pub unsafe extern "C" fn linux___sg_alloc_table(
    table: *mut LinuxSgTable,
    nents: u32,
    _max_ents: u32,
    first_chunk: *mut LinuxScatterList,
    nents_first_chunk: u32,
    gfp_mask: GfpFlags,
    _alloc_fn: *mut c_void,
) -> i32 {
    if table.is_null() || nents == 0 {
        return -EINVAL;
    }

    unsafe {
        (*table).sgl = core::ptr::null_mut();
        (*table).nents = 0;
        (*table).orig_nents = 0;
    }

    let sgl = if first_chunk.is_null() {
        let allocated = unsafe { alloc_sg_entries(nents, gfp_mask) };
        if allocated.is_null() {
            return -ENOMEM;
        }
        allocated
    } else if nents_first_chunk != 0 && nents > nents_first_chunk {
        return -ENOMEM;
    } else {
        first_chunk
    };

    unsafe {
        linux_sg_init_table(sgl, nents);
        (*table).sgl = sgl;
        (*table).nents = nents;
        (*table).orig_nents = nents;
    }
    0
}

/// `sg_alloc_table` - `vendor/linux/lib/scatterlist.c`.
#[unsafe(export_name = "sg_alloc_table")]
pub unsafe extern "C" fn linux_sg_alloc_table(
    table: *mut LinuxSgTable,
    nents: u32,
    gfp_mask: GfpFlags,
) -> i32 {
    unsafe {
        linux___sg_alloc_table(
            table,
            nents,
            0,
            core::ptr::null_mut(),
            0,
            gfp_mask,
            core::ptr::null_mut(),
        )
    }
}

/// `__sg_free_table` - `vendor/linux/lib/scatterlist.c`.
#[unsafe(export_name = "__sg_free_table")]
pub unsafe extern "C" fn linux___sg_free_table(
    table: *mut LinuxSgTable,
    _max_ents: u32,
    nents_first_chunk: u32,
    _free_fn: *mut c_void,
    _num_ents: u32,
) {
    if table.is_null() {
        return;
    }
    let sgl = unsafe { (*table).sgl };
    if !sgl.is_null() && nents_first_chunk == 0 {
        unsafe { crate::mm::slab::kfree(sgl.cast::<u8>()) };
    }
    unsafe {
        (*table).sgl = core::ptr::null_mut();
        (*table).nents = 0;
        (*table).orig_nents = 0;
    }
}

/// `sg_free_table` - `vendor/linux/lib/scatterlist.c`.
#[unsafe(export_name = "sg_free_table")]
pub unsafe extern "C" fn linux_sg_free_table(table: *mut LinuxSgTable) {
    let orig_nents = if table.is_null() {
        0
    } else {
        unsafe { (*table).orig_nents }
    };
    unsafe { linux___sg_free_table(table, 0, 0, core::ptr::null_mut(), orig_nents) };
}

fn page_align(size: usize) -> Option<usize> {
    size.checked_add(PAGE_SIZE - 1)
        .map(|size| size & !(PAGE_SIZE - 1))
}

unsafe fn linux_sg_next(sg: *mut LinuxScatterList) -> *mut LinuxScatterList {
    if sg.is_null() {
        return core::ptr::null_mut();
    }
    let page_link = unsafe { (*sg).page_link };
    if page_link & SG_END != 0 {
        core::ptr::null_mut()
    } else {
        let next = unsafe { sg.add(1) };
        let next_page_link = unsafe { (*next).page_link };
        if next_page_link & SG_CHAIN != 0 {
            (next_page_link & !SG_PAGE_LINK_MASK) as *mut LinuxScatterList
        } else {
            next
        }
    }
}

fn sg_page_count(sg: *const LinuxScatterList, dma: bool) -> u32 {
    if sg.is_null() {
        return 0;
    }
    let offset = unsafe { (*sg).offset as usize };
    let length = if dma {
        unsafe { (*sg).dma_length as usize }
    } else {
        unsafe { (*sg).length as usize }
    };
    page_align(offset.saturating_add(length))
        .map(|bytes| (bytes >> 12) as u32)
        .unwrap_or(0)
}

/// `sg_alloc_table_from_pages_segment` - `vendor/linux/lib/scatterlist.c:581`.
#[unsafe(export_name = "sg_alloc_table_from_pages_segment")]
pub unsafe extern "C" fn linux_sg_alloc_table_from_pages_segment(
    sgt: *mut LinuxSgTable,
    pages: *mut *mut Page,
    n_pages: u32,
    offset: u32,
    size: usize,
    max_segment: u32,
    gfp_mask: GfpFlags,
) -> i32 {
    if sgt.is_null()
        || pages.is_null()
        || n_pages == 0
        || offset as usize >= PAGE_SIZE
        || max_segment == 0
    {
        return -EINVAL;
    }

    let mut entries = 0usize;
    let mut remaining = size;
    let mut page_idx = 0usize;
    let mut page_offset = offset as usize;
    while remaining > 0 {
        if page_idx >= n_pages as usize {
            return -EINVAL;
        }
        let chunk = remaining
            .min(PAGE_SIZE - page_offset)
            .min(max_segment as usize);
        if chunk == 0 {
            return -EINVAL;
        }
        entries += 1;
        remaining -= chunk;
        page_offset += chunk;
        if page_offset == PAGE_SIZE {
            page_idx += 1;
            page_offset = 0;
        }
    }

    if entries == 0 {
        entries = 1;
    }
    let Ok(nents) = u32::try_from(entries) else {
        return -ENOMEM;
    };
    let ret = unsafe { linux_sg_alloc_table(sgt, nents, gfp_mask) };
    if ret != 0 {
        return ret;
    }

    let mut remaining = size;
    let mut page_idx = 0usize;
    let mut page_offset = offset as usize;
    for idx in 0..entries {
        let sg = unsafe { (*sgt).sgl.add(idx) };
        let page = unsafe { *pages.add(page_idx) };
        if page.is_null() {
            unsafe { linux_sg_free_table(sgt) };
            return -EINVAL;
        }
        let chunk = if remaining == 0 {
            0
        } else {
            remaining
                .min(PAGE_SIZE - page_offset)
                .min(max_segment as usize)
        };
        let Ok(length) = u32::try_from(chunk) else {
            unsafe { linux_sg_free_table(sgt) };
            return -EINVAL;
        };
        let dma = page_to_pfn(page)
            .checked_mul(PAGE_SIZE)
            .and_then(|addr| addr.checked_add(page_offset))
            .unwrap_or(0);
        unsafe {
            (*sg).page_link =
                (page as usize & !SG_PAGE_LINK_MASK) | if idx + 1 == entries { SG_END } else { 0 };
            (*sg).offset = page_offset as u32;
            (*sg).length = length;
            (*sg).dma_address = dma;
            (*sg).dma_length = length;
        }
        remaining = remaining.saturating_sub(chunk);
        page_offset += chunk;
        if page_offset == PAGE_SIZE {
            page_idx += 1;
            page_offset = 0;
        }
    }
    unsafe {
        (*sgt).nents = nents;
        (*sgt).orig_nents = nents;
    }
    0
}

/// `__sg_page_iter_start` - `vendor/linux/lib/scatterlist.c:727`.
#[unsafe(export_name = "__sg_page_iter_start")]
pub unsafe extern "C" fn linux___sg_page_iter_start(
    piter: *mut LinuxSgPageIter,
    sglist: *mut LinuxScatterList,
    nents: u32,
    pgoffset: usize,
) {
    if piter.is_null() {
        return;
    }
    unsafe {
        (*piter).pg_advance = 0;
        (*piter).nents = nents;
        (*piter).sg = sglist;
        (*piter).sg_pgoffset = pgoffset as u32;
    }
}

fn linux_sg_page_iter_next_common(piter: *mut LinuxSgPageIter, dma: bool) -> bool {
    if piter.is_null() {
        return false;
    }
    unsafe {
        if (*piter).nents == 0 || (*piter).sg.is_null() {
            return false;
        }
        (*piter).sg_pgoffset = (*piter)
            .sg_pgoffset
            .saturating_add((*piter).pg_advance.max(0) as u32);
        (*piter).pg_advance = 1;

        while (*piter).sg_pgoffset >= sg_page_count((*piter).sg, dma) {
            let count = sg_page_count((*piter).sg, dma);
            if count == 0 {
                return false;
            }
            (*piter).sg_pgoffset -= count;
            (*piter).sg = linux_sg_next((*piter).sg);
            (*piter).nents = (*piter).nents.saturating_sub(1);
            if (*piter).nents == 0 || (*piter).sg.is_null() {
                return false;
            }
        }
    }
    true
}

/// `__sg_page_iter_next` - `vendor/linux/lib/scatterlist.c:744`.
#[unsafe(export_name = "__sg_page_iter_next")]
pub unsafe extern "C" fn linux___sg_page_iter_next(piter: *mut LinuxSgPageIter) -> bool {
    linux_sg_page_iter_next_common(piter, false)
}

/// `__sg_page_iter_dma_next` - `vendor/linux/lib/scatterlist.c:768`.
#[unsafe(export_name = "__sg_page_iter_dma_next")]
pub unsafe extern "C" fn linux___sg_page_iter_dma_next(dma_iter: *mut LinuxSgDmaPageIter) -> bool {
    if dma_iter.is_null() {
        return false;
    }
    linux_sg_page_iter_next_common(unsafe { core::ptr::addr_of_mut!((*dma_iter).base) }, true)
}

/// `sg_free_table_chained` - `vendor/linux/lib/sg_pool.c`.
pub unsafe extern "C" fn linux_sg_free_table_chained(
    table: *mut LinuxSgTable,
    _nents_first_chunk: u32,
) {
    if !table.is_null() {
        unsafe {
            (*table).sgl = core::ptr::null_mut();
            (*table).nents = 0;
            (*table).orig_nents = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatterlist_exports_register_for_modules() {
        register_module_exports();
        for (name, addr) in [
            ("sg_init_one", linux_sg_init_one as usize),
            ("sg_nents", linux_sg_nents as usize),
            (
                "sg_alloc_table_chained",
                linux_sg_alloc_table_chained as usize,
            ),
            (
                "sg_free_table_chained",
                linux_sg_free_table_chained as usize,
            ),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
        }
    }

    #[test]
    fn scatterlist_table_uses_chained_first_chunk() {
        unsafe {
            let mut sg = LinuxScatterList {
                page_link: 0,
                offset: 0,
                length: 0,
                dma_address: 0,
                dma_length: 0,
                dma_flags: 0,
            };
            let data = [1u8; 4];
            linux_sg_init_table(&mut sg, 1);
            assert_eq!(sg.length, 0);
            assert_eq!(sg.page_link, SG_END);
            assert_eq!(sg.dma_address, 0);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, page_link), 0);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, offset), 0x8);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, length), 0xc);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, dma_address), 0x10);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, dma_length), 0x18);
            assert_eq!(core::mem::offset_of!(LinuxScatterList, dma_flags), 0x1c);
            assert_eq!(core::mem::size_of::<LinuxScatterList>(), 0x20);

            let mut table = LinuxSgTable {
                sgl: core::ptr::null_mut(),
                nents: 0,
                orig_nents: 0,
            };
            assert_eq!(linux_sg_alloc_table_chained(&mut table, 1, &mut sg, 1), 0);
            assert_eq!(table.sgl, &mut sg as *mut LinuxScatterList);
            linux_sg_free_table_chained(&mut table, 1);
            assert!(table.sgl.is_null());
        }
    }

    #[test]
    fn sg_nents_counts_until_end_marker() {
        let mut list = [
            LinuxScatterList {
                page_link: 0,
                offset: 0,
                length: 1,
                dma_address: 0,
                dma_length: 0,
                dma_flags: 0,
            },
            LinuxScatterList {
                page_link: 0,
                offset: 0,
                length: 1,
                dma_address: 0,
                dma_length: 0,
                dma_flags: 0,
            },
            LinuxScatterList {
                page_link: SG_END,
                offset: 0,
                length: 1,
                dma_address: 0,
                dma_length: 0,
                dma_flags: 0,
            },
        ];

        assert_eq!(unsafe { linux_sg_nents(list.as_mut_ptr()) }, 3);
    }
}
