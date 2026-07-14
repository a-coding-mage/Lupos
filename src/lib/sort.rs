//! linux-parity: partial
//! linux-source: vendor/linux/lib/sort.c
//! test-origin: linux:vendor/linux/lib/sort.c
//! Generic in-place sort ABI exports used by Linux-built modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

type CmpFunc = unsafe extern "C" fn(a: *const c_void, b: *const c_void) -> i32;
type SwapFunc = unsafe extern "C" fn(a: *mut c_void, b: *mut c_void, size: i32);
type CmpRFunc =
    unsafe extern "C" fn(a: *const c_void, b: *const c_void, priv_data: *const c_void) -> i32;
type SwapRFunc =
    unsafe extern "C" fn(a: *mut c_void, b: *mut c_void, size: i32, priv_data: *const c_void);
type ListCmpFunc = unsafe extern "C" fn(
    priv_data: *mut c_void,
    a: *const LinuxListHead,
    b: *const LinuxListHead,
) -> i32;

#[repr(C)]
pub struct LinuxListHead {
    pub next: *mut LinuxListHead,
    pub prev: *mut LinuxListHead,
}

#[derive(Clone, Copy)]
struct ListSortNode {
    ptr: *mut LinuxListHead,
    original_index: usize,
}

enum LinuxCmp {
    Plain(CmpFunc),
    WithPriv(CmpRFunc, *const c_void),
}

enum LinuxSwap {
    Default,
    Plain(SwapFunc),
    WithPriv(SwapRFunc, *const c_void),
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("sort", linux_sort as usize, false);
    export_symbol_once("sort_nonatomic", linux_sort_nonatomic as usize, false);
    export_symbol_once("sort_r", linux_sort_r as usize, false);
    export_symbol_once("sort_r_nonatomic", linux_sort_r_nonatomic as usize, false);
    export_symbol_once("list_sort", linux_list_sort as usize, false);
}

#[inline]
unsafe fn elem(base: *mut u8, index: usize, size: usize) -> *mut c_void {
    unsafe { base.add(index.saturating_mul(size)).cast::<c_void>() }
}

unsafe fn default_swap(a: *mut c_void, b: *mut c_void, size: usize) {
    if a == b {
        return;
    }
    let a = a.cast::<u8>();
    let b = b.cast::<u8>();
    for offset in 0..size {
        let tmp = unsafe { a.add(offset).read() };
        unsafe {
            a.add(offset).write(b.add(offset).read());
            b.add(offset).write(tmp);
        }
    }
}

unsafe fn do_cmp(cmp: &LinuxCmp, a: *const c_void, b: *const c_void) -> i32 {
    match *cmp {
        LinuxCmp::Plain(func) => unsafe { func(a, b) },
        LinuxCmp::WithPriv(func, priv_data) => unsafe { func(a, b, priv_data) },
    }
}

unsafe fn do_swap(swap: &LinuxSwap, a: *mut c_void, b: *mut c_void, size: usize) {
    match *swap {
        LinuxSwap::Default => unsafe { default_swap(a, b, size) },
        LinuxSwap::Plain(func) => unsafe { func(a, b, size as i32) },
        LinuxSwap::WithPriv(func, priv_data) => unsafe { func(a, b, size as i32, priv_data) },
    }
}

unsafe fn sift_down(
    base: *mut u8,
    mut root: usize,
    end: usize,
    size: usize,
    cmp: &LinuxCmp,
    swap: &LinuxSwap,
) {
    loop {
        let child = root.saturating_mul(2).saturating_add(1);
        if child > end {
            break;
        }

        let mut selected = child;
        if child < end
            && unsafe {
                do_cmp(
                    cmp,
                    elem(base, child, size).cast_const(),
                    elem(base, child + 1, size).cast_const(),
                )
            } < 0
        {
            selected = child + 1;
        }

        if unsafe {
            do_cmp(
                cmp,
                elem(base, root, size).cast_const(),
                elem(base, selected, size).cast_const(),
            )
        } >= 0
        {
            break;
        }

        unsafe {
            do_swap(
                swap,
                elem(base, root, size),
                elem(base, selected, size),
                size,
            )
        };
        root = selected;
    }
}

unsafe fn sort_impl(base: *mut c_void, num: usize, size: usize, cmp: LinuxCmp, swap: LinuxSwap) {
    if base.is_null() || num < 2 || size == 0 {
        return;
    }

    let base = base.cast::<u8>();
    let mut start = num / 2;
    while start > 0 {
        start -= 1;
        unsafe { sift_down(base, start, num - 1, size, &cmp, &swap) };
    }

    let mut end = num - 1;
    while end > 0 {
        unsafe { do_swap(&swap, elem(base, 0, size), elem(base, end, size), size) };
        end -= 1;
        unsafe { sift_down(base, 0, end, size, &cmp, &swap) };
    }
}

/// `sort` - `vendor/linux/lib/sort.c:333`.
#[unsafe(export_name = "sort")]
pub unsafe extern "C" fn linux_sort(
    base: *mut c_void,
    num: usize,
    size: usize,
    cmp_func: Option<CmpFunc>,
    swap_func: Option<SwapFunc>,
) {
    let Some(cmp_func) = cmp_func else {
        return;
    };
    let swap = swap_func.map_or(LinuxSwap::Default, LinuxSwap::Plain);
    unsafe { sort_impl(base, num, size, LinuxCmp::Plain(cmp_func), swap) };
}

/// `sort_nonatomic` - `vendor/linux/lib/sort.c:346`.
#[unsafe(export_name = "sort_nonatomic")]
pub unsafe extern "C" fn linux_sort_nonatomic(
    base: *mut c_void,
    num: usize,
    size: usize,
    cmp_func: Option<CmpFunc>,
    swap_func: Option<SwapFunc>,
) {
    unsafe { linux_sort(base, num, size, cmp_func, swap_func) };
}

/// `sort_r` - `vendor/linux/lib/sort.c:303`.
#[unsafe(export_name = "sort_r")]
pub unsafe extern "C" fn linux_sort_r(
    base: *mut c_void,
    num: usize,
    size: usize,
    cmp_func: Option<CmpRFunc>,
    swap_func: Option<SwapRFunc>,
    priv_data: *const c_void,
) {
    let Some(cmp_func) = cmp_func else {
        return;
    };
    let swap = swap_func.map_or(LinuxSwap::Default, |func| {
        LinuxSwap::WithPriv(func, priv_data)
    });
    unsafe {
        sort_impl(
            base,
            num,
            size,
            LinuxCmp::WithPriv(cmp_func, priv_data),
            swap,
        )
    };
}

/// `sort_r_nonatomic` - `vendor/linux/lib/sort.c:324`.
#[unsafe(export_name = "sort_r_nonatomic")]
pub unsafe extern "C" fn linux_sort_r_nonatomic(
    base: *mut c_void,
    num: usize,
    size: usize,
    cmp_func: Option<CmpRFunc>,
    swap_func: Option<SwapRFunc>,
    priv_data: *const c_void,
) {
    unsafe { linux_sort_r(base, num, size, cmp_func, swap_func, priv_data) };
}

unsafe fn list_sort_left_after_right(
    priv_data: *mut c_void,
    cmp: ListCmpFunc,
    left: ListSortNode,
    right: ListSortNode,
) -> bool {
    if left.original_index < right.original_index {
        unsafe { cmp(priv_data, left.ptr.cast_const(), right.ptr.cast_const()) > 0 }
    } else {
        unsafe { cmp(priv_data, right.ptr.cast_const(), left.ptr.cast_const()) <= 0 }
    }
}

/// `list_sort` - `vendor/linux/lib/list_sort.c`.
pub unsafe extern "C" fn linux_list_sort(
    priv_data: *mut c_void,
    head: *mut LinuxListHead,
    cmp: Option<ListCmpFunc>,
) {
    let Some(cmp) = cmp else {
        return;
    };
    if head.is_null() {
        return;
    }

    let first = unsafe { (*head).next };
    let last = unsafe { (*head).prev };
    if first.is_null() || first == last {
        return;
    }

    let mut nodes: Vec<ListSortNode> = Vec::new();
    let mut current = first;
    while current != head {
        if current.is_null() || nodes.try_reserve_exact(1).is_err() {
            return;
        }
        let original_index = nodes.len();
        nodes.push(ListSortNode {
            ptr: current,
            original_index,
        });
        current = unsafe { (*current).next };
    }

    for index in 1..nodes.len() {
        let mut cursor = index;
        while cursor > 0
            && unsafe {
                list_sort_left_after_right(priv_data, cmp, nodes[cursor - 1], nodes[cursor])
            }
        {
            nodes.swap(cursor - 1, cursor);
            cursor -= 1;
        }
    }

    let mut prev = head;
    for node in nodes {
        unsafe {
            (*node.ptr).prev = prev;
            (*prev).next = node.ptr;
        }
        prev = node.ptr;
    }
    unsafe {
        (*prev).next = head;
        (*head).prev = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> i32 {
        let a = unsafe { *(a as *const i32) };
        let b = unsafe { *(b as *const i32) };
        if a > b {
            1
        } else if a < b {
            -1
        } else {
            0
        }
    }

    #[test]
    fn sort_orders_integers_ascending() {
        let mut values = [7, -3, 4, 4, 0, 19, -8];
        unsafe {
            linux_sort(
                values.as_mut_ptr().cast(),
                values.len(),
                core::mem::size_of::<i32>(),
                Some(cmp_i32),
                None,
            );
        }
        assert_eq!(values, [-8, -3, 0, 4, 4, 7, 19]);
    }

    #[test]
    fn sort_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("sort"),
            Some(linux_sort as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("sort_r"),
            Some(linux_sort_r as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("list_sort"),
            Some(linux_list_sort as usize)
        );
    }

    #[repr(C)]
    struct TestListNode {
        list: LinuxListHead,
        key: i32,
        order: i32,
    }

    unsafe extern "C" fn cmp_list_node(
        _priv_data: *mut c_void,
        a: *const LinuxListHead,
        b: *const LinuxListHead,
    ) -> i32 {
        let a = unsafe { &*(a as *const TestListNode) };
        let b = unsafe { &*(b as *const TestListNode) };
        (a.key > b.key) as i32
    }

    unsafe fn init_list_head(head: *mut LinuxListHead) {
        unsafe {
            (*head).next = head;
            (*head).prev = head;
        }
    }

    unsafe fn list_add_tail(node: *mut LinuxListHead, head: *mut LinuxListHead) {
        let tail = unsafe { (*head).prev };
        unsafe {
            (*node).next = head;
            (*node).prev = tail;
            (*tail).next = node;
            (*head).prev = node;
        }
    }

    #[test]
    fn list_sort_orders_nodes_and_preserves_equal_order() {
        unsafe {
            let mut head = LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            };
            init_list_head(core::ptr::addr_of_mut!(head));

            let mut nodes = [
                TestListNode {
                    list: LinuxListHead {
                        next: core::ptr::null_mut(),
                        prev: core::ptr::null_mut(),
                    },
                    key: 2,
                    order: 0,
                },
                TestListNode {
                    list: LinuxListHead {
                        next: core::ptr::null_mut(),
                        prev: core::ptr::null_mut(),
                    },
                    key: 1,
                    order: 1,
                },
                TestListNode {
                    list: LinuxListHead {
                        next: core::ptr::null_mut(),
                        prev: core::ptr::null_mut(),
                    },
                    key: 2,
                    order: 2,
                },
                TestListNode {
                    list: LinuxListHead {
                        next: core::ptr::null_mut(),
                        prev: core::ptr::null_mut(),
                    },
                    key: 3,
                    order: 3,
                },
            ];

            for node in &mut nodes {
                list_add_tail(
                    core::ptr::addr_of_mut!(node.list),
                    core::ptr::addr_of_mut!(head),
                );
            }

            linux_list_sort(
                core::ptr::null_mut(),
                core::ptr::addr_of_mut!(head),
                Some(cmp_list_node),
            );

            let mut seen: Vec<(i32, i32)> = Vec::new();
            let mut current = head.next;
            while current != core::ptr::addr_of_mut!(head) {
                let node = &*(current as *const TestListNode);
                seen.push((node.key, node.order));
                current = (*current).next;
            }
            assert_eq!(seen.as_slice(), &[(1, 1), (2, 0), (2, 2), (3, 3)]);
            assert_eq!((*head.next).prev, core::ptr::addr_of_mut!(head));
            assert_eq!((*head.prev).next, core::ptr::addr_of_mut!(head));
        }
    }
}
