//! linux-parity: partial
//! linux-source: vendor/linux/drivers/gpu/buddy.c
//! test-origin: linux:vendor/linux/drivers/gpu/buddy.c
//! Core GPU buddy allocator ABI used by vendor-built DRM drivers.
//!
//! Lupos loads the GPU drivers from `vendor/linux`; this module only resolves
//! the generic kernel-side `gpu_buddy_*` symbols those modules import. Until
//! Lupos has a real GPU memory manager, allocations fail closed instead of
//! fabricating VRAM blocks.

use core::ffi::c_void;

use crate::include::uapi::errno::{EINVAL, ENOSPC};
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("gpu_buddy_init", linux_gpu_buddy_init as usize, false);
    export_symbol_once("gpu_buddy_fini", linux_gpu_buddy_fini as usize, false);
    export_symbol_once(
        "gpu_buddy_alloc_blocks",
        linux_gpu_buddy_alloc_blocks as usize,
        false,
    );
    export_symbol_once(
        "gpu_buddy_free_list",
        linux_gpu_buddy_free_list as usize,
        false,
    );
}

#[repr(C)]
struct LinuxGpuBuddy {
    free_trees: *mut c_void,
    roots: *mut c_void,
    n_roots: u32,
    max_order: u32,
    chunk_size: u64,
    size: u64,
    avail: u64,
    clear_avail: u64,
}

unsafe extern "C" fn linux_gpu_buddy_init(
    mm: *mut LinuxGpuBuddy,
    size: u64,
    chunk_size: u64,
) -> i32 {
    if mm.is_null() || size == 0 || chunk_size == 0 {
        return -EINVAL;
    }

    unsafe {
        (*mm).free_trees = core::ptr::null_mut();
        (*mm).roots = core::ptr::null_mut();
        (*mm).n_roots = 0;
        (*mm).max_order = 0;
        (*mm).chunk_size = chunk_size;
        (*mm).size = size;
        (*mm).avail = 0;
        (*mm).clear_avail = 0;
    }
    0
}

unsafe extern "C" fn linux_gpu_buddy_fini(mm: *mut LinuxGpuBuddy) {
    if mm.is_null() {
        return;
    }
    unsafe {
        (*mm).free_trees = core::ptr::null_mut();
        (*mm).roots = core::ptr::null_mut();
        (*mm).n_roots = 0;
        (*mm).max_order = 0;
        (*mm).chunk_size = 0;
        (*mm).size = 0;
        (*mm).avail = 0;
        (*mm).clear_avail = 0;
    }
}

unsafe extern "C" fn linux_gpu_buddy_alloc_blocks(
    _mm: *mut LinuxGpuBuddy,
    _start: u64,
    _end: u64,
    _size: u64,
    _min_block_size: u64,
    _blocks: *mut c_void,
    _flags: u64,
) -> i32 {
    -ENOSPC
}

unsafe extern "C" fn linux_gpu_buddy_free_list(
    _mm: *mut LinuxGpuBuddy,
    _objects: *mut c_void,
    _flags: u64,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_gpu_buddy_symbols() {
        register_module_exports();
        assert_eq!(
            find_symbol("gpu_buddy_alloc_blocks"),
            Some(linux_gpu_buddy_alloc_blocks as usize)
        );
        assert_eq!(
            find_symbol("gpu_buddy_init"),
            Some(linux_gpu_buddy_init as usize)
        );
    }

    #[test]
    fn init_records_shape_and_alloc_fails_closed() {
        let mut mm = LinuxGpuBuddy {
            free_trees: 1usize as *mut c_void,
            roots: 1usize as *mut c_void,
            n_roots: 7,
            max_order: 9,
            chunk_size: 0,
            size: 0,
            avail: 123,
            clear_avail: 456,
        };

        assert_eq!(
            unsafe { linux_gpu_buddy_init(&mut mm, 16 * 1024 * 1024, 4096) },
            0
        );
        assert_eq!(mm.chunk_size, 4096);
        assert_eq!(mm.size, 16 * 1024 * 1024);
        assert_eq!(mm.avail, 0);
        assert_eq!(
            unsafe {
                linux_gpu_buddy_alloc_blocks(
                    &mut mm,
                    0,
                    mm.size,
                    4096,
                    4096,
                    core::ptr::null_mut(),
                    0,
                )
            },
            -ENOSPC
        );
    }
}
