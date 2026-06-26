//! linux-parity: complete
//! linux-source: vendor/linux/lib/min_heap.c
//! test-origin: linux:vendor/linux/lib/min_heap.c
//! Byte-oriented min-heap helpers matching Linux callback semantics.

use core::ffi::c_void;
use core::ptr::{copy_nonoverlapping, read, write};

use crate::kernel::module::{export_symbol, find_symbol};

pub type MinHeapLess =
    unsafe extern "C" fn(lhs: *const c_void, rhs: *const c_void, args: *mut c_void) -> bool;
pub type MinHeapSwap = unsafe extern "C" fn(lhs: *mut c_void, rhs: *mut c_void, args: *mut c_void);

#[repr(C)]
#[derive(Debug)]
pub struct MinHeapChar {
    pub nr: usize,
    pub size: usize,
    pub data: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MinHeapCallbacks {
    pub less: Option<MinHeapLess>,
    pub swp: Option<MinHeapSwap>,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__min_heap_init", __min_heap_init as usize, false);
    export_symbol_once("__min_heap_peek", __min_heap_peek as usize, false);
    export_symbol_once("__min_heap_full", __min_heap_full as usize, false);
    export_symbol_once("__min_heap_sift_down", __min_heap_sift_down as usize, false);
    export_symbol_once("__min_heap_sift_up", __min_heap_sift_up as usize, false);
    export_symbol_once("__min_heapify_all", __min_heapify_all as usize, false);
    export_symbol_once("__min_heap_pop", __min_heap_pop as usize, false);
    export_symbol_once("__min_heap_pop_push", __min_heap_pop_push as usize, false);
    export_symbol_once("__min_heap_push", __min_heap_push as usize, false);
    export_symbol_once("__min_heap_del", __min_heap_del as usize, false);
}

unsafe fn elem(heap: *mut MinHeapChar, idx: usize, elem_size: usize) -> *mut u8 {
    let data = unsafe { (*heap).data };
    unsafe { data.add(idx * elem_size) }
}

unsafe fn less(
    func: *const MinHeapCallbacks,
    lhs: *const u8,
    rhs: *const u8,
    args: *mut c_void,
) -> bool {
    let Some(callback) = (unsafe { &*func }).less else {
        return false;
    };
    unsafe { callback(lhs.cast(), rhs.cast(), args) }
}

unsafe fn do_swap(
    a: *mut u8,
    b: *mut u8,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    if let Some(callback) = unsafe { (&*func).swp } {
        unsafe { callback(a.cast(), b.cast(), args) };
        return;
    }

    let mut offset = 0usize;
    while offset < elem_size {
        let lhs = unsafe { a.add(offset) };
        let rhs = unsafe { b.add(offset) };
        let tmp = unsafe { read(lhs) };
        unsafe {
            write(lhs, read(rhs));
            write(rhs, tmp);
        }
        offset += 1;
    }
}

unsafe fn copy_elem(dst: *mut u8, src: *const u8, elem_size: usize) {
    unsafe { copy_nonoverlapping(src, dst, elem_size) };
}

unsafe fn sift_down(
    heap: *mut MinHeapChar,
    mut pos: usize,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    loop {
        let left = pos * 2 + 1;
        let right = left + 1;
        let nr = unsafe { (*heap).nr };
        if left >= nr {
            break;
        }

        let mut child = left;
        if right < nr
            && unsafe {
                less(
                    func,
                    elem(heap, right, elem_size),
                    elem(heap, left, elem_size),
                    args,
                )
            }
        {
            child = right;
        }

        if !unsafe {
            less(
                func,
                elem(heap, child, elem_size),
                elem(heap, pos, elem_size),
                args,
            )
        } {
            break;
        }

        unsafe {
            do_swap(
                elem(heap, pos, elem_size),
                elem(heap, child, elem_size),
                elem_size,
                func,
                args,
            )
        };
        pos = child;
    }
}

unsafe fn sift_up(
    heap: *mut MinHeapChar,
    elem_size: usize,
    mut idx: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    while idx != 0 {
        let parent = (idx - 1) / 2;
        if !unsafe {
            less(
                func,
                elem(heap, idx, elem_size),
                elem(heap, parent, elem_size),
                args,
            )
        } {
            break;
        }
        unsafe {
            do_swap(
                elem(heap, idx, elem_size),
                elem(heap, parent, elem_size),
                elem_size,
                func,
                args,
            )
        };
        idx = parent;
    }
}

pub unsafe extern "C" fn __min_heap_init(heap: *mut MinHeapChar, data: *mut c_void, size: usize) {
    if let Some(heap) = unsafe { heap.as_mut() } {
        heap.nr = 0;
        heap.size = size;
        heap.data = data.cast();
    }
}

pub unsafe extern "C" fn __min_heap_peek(heap: *mut MinHeapChar) -> *mut c_void {
    let Some(heap) = (unsafe { heap.as_ref() }) else {
        return core::ptr::null_mut();
    };
    if heap.nr == 0 {
        core::ptr::null_mut()
    } else {
        heap.data.cast()
    }
}

pub unsafe extern "C" fn __min_heap_full(heap: *mut MinHeapChar) -> bool {
    let Some(heap) = (unsafe { heap.as_ref() }) else {
        return false;
    };
    heap.nr == heap.size
}

pub unsafe extern "C" fn __min_heap_sift_down(
    heap: *mut MinHeapChar,
    pos: usize,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    if heap.is_null() || func.is_null() || elem_size == 0 {
        return;
    }
    unsafe { sift_down(heap, pos, elem_size, func, args) };
}

pub unsafe extern "C" fn __min_heap_sift_up(
    heap: *mut MinHeapChar,
    elem_size: usize,
    idx: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    if heap.is_null() || func.is_null() || elem_size == 0 {
        return;
    }
    unsafe { sift_up(heap, elem_size, idx, func, args) };
}

pub unsafe extern "C" fn __min_heapify_all(
    heap: *mut MinHeapChar,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    if heap.is_null() || func.is_null() || elem_size == 0 {
        return;
    }
    let nr = unsafe { (*heap).nr };
    let mut i = nr / 2;
    while i != 0 {
        i -= 1;
        unsafe { sift_down(heap, i, elem_size, func, args) };
    }
}

pub unsafe extern "C" fn __min_heap_pop(
    heap: *mut MinHeapChar,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) -> bool {
    if heap.is_null() || func.is_null() || elem_size == 0 || unsafe { (*heap).nr } == 0 {
        return false;
    }

    unsafe {
        (*heap).nr -= 1;
        let last = elem(heap, (*heap).nr, elem_size);
        copy_elem(elem(heap, 0, elem_size), last, elem_size);
        if (*heap).nr != 0 {
            sift_down(heap, 0, elem_size, func, args);
        }
    }
    true
}

pub unsafe extern "C" fn __min_heap_pop_push(
    heap: *mut MinHeapChar,
    element: *const c_void,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) {
    if heap.is_null() || element.is_null() || func.is_null() || elem_size == 0 {
        return;
    }
    unsafe {
        copy_elem(elem(heap, 0, elem_size), element.cast(), elem_size);
        if (*heap).nr != 0 {
            sift_down(heap, 0, elem_size, func, args);
        }
    }
}

pub unsafe extern "C" fn __min_heap_push(
    heap: *mut MinHeapChar,
    element: *const c_void,
    elem_size: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) -> bool {
    if heap.is_null() || element.is_null() || func.is_null() || elem_size == 0 {
        return false;
    }

    unsafe {
        if (*heap).nr >= (*heap).size {
            return false;
        }
        let pos = (*heap).nr;
        copy_elem(elem(heap, pos, elem_size), element.cast(), elem_size);
        (*heap).nr += 1;
        sift_up(heap, elem_size, pos, func, args);
    }
    true
}

pub unsafe extern "C" fn __min_heap_del(
    heap: *mut MinHeapChar,
    elem_size: usize,
    idx: usize,
    func: *const MinHeapCallbacks,
    args: *mut c_void,
) -> bool {
    if heap.is_null() || func.is_null() || elem_size == 0 {
        return false;
    }

    unsafe {
        if (*heap).nr == 0 || idx >= (*heap).nr {
            return false;
        }
        (*heap).nr -= 1;
        if idx == (*heap).nr {
            return true;
        }
        do_swap(
            elem(heap, idx, elem_size),
            elem(heap, (*heap).nr, elem_size),
            elem_size,
            func,
            args,
        );
        sift_up(heap, elem_size, idx, func, args);
        sift_down(heap, idx, elem_size, func, args);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn less_i32(
        lhs: *const c_void,
        rhs: *const c_void,
        _args: *mut c_void,
    ) -> bool {
        let lhs = unsafe { *(lhs.cast::<i32>()) };
        let rhs = unsafe { *(rhs.cast::<i32>()) };
        lhs < rhs
    }

    #[test]
    fn min_heap_raw_callbacks_match_linux_wrappers() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/min_heap.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/min_heap.h"
        ));
        assert!(source.contains("__min_heap_init_inline(heap, data, size);"));
        assert!(source.contains("__min_heap_sift_down_inline"));
        assert!(source.contains("EXPORT_SYMBOL(__min_heap_push);"));
        assert!(source.contains("EXPORT_SYMBOL(__min_heap_del);"));
        assert!(header.contains("struct min_heap_callbacks"));
        assert!(header.contains("bool (*less)(const void *lhs, const void *rhs, void *args);"));

        let mut storage = [0i32; 8];
        let mut heap = MinHeapChar {
            nr: 0,
            size: 0,
            data: core::ptr::null_mut(),
        };
        let callbacks = MinHeapCallbacks {
            less: Some(less_i32),
            swp: None,
        };

        unsafe {
            __min_heap_init(&mut heap, storage.as_mut_ptr().cast(), storage.len());
            for value in [7i32, 3, 5, 1, 9] {
                assert!(__min_heap_push(
                    &mut heap,
                    (&value as *const i32).cast(),
                    core::mem::size_of::<i32>(),
                    &callbacks,
                    core::ptr::null_mut(),
                ));
            }
            assert_eq!(*(__min_heap_peek(&mut heap).cast::<i32>()), 1);
            assert!(__min_heap_del(
                &mut heap,
                core::mem::size_of::<i32>(),
                0,
                &callbacks,
                core::ptr::null_mut(),
            ));
            assert_eq!(*(__min_heap_peek(&mut heap).cast::<i32>()), 3);
            assert!(__min_heap_pop(
                &mut heap,
                core::mem::size_of::<i32>(),
                &callbacks,
                core::ptr::null_mut(),
            ));
            assert_eq!(*(__min_heap_peek(&mut heap).cast::<i32>()), 5);
        }
    }
}
