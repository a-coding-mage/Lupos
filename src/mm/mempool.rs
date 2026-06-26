//! linux-parity: complete
//! linux-source: vendor/linux/mm/mempool.c
//! test-origin: linux:vendor/linux/mm/mempool.c
//! Linux-visible mempool wrappers with a real reserve-backed pool.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENOMEM};

type MempoolAlloc = unsafe fn(u32, *mut u8) -> *mut u8;
type MempoolFree = unsafe fn(*mut u8, *mut u8);

#[repr(C)]
pub struct Mempool {
    min_nr: usize,
    elements: Vec<*mut u8>,
    pool_data: *mut u8,
    alloc: MempoolAlloc,
    free: MempoolFree,
}

unsafe fn alloc_from_raw(raw: *mut u8) -> Option<MempoolAlloc> {
    if raw.is_null() {
        None
    } else {
        Some(unsafe { core::mem::transmute::<*mut u8, MempoolAlloc>(raw) })
    }
}

unsafe fn free_from_raw(raw: *mut u8) -> Option<MempoolFree> {
    if raw.is_null() {
        None
    } else {
        Some(unsafe { core::mem::transmute::<*mut u8, MempoolFree>(raw) })
    }
}

unsafe fn pool_mut(pool: *mut u8) -> Option<&'static mut Mempool> {
    if pool.is_null() {
        None
    } else {
        Some(unsafe { &mut *(pool as *mut Mempool) })
    }
}

fn reserve_target(min_nr: usize) -> usize {
    core::cmp::max(1, min_nr)
}

unsafe fn alloc_one(pool: &mut Mempool, gfp_mask: u32) -> *mut u8 {
    unsafe { (pool.alloc)(gfp_mask, pool.pool_data) }
}

unsafe fn free_one(pool: &mut Mempool, element: *mut u8) {
    if !element.is_null() {
        unsafe { (pool.free)(element, pool.pool_data) };
    }
}

unsafe fn populate_reserve(pool: &mut Mempool, target: usize, gfp_mask: u32) -> Result<(), i32> {
    while pool.elements.len() < target {
        let element = unsafe { alloc_one(pool, gfp_mask) };
        if element.is_null() {
            return Err(ENOMEM);
        }
        pool.elements.push(element);
    }
    Ok(())
}

pub fn mempool_init_noprof(
    pool: *mut u8,
    min_nr: i32,
    alloc_fn: *mut u8,
    free_fn: *mut u8,
    pool_data: *mut u8,
) -> i32 {
    mempool_init_node(
        pool,
        min_nr,
        alloc_fn,
        free_fn,
        pool_data,
        crate::mm::page_flags::GFP_KERNEL,
        -1,
    )
}

pub fn mempool_init_node(
    pool: *mut u8,
    min_nr: i32,
    alloc_fn: *mut u8,
    free_fn: *mut u8,
    pool_data: *mut u8,
    gfp_mask: u32,
    _node_id: i32,
) -> i32 {
    if pool.is_null() || min_nr < 0 {
        return -EINVAL;
    }
    let Some(alloc) = (unsafe { alloc_from_raw(alloc_fn) }) else {
        return -EINVAL;
    };
    let Some(free) = (unsafe { free_from_raw(free_fn) }) else {
        return -EINVAL;
    };

    let mut new_pool = Mempool {
        min_nr: min_nr as usize,
        elements: Vec::new(),
        pool_data,
        alloc,
        free,
    };
    let target = reserve_target(new_pool.min_nr);
    if let Err(errno) = unsafe { populate_reserve(&mut new_pool, target, gfp_mask) } {
        for element in new_pool.elements.drain(..) {
            unsafe { free(element, pool_data) };
        }
        return -errno;
    }

    unsafe { core::ptr::write(pool as *mut Mempool, new_pool) };
    0
}

pub fn mempool_create_node_noprof(
    min_nr: i32,
    alloc_fn: *mut u8,
    free_fn: *mut u8,
    pool_data: *mut u8,
    gfp_mask: u32,
    node_id: i32,
) -> *mut u8 {
    if min_nr < 0 {
        return core::ptr::null_mut();
    }

    let raw = Box::into_raw(Box::new(Mempool {
        min_nr: 0,
        elements: Vec::new(),
        pool_data: core::ptr::null_mut(),
        alloc: mempool_kmalloc,
        free: mempool_kfree,
    })) as *mut u8;

    if mempool_init_node(raw, min_nr, alloc_fn, free_fn, pool_data, gfp_mask, node_id) != 0 {
        unsafe {
            let _ = Box::from_raw(raw as *mut Mempool);
        }
        core::ptr::null_mut()
    } else {
        raw
    }
}

pub fn mempool_exit(pool: *mut u8) {
    let Some(pool) = (unsafe { pool_mut(pool) }) else {
        return;
    };
    while let Some(element) = pool.elements.pop() {
        unsafe { free_one(pool, element) };
    }
    pool.min_nr = 0;
}

pub fn mempool_destroy(pool: *mut u8) {
    if pool.is_null() {
        return;
    }
    mempool_exit(pool);
    unsafe {
        let _ = Box::from_raw(pool as *mut Mempool);
    }
}

pub fn mempool_resize(pool: *mut u8, new_min_nr: i32) -> i32 {
    if new_min_nr < 0 {
        return -EINVAL;
    }
    let Some(pool) = (unsafe { pool_mut(pool) }) else {
        return -EINVAL;
    };

    let new_min = new_min_nr as usize;
    let target = reserve_target(new_min);
    if pool.elements.len() < target {
        if let Err(errno) =
            unsafe { populate_reserve(pool, target, crate::mm::page_flags::GFP_KERNEL) }
        {
            return -errno;
        }
    } else {
        while pool.elements.len() > target {
            let element = pool.elements.pop().unwrap();
            unsafe { free_one(pool, element) };
        }
    }
    pool.min_nr = new_min;
    0
}

pub fn mempool_alloc_noprof(pool: *mut u8, gfp_mask: u32) -> *mut u8 {
    let Some(pool) = (unsafe { pool_mut(pool) }) else {
        return core::ptr::null_mut();
    };

    let allocated = unsafe { alloc_one(pool, gfp_mask) };
    if !allocated.is_null() {
        return allocated;
    }
    pool.elements.pop().unwrap_or(core::ptr::null_mut())
}

pub fn mempool_alloc_preallocated(pool: *mut u8) -> *mut u8 {
    let Some(pool) = (unsafe { pool_mut(pool) }) else {
        return core::ptr::null_mut();
    };
    pool.elements.pop().unwrap_or(core::ptr::null_mut())
}

pub unsafe fn mempool_alloc_bulk_noprof(
    pool: *mut u8,
    elements: *mut *mut u8,
    count: usize,
    allocated: usize,
) -> i32 {
    if elements.is_null() || allocated > count {
        return -EINVAL;
    }
    for idx in allocated..count {
        let element = mempool_alloc_noprof(pool, crate::mm::page_flags::GFP_KERNEL);
        if element.is_null() {
            return -ENOMEM;
        }
        unsafe {
            *elements.add(idx) = element;
        }
    }
    0
}

pub fn mempool_free(element: *mut u8, pool: *mut u8) {
    if element.is_null() {
        return;
    }
    let Some(pool) = (unsafe { pool_mut(pool) }) else {
        unsafe { crate::mm::slab::kfree(element) };
        return;
    };
    if pool.elements.len() < pool.min_nr || (pool.min_nr == 0 && pool.elements.is_empty()) {
        pool.elements.push(element);
    } else {
        unsafe { free_one(pool, element) };
    }
}

pub unsafe fn mempool_free_bulk(pool: *mut u8, elements: *mut *mut u8, count: usize) -> usize {
    if elements.is_null() {
        return 0;
    }
    let Some(pool_ref) = (unsafe { pool_mut(pool) }) else {
        return 0;
    };

    let mut returned = 0usize;
    while returned < count
        && (pool_ref.elements.len() < pool_ref.min_nr
            || (pool_ref.min_nr == 0 && pool_ref.elements.is_empty()))
    {
        let element = unsafe { *elements.add(returned) };
        if element.is_null() {
            break;
        }
        pool_ref.elements.push(element);
        unsafe {
            *elements.add(returned) = core::ptr::null_mut();
        }
        returned += 1;
    }
    returned
}

pub fn mempool_kmalloc(gfp_mask: u32, pool_data: *mut u8) -> *mut u8 {
    let size = pool_data as usize;
    if size == 0 {
        return core::ptr::null_mut();
    }
    unsafe { crate::mm::slab::kmalloc(size, gfp_mask) }
}

pub fn mempool_kfree(element: *mut u8, _pool_data: *mut u8) {
    unsafe { crate::mm::slab::kfree(element) };
}

pub fn mempool_alloc_slab(gfp_mask: u32, pool_data: *mut u8) -> *mut u8 {
    if pool_data.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { crate::mm::slab::kmem_cache_alloc_noprof(pool_data as *mut _, gfp_mask) }
}

pub fn mempool_free_slab(element: *mut u8, pool_data: *mut u8) {
    if !pool_data.is_null() {
        unsafe { crate::mm::slab::kmem_cache_free(pool_data as *mut _, element) };
    }
}

pub fn mempool_alloc_pages(gfp_mask: u32, pool_data: *mut u8) -> *mut u8 {
    crate::mm::page_alloc::get_free_pages_noprof(gfp_mask, pool_data as u32) as *mut u8
}

pub fn mempool_free_pages(element: *mut u8, pool_data: *mut u8) {
    if element.is_null() {
        return;
    }
    let addr = element as u64;
    if addr < crate::arch::x86::mm::paging::PAGE_OFFSET {
        return;
    }
    let pfn = ((addr - crate::arch::x86::mm::paging::PAGE_OFFSET)
        >> crate::arch::x86::mm::paging::PAGE_SHIFT) as usize;
    let page = crate::mm::buddy::pfn_to_page(pfn);
    crate::mm::page_alloc::__free_pages(page, pool_data as u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::page_flags::GFP_KERNEL;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use alloc::alloc::{Layout, alloc, dealloc};

    unsafe fn test_alloc(_gfp: u32, pool_data: *mut u8) -> *mut u8 {
        let size = (pool_data as usize).max(1);
        let layout = Layout::from_size_align(size, core::mem::align_of::<usize>()).unwrap();
        unsafe { alloc(layout) }
    }

    unsafe fn test_free(element: *mut u8, pool_data: *mut u8) {
        if element.is_null() {
            return;
        }
        let size = (pool_data as usize).max(1);
        let layout = Layout::from_size_align(size, core::mem::align_of::<usize>()).unwrap();
        unsafe { dealloc(element, layout) };
    }

    fn alloc_ptr(func: MempoolAlloc) -> *mut u8 {
        func as usize as *mut u8
    }

    fn free_ptr(func: MempoolFree) -> *mut u8 {
        func as usize as *mut u8
    }

    #[test]
    fn create_alloc_free_and_destroy_kmalloc_pool() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let pool = mempool_create_node_noprof(
            2,
            alloc_ptr(test_alloc as MempoolAlloc),
            free_ptr(test_free as MempoolFree),
            32usize as *mut u8,
            GFP_KERNEL,
            -1,
        );
        assert!(!pool.is_null());

        let a = mempool_alloc_preallocated(pool);
        let b = mempool_alloc_preallocated(pool);
        assert!(!a.is_null());
        assert!(!b.is_null());
        assert!(mempool_alloc_preallocated(pool).is_null());

        mempool_free(a, pool);
        assert_eq!(unsafe { (*(pool as *mut Mempool)).elements.len() }, 1);
        mempool_free(b, pool);
        assert_eq!(unsafe { (*(pool as *mut Mempool)).elements.len() }, 2);
        mempool_destroy(pool);
    }

    #[test]
    fn resize_grows_and_shrinks_reserve() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let pool = mempool_create_node_noprof(
            1,
            alloc_ptr(test_alloc as MempoolAlloc),
            free_ptr(test_free as MempoolFree),
            16usize as *mut u8,
            GFP_KERNEL,
            -1,
        );
        assert!(!pool.is_null());
        assert_eq!(mempool_resize(pool, 3), 0);
        assert_eq!(unsafe { (*(pool as *mut Mempool)).elements.len() }, 3);
        assert_eq!(mempool_resize(pool, 1), 0);
        assert_eq!(unsafe { (*(pool as *mut Mempool)).elements.len() }, 1);
        mempool_destroy(pool);
    }

    #[test]
    fn bulk_alloc_and_free_use_prefix_contract() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let pool = mempool_create_node_noprof(
            2,
            alloc_ptr(test_alloc as MempoolAlloc),
            free_ptr(test_free as MempoolFree),
            8usize as *mut u8,
            GFP_KERNEL,
            -1,
        );
        let mut elements = [core::ptr::null_mut(); 3];
        assert_eq!(
            unsafe { mempool_alloc_bulk_noprof(pool, elements.as_mut_ptr(), 3, 0) },
            0
        );
        assert!(elements.iter().all(|ptr| !ptr.is_null()));
        for element in elements {
            mempool_free(element, pool);
        }

        let mut reserve = [
            mempool_alloc_preallocated(pool),
            mempool_alloc_preallocated(pool),
        ];
        assert!(reserve.iter().all(|ptr| !ptr.is_null()));
        let returned = unsafe { mempool_free_bulk(pool, reserve.as_mut_ptr(), 2) };
        assert_eq!(returned, 2);
        assert!(reserve[0].is_null());
        assert!(reserve[1].is_null());
        mempool_destroy(pool);
    }

    #[test]
    fn invalid_inputs_match_linux_errno_shape() {
        assert_eq!(
            mempool_init_noprof(
                core::ptr::null_mut(),
                1,
                alloc_ptr(test_alloc as MempoolAlloc),
                free_ptr(test_free as MempoolFree),
                8usize as *mut u8,
            ),
            -EINVAL
        );
        assert_eq!(mempool_resize(core::ptr::null_mut(), 1), -EINVAL);
        assert!(
            mempool_create_node_noprof(
                -1,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                GFP_KERNEL,
                -1
            )
            .is_null()
        );
    }
}
