//! linux-parity: partial
//! linux-source: vendor/linux/lib/scatterlist.c
//! test-origin: linux:vendor/linux/lib/scatterlist.c
//! Scatterlist exports used by Linux-built modules.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

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

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sg_init_one", linux_sg_init_one as usize, true);
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
}

/// `sg_init_one` - `vendor/linux/lib/scatterlist.c`.
pub unsafe extern "C" fn linux_sg_init_one(
    sg: *mut LinuxScatterList,
    buf: *const c_void,
    len: u32,
) {
    if sg.is_null() {
        return;
    }
    unsafe {
        (*sg).page_link = (buf as usize & !SG_PAGE_LINK_MASK) | SG_END;
        (*sg).offset = 0;
        (*sg).length = len;
        (*sg).dma_address = buf as usize;
        (*sg).dma_length = len;
        (*sg).dma_flags = 0;
    }
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
            linux_sg_init_one(&mut sg, data.as_ptr().cast(), data.len() as u32);
            assert_eq!(sg.length, 4);
            assert_eq!(sg.page_link & SG_END, SG_END);
            assert_eq!(sg.dma_address, data.as_ptr() as usize);
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
}
