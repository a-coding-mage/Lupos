//! linux-parity: partial
//! linux-source: vendor/linux/drivers/dma-buf/dma-resv.c
//! test-origin: linux:vendor/linux/drivers/dma-buf/dma-resv.c
//! DMA reservation object ABI used by DRM and dma-buf modules.

use core::ffi::c_void;
use core::sync::atomic::AtomicU64;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::locking::mutex::linux_ww_mutex_init_raw;
use crate::kernel::module::{export_symbol, find_symbol};

const DMA_RESV_LOCK_OFFSET: usize = 0;
const DMA_RESV_FENCES_OFFSET: usize = 32;

const DMA_RESV_ITER_FENCE_OFFSET: usize = 16;
const DMA_RESV_ITER_FENCE_USAGE_OFFSET: usize = 24;
const DMA_RESV_ITER_INDEX_OFFSET: usize = 28;
const DMA_RESV_ITER_FENCES_OFFSET: usize = 32;
const DMA_RESV_ITER_NUM_FENCES_OFFSET: usize = 40;
const DMA_RESV_ITER_IS_RESTARTED_OFFSET: usize = 44;

#[repr(C)]
struct LinuxWwClass {
    stamp: AtomicU64,
    acquire_name: usize,
    mutex_name: usize,
    is_wait_die: u32,
    _pad: u32,
}

static mut LINUX_RESERVATION_WW_CLASS: LinuxWwClass = LinuxWwClass {
    stamp: AtomicU64::new(0),
    acquire_name: 0,
    mutex_name: 0,
    is_wait_die: 1,
    _pad: 0,
};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "reservation_ww_class",
        core::ptr::addr_of_mut!(LINUX_RESERVATION_WW_CLASS) as usize,
        false,
    );
    export_symbol_once("dma_resv_init", linux_dma_resv_init as usize, false);
    export_symbol_once("dma_resv_fini", linux_dma_resv_fini as usize, false);
    export_symbol_once(
        "dma_resv_test_signaled",
        linux_dma_resv_test_signaled as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_wait_timeout",
        linux_dma_resv_wait_timeout as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_get_singleton",
        linux_dma_resv_get_singleton as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_reserve_fences",
        linux_dma_resv_reserve_fences as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_add_fence",
        linux_dma_resv_add_fence as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_copy_fences",
        linux_dma_resv_copy_fences as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_iter_first_unlocked",
        linux_dma_resv_iter_first_unlocked as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_iter_next_unlocked",
        linux_dma_resv_iter_next_unlocked as usize,
        false,
    );
    export_symbol_once(
        "dma_resv_iter_first",
        linux_dma_resv_iter_first as usize,
        true,
    );
    export_symbol_once(
        "dma_resv_iter_next",
        linux_dma_resv_iter_next as usize,
        true,
    );
}

unsafe fn dma_resv_lock(obj: *mut c_void) -> *mut c_void {
    unsafe { obj.cast::<u8>().add(DMA_RESV_LOCK_OFFSET).cast() }
}

unsafe fn dma_resv_fences_slot(obj: *mut c_void) -> *mut *mut c_void {
    unsafe {
        obj.cast::<u8>()
            .add(DMA_RESV_FENCES_OFFSET)
            .cast::<*mut c_void>()
    }
}

unsafe fn write_cursor_ptr(cursor: *mut c_void, offset: usize, value: *mut c_void) {
    unsafe {
        cursor
            .cast::<u8>()
            .add(offset)
            .cast::<*mut c_void>()
            .write(value)
    };
}

unsafe fn write_cursor_u32(cursor: *mut c_void, offset: usize, value: u32) {
    unsafe { cursor.cast::<u8>().add(offset).cast::<u32>().write(value) };
}

unsafe fn write_cursor_bool(cursor: *mut c_void, offset: usize, value: bool) {
    unsafe { cursor.cast::<u8>().add(offset).cast::<bool>().write(value) };
}

unsafe fn dma_resv_iter_empty(cursor: *mut c_void, restarted: bool) {
    if cursor.is_null() {
        return;
    }
    unsafe {
        write_cursor_ptr(cursor, DMA_RESV_ITER_FENCE_OFFSET, core::ptr::null_mut());
        write_cursor_u32(cursor, DMA_RESV_ITER_FENCE_USAGE_OFFSET, 0);
        write_cursor_u32(cursor, DMA_RESV_ITER_INDEX_OFFSET, 0);
        write_cursor_ptr(cursor, DMA_RESV_ITER_FENCES_OFFSET, core::ptr::null_mut());
        write_cursor_u32(cursor, DMA_RESV_ITER_NUM_FENCES_OFFSET, 0);
        write_cursor_bool(cursor, DMA_RESV_ITER_IS_RESTARTED_OFFSET, restarted);
    }
}

/// `dma_resv_init` - `vendor/linux/drivers/dma-buf/dma-resv.c:122`.
pub unsafe extern "C" fn linux_dma_resv_init(obj: *mut c_void) {
    if obj.is_null() {
        return;
    }
    unsafe {
        linux_ww_mutex_init_raw(dma_resv_lock(obj));
        dma_resv_fences_slot(obj).write(core::ptr::null_mut());
    }
}

/// `dma_resv_fini` - `vendor/linux/drivers/dma-buf/dma-resv.c:133`.
pub unsafe extern "C" fn linux_dma_resv_fini(obj: *mut c_void) {
    if !obj.is_null() {
        unsafe { dma_resv_fences_slot(obj).write(core::ptr::null_mut()) };
    }
}

/// `dma_resv_test_signaled` - no fences are attached by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_test_signaled(_obj: *mut c_void, _usage: u32) -> bool {
    true
}

/// `dma_resv_wait_timeout` - no fences are attached by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_wait_timeout(
    _obj: *mut c_void,
    _usage: u32,
    _intr: bool,
    timeout: i64,
) -> i64 {
    core::cmp::max(timeout, 1)
}

/// `dma_resv_get_singleton` - no fences are attached by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_get_singleton(
    _obj: *mut c_void,
    _usage: u32,
    fence: *mut *mut c_void,
) -> i32 {
    if !fence.is_null() {
        unsafe { fence.write(core::ptr::null_mut()) };
    }
    0
}

/// `dma_resv_reserve_fences` - no fence array is needed by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_reserve_fences(obj: *mut c_void, num_fences: u32) -> i32 {
    if obj.is_null() || num_fences == 0 {
        return -EINVAL;
    }
    0
}

/// `dma_resv_add_fence` - no fences are attached by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_add_fence(
    _obj: *mut c_void,
    _fence: *mut c_void,
    _usage: u32,
) {
}

/// `dma_resv_copy_fences` - no fences are attached by this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_copy_fences(dst: *mut c_void, _src: *mut c_void) -> i32 {
    if dst.is_null() {
        return -EINVAL;
    }
    unsafe { dma_resv_fences_slot(dst).write(core::ptr::null_mut()) };
    0
}

/// `dma_resv_iter_first_unlocked` - empty iterator for this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_iter_first_unlocked(cursor: *mut c_void) -> *mut c_void {
    unsafe { dma_resv_iter_empty(cursor, true) };
    core::ptr::null_mut()
}

/// `dma_resv_iter_next_unlocked` - empty iterator for this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_iter_next_unlocked(cursor: *mut c_void) -> *mut c_void {
    unsafe { dma_resv_iter_empty(cursor, false) };
    core::ptr::null_mut()
}

/// `dma_resv_iter_first` - empty locked iterator for this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_iter_first(cursor: *mut c_void) -> *mut c_void {
    unsafe { dma_resv_iter_empty(cursor, true) };
    core::ptr::null_mut()
}

/// `dma_resv_iter_next` - empty locked iterator for this partial ABI.
pub unsafe extern "C" fn linux_dma_resv_iter_next(cursor: *mut c_void) -> *mut c_void {
    unsafe { dma_resv_iter_empty(cursor, false) };
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    #[test]
    fn reservation_class_matches_wait_die_initializer() {
        unsafe {
            let class = core::ptr::addr_of!(LINUX_RESERVATION_WW_CLASS);
            let stamp = core::ptr::addr_of!((*class).stamp);
            let is_wait_die = core::ptr::addr_of!((*class).is_wait_die);
            assert_eq!((*stamp).load(Ordering::Acquire), 0);
            assert_eq!(is_wait_die.read(), 1);
        }
    }

    #[test]
    fn dma_resv_init_sets_lock_and_empty_fence_list() {
        unsafe {
            let mut storage = [0xffu8; 40];
            linux_dma_resv_init(storage.as_mut_ptr().cast());
            let owner = storage.as_ptr().cast::<AtomicU64>();
            assert_eq!((*owner).load(Ordering::Acquire), 0);
            assert!(
                dma_resv_fences_slot(storage.as_mut_ptr().cast())
                    .read()
                    .is_null()
            );
        }
    }

    #[test]
    fn registers_vendor_dma_resv_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("dma_resv_reserve_fences"),
            Some(linux_dma_resv_reserve_fences as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_resv_add_fence"),
            Some(linux_dma_resv_add_fence as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("dma_resv_iter_first_unlocked"),
            Some(linux_dma_resv_iter_first_unlocked as usize)
        );
    }

    #[test]
    fn dma_resv_no_fence_model_accepts_reserve_and_copy() {
        unsafe {
            let mut dst = [0xffu8; 40];
            let mut src = [0xffu8; 40];
            linux_dma_resv_init(dst.as_mut_ptr().cast());
            linux_dma_resv_init(src.as_mut_ptr().cast());

            assert_eq!(linux_dma_resv_reserve_fences(dst.as_mut_ptr().cast(), 1), 0);
            assert_eq!(
                linux_dma_resv_reserve_fences(dst.as_mut_ptr().cast(), 0),
                -EINVAL
            );
            linux_dma_resv_add_fence(dst.as_mut_ptr().cast(), 0x1000usize as *mut c_void, 1);
            assert_eq!(
                linux_dma_resv_copy_fences(dst.as_mut_ptr().cast(), src.as_mut_ptr().cast()),
                0
            );
            assert!(
                dma_resv_fences_slot(dst.as_mut_ptr().cast())
                    .read()
                    .is_null()
            );
        }
    }

    #[test]
    fn dma_resv_iterators_return_empty() {
        unsafe {
            let mut cursor = [0xffu8; 48];
            assert!(linux_dma_resv_iter_first_unlocked(cursor.as_mut_ptr().cast()).is_null());
            assert!(
                cursor
                    .as_ptr()
                    .add(DMA_RESV_ITER_FENCE_OFFSET)
                    .cast::<*mut c_void>()
                    .read()
                    .is_null()
            );
            assert_eq!(
                cursor
                    .as_ptr()
                    .add(DMA_RESV_ITER_NUM_FENCES_OFFSET)
                    .cast::<u32>()
                    .read(),
                0
            );
            assert!(
                cursor
                    .as_ptr()
                    .add(DMA_RESV_ITER_IS_RESTARTED_OFFSET)
                    .cast::<bool>()
                    .read()
            );

            assert!(linux_dma_resv_iter_next(cursor.as_mut_ptr().cast()).is_null());
            assert!(
                !cursor
                    .as_ptr()
                    .add(DMA_RESV_ITER_IS_RESTARTED_OFFSET)
                    .cast::<bool>()
                    .read()
            );
        }
    }
}
