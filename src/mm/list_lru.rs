//! linux-parity: partial
//! linux-source: vendor/linux/mm/list_lru.c
//! List-LRU core ABI used by Linux-built shrinkers.

use core::ffi::c_void;

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::list::ListHead;
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL};

const LIST_LRU_NODE_OFFSET: usize = 0;

const LIST_LRU_ONE_LIST_OFFSET: usize = 0;
const LIST_LRU_ONE_NR_ITEMS_OFFSET: usize = 16;
const LIST_LRU_NODE_NR_ITEMS_OFFSET: usize = 32;
const LIST_LRU_NODE_SIZE: usize = 64;

const LRU_REMOVED: u32 = 0;
const LRU_REMOVED_RETRY: u32 = 1;
const LRU_ROTATE: u32 = 2;
const LRU_SKIP: u32 = 3;
const LRU_RETRY: u32 = 4;
const LRU_STOP: u32 = 5;

type ListLruWalkCb = unsafe extern "C" fn(*mut ListHead, *mut c_void, *mut c_void) -> u32;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__list_lru_init", linux___list_lru_init as usize, true);
    export_symbol_once("list_lru_add", linux_list_lru_add as usize, true);
    export_symbol_once(
        "list_lru_count_node",
        linux_list_lru_count_node as usize,
        true,
    );
    export_symbol_once(
        "list_lru_walk_node",
        linux_list_lru_walk_node as usize,
        true,
    );
    export_symbol_once("list_lru_isolate", linux_list_lru_isolate as usize, true);
    export_symbol_once(
        "list_lru_isolate_move",
        linux_list_lru_isolate_move as usize,
        true,
    );
}

unsafe fn read_usize(base: *mut c_void, offset: usize) -> usize {
    unsafe { base.cast::<u8>().add(offset).cast::<usize>().read() }
}

unsafe fn write_usize(base: *mut c_void, offset: usize, value: usize) {
    unsafe { base.cast::<u8>().add(offset).cast::<usize>().write(value) };
}

unsafe fn read_isize(base: *mut c_void, offset: usize) -> isize {
    unsafe { base.cast::<u8>().add(offset).cast::<isize>().read() }
}

unsafe fn write_isize(base: *mut c_void, offset: usize, value: isize) {
    unsafe { base.cast::<u8>().add(offset).cast::<isize>().write(value) };
}

unsafe fn add_isize(base: *mut c_void, offset: usize, delta: isize) {
    let value = unsafe { read_isize(base, offset) };
    unsafe { write_isize(base, offset, value.saturating_add(delta)) };
}

unsafe fn lru_node(lru: *mut c_void) -> *mut c_void {
    if lru.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { read_usize(lru, LIST_LRU_NODE_OFFSET) as *mut c_void }
}

unsafe fn lru_one(lru: *mut c_void, _nid: i32) -> *mut c_void {
    unsafe { lru_node(lru) }
}

unsafe fn init_lru_one(one: *mut c_void) {
    if one.is_null() {
        return;
    }
    unsafe {
        ListHead::init(one.cast::<u8>().add(LIST_LRU_ONE_LIST_OFFSET).cast());
        write_isize(one, LIST_LRU_ONE_NR_ITEMS_OFFSET, 0);
    }
}

unsafe fn list_is_empty(item: *const ListHead) -> bool {
    !item.is_null() && unsafe { (*item).next == item.cast_mut() }
}

unsafe fn list_move_tail(item: *mut ListHead, head: *mut ListHead) {
    unsafe {
        ListHead::list_del(item);
        ListHead::list_add_tail(item, head);
    }
}

/// `__list_lru_init` - `vendor/linux/mm/list_lru.c:664`.
pub unsafe extern "C" fn linux___list_lru_init(
    lru: *mut c_void,
    _memcg_aware: bool,
    _shrinker: *mut c_void,
) -> i32 {
    if lru.is_null() {
        return -EINVAL;
    }
    let node = unsafe {
        crate::mm::slab::kmalloc(LIST_LRU_NODE_SIZE, GFP_KERNEL | __GFP_ZERO).cast::<c_void>()
    };
    if node.is_null() {
        return -ENOMEM;
    }
    unsafe {
        init_lru_one(node);
        write_isize(node, LIST_LRU_NODE_NR_ITEMS_OFFSET, 0);
        write_usize(lru, LIST_LRU_NODE_OFFSET, node as usize);
    }
    0
}

/// `list_lru_add` - `vendor/linux/mm/list_lru.c:223`.
pub unsafe extern "C" fn linux_list_lru_add(
    lru: *mut c_void,
    item: *mut ListHead,
    nid: i32,
    _memcg: *mut c_void,
) -> bool {
    if lru.is_null() || item.is_null() {
        return false;
    }
    let one = unsafe { lru_one(lru, nid) };
    if one.is_null() || !unsafe { list_is_empty(item) } {
        return false;
    }
    unsafe {
        let head = one.cast::<u8>().add(LIST_LRU_ONE_LIST_OFFSET).cast();
        ListHead::list_add_tail(item, head);
        add_isize(one, LIST_LRU_ONE_NR_ITEMS_OFFSET, 1);
        add_isize(one, LIST_LRU_NODE_NR_ITEMS_OFFSET, 1);
    }
    true
}

/// `list_lru_count_node` - `vendor/linux/mm/list_lru.c:327`.
pub unsafe extern "C" fn linux_list_lru_count_node(lru: *mut c_void, nid: i32) -> usize {
    let one = unsafe { lru_one(lru, nid) };
    if one.is_null() {
        return 0;
    }
    unsafe { read_isize(one, LIST_LRU_NODE_NR_ITEMS_OFFSET).max(0) as usize }
}

/// `list_lru_isolate` - `vendor/linux/mm/list_lru.c:294`.
pub unsafe extern "C" fn linux_list_lru_isolate(list: *mut c_void, item: *mut ListHead) {
    if list.is_null() || item.is_null() {
        return;
    }
    unsafe {
        ListHead::list_del(item);
        add_isize(list, LIST_LRU_ONE_NR_ITEMS_OFFSET, -1);
    }
}

/// `list_lru_isolate_move` - `vendor/linux/mm/list_lru.c:301`.
pub unsafe extern "C" fn linux_list_lru_isolate_move(
    list: *mut c_void,
    item: *mut ListHead,
    head: *mut ListHead,
) {
    if list.is_null() || item.is_null() || head.is_null() {
        return;
    }
    unsafe {
        list_move_tail(item, head);
        add_isize(list, LIST_LRU_ONE_NR_ITEMS_OFFSET, -1);
    }
}

/// `list_lru_walk_node` - `vendor/linux/mm/list_lru.c:413`.
pub unsafe extern "C" fn linux_list_lru_walk_node(
    lru: *mut c_void,
    nid: i32,
    isolate: Option<ListLruWalkCb>,
    cb_arg: *mut c_void,
    nr_to_walk: *mut usize,
) -> usize {
    let Some(isolate) = isolate else {
        return 0;
    };
    let one = unsafe { lru_one(lru, nid) };
    if one.is_null() || nr_to_walk.is_null() {
        return 0;
    }
    let head = unsafe {
        one.cast::<u8>()
            .add(LIST_LRU_ONE_LIST_OFFSET)
            .cast::<ListHead>()
    };
    let mut isolated = 0usize;
    loop {
        if unsafe { *nr_to_walk == 0 || ListHead::is_empty(head) } {
            break;
        }
        unsafe { *nr_to_walk = (*nr_to_walk).saturating_sub(1) };
        let item = unsafe { (*head).next };
        let ret = unsafe { isolate(item, one, cb_arg) };
        match ret {
            LRU_REMOVED | LRU_REMOVED_RETRY => {
                isolated = isolated.saturating_add(1);
                unsafe { add_isize(one, LIST_LRU_NODE_NR_ITEMS_OFFSET, -1) };
            }
            LRU_ROTATE => unsafe { list_move_tail(item, head) },
            LRU_SKIP => {}
            LRU_RETRY => {}
            LRU_STOP => break,
            _ => break,
        }
    }
    isolated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_list_lru_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("list_lru_count_node"),
            Some(linux_list_lru_count_node as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("list_lru_walk_node"),
            Some(linux_list_lru_walk_node as usize)
        );
    }
}
