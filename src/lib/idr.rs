//! linux-parity: partial
//! linux-source: vendor/linux/lib/idr.c
//! test-origin: linux:vendor/linux/lib/idr.c
//! IDA exports used by Linux-built modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOENT, ENOMEM, ENOSPC};
use crate::kernel::module::{export_symbol, find_symbol};

struct IdaState {
    ida: usize,
    allocated: Vec<bool>,
}

struct IdrEntry {
    id: u32,
    ptr: usize,
}

type IdrIterFn = unsafe extern "C" fn(i32, *mut c_void, *mut c_void) -> i32;

struct IdrState {
    idr: usize,
    entries: Vec<IdrEntry>,
}

struct XArrayEntry {
    index: usize,
    ptr: usize,
}

struct XArrayState {
    xa: usize,
    entries: Vec<XArrayEntry>,
}

static IDA_STATES: Mutex<Vec<IdaState>> = Mutex::new(Vec::new());
static IDR_STATES: Mutex<Vec<IdrState>> = Mutex::new(Vec::new());
static XARRAY_STATES: Mutex<Vec<XArrayState>> = Mutex::new(Vec::new());

const XA_MAX_MARKS: u32 = 3;
const LINUX_XARRAY_XA_FLAGS_OFFSET: usize = 4;
const GFP_BITS_SHIFT: u32 = 25;
const MAX_XA_STORE_RANGE_SLOTS: usize = 1 << 16;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ida_alloc_range", linux_ida_alloc_range as usize, true);
    export_symbol_once("ida_free", linux_ida_free as usize, true);
    export_symbol_once("ida_destroy", linux_ida_destroy as usize, false);
    export_symbol_once("idr_alloc", linux_idr_alloc as usize, true);
    export_symbol_once("idr_alloc_u32", linux_idr_alloc_u32 as usize, true);
    export_symbol_once("idr_find", linux_idr_find as usize, true);
    export_symbol_once("idr_remove", linux_idr_remove as usize, true);
    export_symbol_once("idr_preload", linux_idr_preload as usize, false);
    export_symbol_once("idr_for_each", linux_idr_for_each as usize, false);
    export_symbol_once("idr_get_next", linux_idr_get_next as usize, false);
    export_symbol_once("idr_get_next_ul", linux_idr_get_next_ul as usize, false);
    export_symbol_once("idr_replace", linux_idr_replace as usize, false);
    export_symbol_once("idr_destroy", linux_idr_destroy as usize, false);
    export_symbol_once("radix_tree_tagged", linux_radix_tree_tagged as usize, false);
    export_symbol_once("radix_tree_insert", linux_radix_tree_insert as usize, false);
    export_symbol_once("radix_tree_lookup", linux_radix_tree_lookup as usize, false);
    export_symbol_once("radix_tree_delete", linux_radix_tree_delete as usize, false);
    export_symbol_once(
        "radix_tree_delete_item",
        linux_radix_tree_delete_item as usize,
        false,
    );
    export_symbol_once(
        "radix_tree_iter_delete",
        linux_radix_tree_iter_delete as usize,
        false,
    );
    export_symbol_once(
        "radix_tree_next_chunk",
        linux_radix_tree_next_chunk as usize,
        false,
    );
    export_symbol_once("xa_store", linux_xa_store as usize, false);
    export_symbol_once("xa_store_range", linux_xa_store_range as usize, false);
    export_symbol_once("xa_load", linux_xa_load as usize, false);
    export_symbol_once("xa_find", linux_xa_find as usize, false);
    export_symbol_once("xa_find_after", linux_xa_find_after as usize, false);
    export_symbol_once("xa_erase", linux_xa_erase as usize, false);
    export_symbol_once("__xa_store", linux___xa_store as usize, false);
    export_symbol_once("__xa_erase", linux___xa_erase as usize, false);
    export_symbol_once("__xa_insert", linux___xa_insert as usize, false);
    export_symbol_once("__xa_alloc", linux___xa_alloc as usize, false);
    export_symbol_once("__xa_alloc_cyclic", linux___xa_alloc_cyclic as usize, false);
    export_symbol_once("xa_destroy", linux_xa_destroy as usize, false);
}

/// `ida_alloc_range` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_ida_alloc_range(
    ida: *mut c_void,
    min: u32,
    max: u32,
    _gfp: u32,
) -> i32 {
    if min > max {
        return -EINVAL;
    }
    let max = max.min(i32::MAX as u32);
    if min > max {
        return -ENOSPC;
    }

    let key = ida as usize;
    let mut states = IDA_STATES.lock();
    let state_index = match states.iter().position(|state| state.ida == key) {
        Some(index) => index,
        None => {
            if states.try_reserve_exact(1).is_err() {
                return -ENOMEM;
            }
            states.push(IdaState {
                ida: key,
                allocated: Vec::new(),
            });
            states.len() - 1
        }
    };
    let allocated = &mut states[state_index].allocated;
    let min = min as usize;
    let max = max as usize;

    let mut id = None;
    if min < allocated.len() {
        let end = max.min(allocated.len().saturating_sub(1));
        for candidate in min..=end {
            if !allocated[candidate] {
                id = Some(candidate);
                break;
            }
        }
    }

    let id = match id {
        Some(id) => id,
        None => {
            let next = min.max(allocated.len());
            if next > max {
                return -ENOSPC;
            }
            let target_len = next + 1;
            let additional = target_len.saturating_sub(allocated.len());
            if allocated.try_reserve_exact(additional).is_err() {
                return -ENOMEM;
            }
            allocated.resize(target_len, false);
            next
        }
    };
    allocated[id] = true;
    id as i32
}

/// `ida_free` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_ida_free(ida: *mut c_void, id: u32) {
    let key = ida as usize;
    let mut states = IDA_STATES.lock();
    if let Some(state) = states.iter_mut().find(|state| state.ida == key) {
        let index = id as usize;
        if index < state.allocated.len() {
            state.allocated[index] = false;
        }
    }
}

/// `ida_destroy` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_ida_destroy(ida: *mut c_void) {
    let key = ida as usize;
    let mut states = IDA_STATES.lock();
    if let Some(index) = states.iter().position(|state| state.ida == key) {
        states.swap_remove(index);
    }
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as usize as *mut c_void
}

fn is_err_ptr(ptr: *mut c_void) -> bool {
    (ptr as usize) >= usize::MAX - 4095
}

unsafe fn read_u32(base: *const c_void, offset: usize) -> u32 {
    unsafe { base.cast::<u8>().add(offset).cast::<u32>().read_unaligned() }
}

fn find_or_create_idr_state(states: &mut Vec<IdrState>, idr: usize) -> Result<usize, i32> {
    if let Some(index) = states.iter().position(|state| state.idr == idr) {
        return Ok(index);
    }
    if states.try_reserve_exact(1).is_err() {
        return Err(-ENOMEM);
    }
    states.push(IdrState {
        idr,
        entries: Vec::new(),
    });
    Ok(states.len() - 1)
}

/// `idr_preload` - `vendor/linux/lib/radix-tree.c`.
///
/// Lupos backs this compatibility IDR with heap-owned vectors, so there is no
/// separate radix-tree node cache to preload before a locked `idr_alloc()`.
pub unsafe extern "C" fn linux_idr_preload(_gfp: u32) {}

/// `idr_alloc_u32` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_alloc_u32(
    idr: *mut c_void,
    ptr: *mut c_void,
    nextid: *mut u32,
    max: usize,
    _gfp: u32,
) -> i32 {
    if nextid.is_null() {
        return -EINVAL;
    }
    let start = unsafe { *nextid };
    let max = max.min(u32::MAX as usize) as u32;
    if start > max {
        return -ENOSPC;
    }

    let mut states = IDR_STATES.lock();
    let state_index = match find_or_create_idr_state(&mut states, idr as usize) {
        Ok(index) => index,
        Err(err) => return err,
    };
    let entries = &mut states[state_index].entries;
    let mut candidate = start;
    loop {
        if entries.iter().all(|entry| entry.id != candidate) {
            if entries.try_reserve_exact(1).is_err() {
                return -ENOMEM;
            }
            unsafe {
                *nextid = candidate;
            }
            entries.push(IdrEntry {
                id: candidate,
                ptr: ptr as usize,
            });
            return 0;
        }
        if candidate == max {
            break;
        }
        candidate += 1;
    }
    -ENOSPC
}

/// `idr_alloc` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_alloc(
    idr: *mut c_void,
    ptr: *mut c_void,
    start: i32,
    end: i32,
    _gfp: u32,
) -> i32 {
    if start < 0 {
        return -EINVAL;
    }
    let min = start as u32;
    let max = if end > 0 {
        (end as u32).saturating_sub(1)
    } else {
        i32::MAX as u32
    };
    if min > max {
        return -ENOSPC;
    }

    let mut states = IDR_STATES.lock();
    let state_index = match find_or_create_idr_state(&mut states, idr as usize) {
        Ok(index) => index,
        Err(err) => return err,
    };
    let entries = &mut states[state_index].entries;
    let mut candidate = min;
    while candidate <= max {
        if entries.iter().all(|entry| entry.id != candidate) {
            if entries.try_reserve_exact(1).is_err() {
                return -ENOMEM;
            }
            entries.push(IdrEntry {
                id: candidate,
                ptr: ptr as usize,
            });
            return candidate as i32;
        }
        if candidate == u32::MAX {
            break;
        }
        candidate += 1;
    }
    -ENOSPC
}

/// `idr_find` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_find(idr: *const c_void, id: usize) -> *mut c_void {
    let states = IDR_STATES.lock();
    states
        .iter()
        .find(|state| state.idr == idr as usize)
        .and_then(|state| state.entries.iter().find(|entry| entry.id as usize == id))
        .map(|entry| entry.ptr as *mut c_void)
        .unwrap_or(core::ptr::null_mut())
}

/// `idr_remove` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_remove(idr: *mut c_void, id: usize) -> *mut c_void {
    let mut states = IDR_STATES.lock();
    let Some(state) = states.iter_mut().find(|state| state.idr == idr as usize) else {
        return core::ptr::null_mut();
    };
    let Some(index) = state
        .entries
        .iter()
        .position(|entry| entry.id as usize == id)
    else {
        return core::ptr::null_mut();
    };
    state.entries.swap_remove(index).ptr as *mut c_void
}

fn sorted_idr_entries(state: &IdrState) -> Vec<(u32, usize)> {
    let mut entries: Vec<(u32, usize)> = state
        .entries
        .iter()
        .map(|entry| (entry.id, entry.ptr))
        .collect();
    entries.sort_unstable_by_key(|entry| entry.0);
    entries
}

/// `idr_for_each` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_for_each(
    idr: *const c_void,
    func: Option<IdrIterFn>,
    data: *mut c_void,
) -> i32 {
    let Some(func) = func else { return 0 };
    let entries = {
        let states = IDR_STATES.lock();
        states
            .iter()
            .find(|state| state.idr == idr as usize)
            .map(sorted_idr_entries)
            .unwrap_or_default()
    };
    for (id, ptr) in entries {
        if id > i32::MAX as u32 {
            break;
        }
        let ret = unsafe { func(id as i32, ptr as *mut c_void, data) };
        if ret != 0 {
            return ret;
        }
    }
    0
}

/// `idr_get_next_ul` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_get_next_ul(
    idr: *mut c_void,
    nextid: *mut usize,
) -> *mut c_void {
    if nextid.is_null() {
        return core::ptr::null_mut();
    }
    let start = unsafe { *nextid };
    let states = IDR_STATES.lock();
    let Some(state) = states.iter().find(|state| state.idr == idr as usize) else {
        return core::ptr::null_mut();
    };
    let Some(entry) = state
        .entries
        .iter()
        .filter(|entry| entry.id as usize >= start)
        .min_by_key(|entry| entry.id)
    else {
        return core::ptr::null_mut();
    };
    unsafe {
        *nextid = entry.id as usize;
    }
    entry.ptr as *mut c_void
}

/// `idr_get_next` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_get_next(idr: *mut c_void, nextid: *mut i32) -> *mut c_void {
    if nextid.is_null() {
        return core::ptr::null_mut();
    }
    let start = unsafe { *nextid };
    if start < 0 {
        return core::ptr::null_mut();
    }
    let mut id = start as usize;
    let entry = unsafe { linux_idr_get_next_ul(idr, core::ptr::addr_of_mut!(id)) };
    if entry.is_null() || id > i32::MAX as usize {
        return core::ptr::null_mut();
    }
    unsafe {
        *nextid = id as i32;
    }
    entry
}

/// `idr_replace` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_replace(
    idr: *mut c_void,
    ptr: *mut c_void,
    id: usize,
) -> *mut c_void {
    let mut states = IDR_STATES.lock();
    let Some(state) = states.iter_mut().find(|state| state.idr == idr as usize) else {
        return err_ptr(ENOENT);
    };
    let Some(entry) = state
        .entries
        .iter_mut()
        .find(|entry| entry.id as usize == id)
    else {
        return err_ptr(ENOENT);
    };
    let old = entry.ptr;
    entry.ptr = ptr as usize;
    old as *mut c_void
}

/// `idr_destroy` - `vendor/linux/lib/idr.c`.
pub unsafe extern "C" fn linux_idr_destroy(idr: *mut c_void) {
    let mut states = IDR_STATES.lock();
    if let Some(index) = states.iter().position(|state| state.idr == idr as usize) {
        states.swap_remove(index);
    }
}

/// `radix_tree_tagged` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_tagged(root: *const c_void, tag: u32) -> i32 {
    if root.is_null() || tag >= XA_MAX_MARKS {
        return 0;
    }
    let flags = unsafe { read_u32(root, LINUX_XARRAY_XA_FLAGS_OFFSET) };
    let mask = 1u32 << (GFP_BITS_SHIFT + tag);
    (flags & mask) as i32
}

/// `radix_tree_insert` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_insert(
    root: *mut c_void,
    index: usize,
    item: *mut c_void,
) -> i32 {
    unsafe { linux___xa_insert(root, index, item, 0) }
}

/// `radix_tree_lookup` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_lookup(root: *const c_void, index: usize) -> *mut c_void {
    unsafe { linux_xa_load(root, index) }
}

/// `radix_tree_delete_item` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_delete_item(
    root: *mut c_void,
    index: usize,
    item: *mut c_void,
) -> *mut c_void {
    let entry = unsafe { linux_xa_load(root, index) };
    if entry.is_null() || (!item.is_null() && entry != item) {
        return core::ptr::null_mut();
    }
    unsafe { linux_xa_erase(root, index) }
}

/// `radix_tree_delete` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_delete(root: *mut c_void, index: usize) -> *mut c_void {
    unsafe { linux_radix_tree_delete_item(root, index, core::ptr::null_mut()) }
}

/// `radix_tree_iter_delete` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_iter_delete(
    _root: *mut c_void,
    _iter: *mut c_void,
    slot: *mut *mut c_void,
) {
    if !slot.is_null() {
        unsafe { slot.write(core::ptr::null_mut()) };
    }
}

/// `radix_tree_next_chunk` - `vendor/linux/lib/radix-tree.c`.
pub unsafe extern "C" fn linux_radix_tree_next_chunk(
    _root: *const c_void,
    _iter: *mut c_void,
    _flags: u32,
) -> *mut *mut c_void {
    core::ptr::null_mut()
}

fn find_or_create_xarray_state(states: &mut Vec<XArrayState>, xa: usize) -> Result<usize, i32> {
    if let Some(index) = states.iter().position(|state| state.xa == xa) {
        return Ok(index);
    }
    if states.try_reserve_exact(1).is_err() {
        return Err(-ENOMEM);
    }
    states.push(XArrayState {
        xa,
        entries: Vec::new(),
    });
    Ok(states.len() - 1)
}

fn xarray_store_occupied(
    entries: &mut Vec<XArrayEntry>,
    index: usize,
    ptr: usize,
) -> Result<Option<usize>, i32> {
    if let Some(slot) = entries.iter_mut().find(|slot| slot.index == index) {
        let old = slot.ptr;
        slot.ptr = ptr;
        return Ok(Some(old));
    }
    if entries.try_reserve_exact(1).is_err() {
        return Err(-ENOMEM);
    }
    entries.push(XArrayEntry { index, ptr });
    Ok(None)
}

fn xarray_find_present(
    entries: &[XArrayEntry],
    start: usize,
    max: usize,
) -> Option<(usize, usize)> {
    if start > max {
        return None;
    }
    entries
        .iter()
        .filter(|entry| entry.ptr != 0 && entry.index >= start && entry.index <= max)
        .min_by_key(|entry| entry.index)
        .map(|entry| (entry.index, entry.ptr))
}

/// `xa_store` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_store(
    xa: *mut c_void,
    index: usize,
    entry: *mut c_void,
    _gfp: u32,
) -> *mut c_void {
    let mut states = XARRAY_STATES.lock();
    let state_index = match find_or_create_xarray_state(&mut states, xa as usize) {
        Ok(index) => index,
        Err(err) => return err_ptr(-err),
    };
    let entries = &mut states[state_index].entries;

    if entry.is_null() {
        return entries
            .iter()
            .position(|slot| slot.index == index)
            .map(|slot| entries.swap_remove(slot).ptr as *mut c_void)
            .unwrap_or(core::ptr::null_mut());
    }

    match xarray_store_occupied(entries, index, entry as usize) {
        Ok(Some(old)) => old as *mut c_void,
        Ok(None) => core::ptr::null_mut(),
        Err(err) => err_ptr(-err),
    }
}

/// `xa_store_range` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_store_range(
    xa: *mut c_void,
    first: usize,
    last: usize,
    entry: *mut c_void,
    gfp: u32,
) -> *mut c_void {
    if first > last {
        return err_ptr(EINVAL);
    }
    let Some(count) = last.checked_sub(first).and_then(|span| span.checked_add(1)) else {
        return err_ptr(EINVAL);
    };
    if count > MAX_XA_STORE_RANGE_SLOTS {
        return err_ptr(ENOMEM);
    }
    let mut index = first;
    while index <= last {
        let previous = unsafe { linux_xa_store(xa, index, entry, gfp) };
        if is_err_ptr(previous) {
            return previous;
        }
        if index == usize::MAX {
            break;
        }
        index += 1;
    }
    core::ptr::null_mut()
}

/// `xa_load` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_load(xa: *const c_void, index: usize) -> *mut c_void {
    let states = XARRAY_STATES.lock();
    states
        .iter()
        .find(|state| state.xa == xa as usize)
        .and_then(|state| state.entries.iter().find(|slot| slot.index == index))
        .map(|slot| slot.ptr as *mut c_void)
        .unwrap_or(core::ptr::null_mut())
}

/// `xa_find` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_find(
    xa: *mut c_void,
    indexp: *mut usize,
    max: usize,
    filter: u32,
) -> *mut c_void {
    if indexp.is_null() || filter < XA_MAX_MARKS {
        return core::ptr::null_mut();
    }

    let start = unsafe { *indexp };
    let states = XARRAY_STATES.lock();
    let Some((index, ptr)) = states
        .iter()
        .find(|state| state.xa == xa as usize)
        .and_then(|state| xarray_find_present(&state.entries, start, max))
    else {
        return core::ptr::null_mut();
    };

    unsafe { *indexp = index };
    ptr as *mut c_void
}

/// `xa_find_after` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_find_after(
    xa: *mut c_void,
    indexp: *mut usize,
    max: usize,
    filter: u32,
) -> *mut c_void {
    if indexp.is_null() || filter < XA_MAX_MARKS {
        return core::ptr::null_mut();
    }
    let Some(start) = unsafe { *indexp }.checked_add(1) else {
        return core::ptr::null_mut();
    };

    let states = XARRAY_STATES.lock();
    let Some((index, ptr)) = states
        .iter()
        .find(|state| state.xa == xa as usize)
        .and_then(|state| xarray_find_present(&state.entries, start, max))
    else {
        return core::ptr::null_mut();
    };

    unsafe { *indexp = index };
    ptr as *mut c_void
}

/// `xa_erase` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_erase(xa: *mut c_void, index: usize) -> *mut c_void {
    unsafe { linux_xa_store(xa, index, core::ptr::null_mut(), 0) }
}

/// `__xa_store` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux___xa_store(
    xa: *mut c_void,
    index: usize,
    entry: *mut c_void,
    gfp: u32,
) -> *mut c_void {
    unsafe { linux_xa_store(xa, index, entry, gfp) }
}

/// `__xa_erase` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux___xa_erase(xa: *mut c_void, index: usize) -> *mut c_void {
    unsafe { linux_xa_erase(xa, index) }
}

/// `__xa_insert` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux___xa_insert(
    xa: *mut c_void,
    index: usize,
    entry: *mut c_void,
    gfp: u32,
) -> i32 {
    let mut states = XARRAY_STATES.lock();
    let state_index = match find_or_create_xarray_state(&mut states, xa as usize) {
        Ok(index) => index,
        Err(err) => return err,
    };
    let entries = &mut states[state_index].entries;
    if entries.iter().any(|slot| slot.index == index) {
        return -EBUSY;
    }
    if xarray_store_occupied(entries, index, entry as usize).is_err() {
        return -ENOMEM;
    }
    let _ = gfp;
    0
}

/// `__xa_alloc` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux___xa_alloc(
    xa: *mut c_void,
    id: *mut u32,
    entry: *mut c_void,
    limit: u64,
    _gfp: u32,
) -> i32 {
    let max = (limit & 0xffff_ffff) as usize;
    let min = (limit >> 32) as usize;
    if min > max {
        return -EBUSY;
    }

    let mut states = XARRAY_STATES.lock();
    let state_index = match find_or_create_xarray_state(&mut states, xa as usize) {
        Ok(index) => index,
        Err(err) => return err,
    };
    let entries = &mut states[state_index].entries;
    let mut index = min;
    while index <= max {
        if entries.iter().all(|slot| slot.index != index) {
            if xarray_store_occupied(entries, index, entry as usize).is_err() {
                return -ENOMEM;
            }
            if !id.is_null() {
                unsafe { *id = index as u32 };
            }
            return 0;
        }
        if index == usize::MAX {
            break;
        }
        index += 1;
    }
    -EBUSY
}

/// `__xa_alloc_cyclic` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux___xa_alloc_cyclic(
    xa: *mut c_void,
    id: *mut u32,
    entry: *mut c_void,
    limit: u64,
    next: *mut u32,
    gfp: u32,
) -> i32 {
    if next.is_null() {
        return -EINVAL;
    }
    let max = (limit & 0xffff_ffff) as u32;
    let min = (limit >> 32) as u32;
    if min > max {
        return -EBUSY;
    }

    let start = unsafe { *next }.max(min).min(max);
    let first_limit = ((start as u64) << 32) | max as u64;
    let mut ret = unsafe { linux___xa_alloc(xa, id, entry, first_limit, gfp) };
    if ret < 0 && start > min {
        let wrapped_limit = ((min as u64) << 32) | (start.saturating_sub(1) as u64);
        ret = unsafe { linux___xa_alloc(xa, id, entry, wrapped_limit, gfp) };
        if ret == 0 {
            ret = 1;
        }
    }
    if ret >= 0 && !id.is_null() {
        unsafe {
            *next = (*id).wrapping_add(1);
        }
    }
    ret
}

/// `xa_destroy` - `vendor/linux/lib/xarray.c`.
pub unsafe extern "C" fn linux_xa_destroy(xa: *mut c_void) {
    let mut states = XARRAY_STATES.lock();
    if let Some(index) = states.iter().position(|state| state.xa == xa as usize) {
        states.swap_remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("ida_alloc_range"),
            Some(linux_ida_alloc_range as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("ida_free"),
            Some(linux_ida_free as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("ida_destroy"),
            Some(linux_ida_destroy as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("idr_find"),
            Some(linux_idr_find as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("idr_alloc_u32"),
            Some(linux_idr_alloc_u32 as usize)
        );
        for name in [
            "radix_tree_insert",
            "radix_tree_lookup",
            "radix_tree_delete",
            "radix_tree_delete_item",
            "radix_tree_iter_delete",
            "radix_tree_next_chunk",
            "__xa_store",
            "__xa_erase",
            "__xa_alloc_cyclic",
        ] {
            assert!(
                crate::kernel::module::find_symbol(name).is_some(),
                "missing export {name}"
            );
        }
    }

    #[test]
    fn idr_alloc_u32_export_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/idr.c"
        ));
        assert!(source.contains("int idr_alloc_u32(struct idr *idr"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(idr_alloc_u32);"));
    }

    #[test]
    fn ida_alloc_range_respects_bounds() {
        unsafe {
            let id = linux_ida_alloc_range(core::ptr::null_mut(), 10, 12, 0);
            assert!((10..=12).contains(&id));
            linux_ida_free(core::ptr::null_mut(), id as u32);
            assert_eq!(
                linux_ida_alloc_range(core::ptr::null_mut(), 12, 10, 0),
                -EINVAL
            );
        }
    }

    #[test]
    fn ida_alloc_range_is_scoped_per_ida_pointer() {
        unsafe {
            let mut first = 0usize;
            let mut second = 0usize;
            let first_ptr = core::ptr::addr_of_mut!(first).cast::<c_void>();
            let second_ptr = core::ptr::addr_of_mut!(second).cast::<c_void>();

            assert_eq!(linux_ida_alloc_range(first_ptr, 0, 1024, 0), 0);
            assert_eq!(linux_ida_alloc_range(first_ptr, 0, 1024, 0), 1);
            assert_eq!(linux_ida_alloc_range(second_ptr, 0, 1024, 0), 0);

            linux_ida_free(first_ptr, 0);
            assert_eq!(linux_ida_alloc_range(first_ptr, 0, 1024, 0), 0);
        }
    }

    #[test]
    fn idr_alloc_u32_uses_inclusive_max_and_preserves_nextid_on_error() {
        unsafe {
            let mut idr = 0usize;
            let idr_ptr = core::ptr::addr_of_mut!(idr).cast::<c_void>();

            let mut next = 5u32;
            assert_eq!(
                linux_idr_alloc_u32(
                    idr_ptr,
                    0x50usize as *mut c_void,
                    core::ptr::addr_of_mut!(next),
                    6,
                    0,
                ),
                0
            );
            assert_eq!(next, 5);
            assert_eq!(linux_idr_find(idr_ptr, 5) as usize, 0x50);

            next = 5;
            assert_eq!(
                linux_idr_alloc_u32(
                    idr_ptr,
                    0x60usize as *mut c_void,
                    core::ptr::addr_of_mut!(next),
                    6,
                    0,
                ),
                0
            );
            assert_eq!(next, 6);
            assert_eq!(linux_idr_find(idr_ptr, 6) as usize, 0x60);

            next = 5;
            assert_eq!(
                linux_idr_alloc_u32(
                    idr_ptr,
                    0x70usize as *mut c_void,
                    core::ptr::addr_of_mut!(next),
                    6,
                    0,
                ),
                -ENOSPC
            );
            assert_eq!(next, 5);

            next = 10;
            assert_eq!(
                linux_idr_alloc_u32(
                    idr_ptr,
                    0x80usize as *mut c_void,
                    core::ptr::addr_of_mut!(next),
                    9,
                    0,
                ),
                -ENOSPC
            );
            assert_eq!(next, 10);
            linux_idr_destroy(idr_ptr);
        }
    }

    #[test]
    fn idr_alloc_find_replace_remove_round_trip() {
        unsafe {
            let mut idr = 0usize;
            let idr_ptr = core::ptr::addr_of_mut!(idr).cast::<c_void>();
            let first = 0x1111usize as *mut c_void;
            let second = 0x2222usize as *mut c_void;

            let id = linux_idr_alloc(idr_ptr, first, 3, 8, 0);
            assert_eq!(id, 3);
            assert_eq!(linux_idr_find(idr_ptr, id as usize), first);
            assert_eq!(linux_idr_replace(idr_ptr, second, id as usize), first);
            assert_eq!(linux_idr_find(idr_ptr, id as usize), second);
            assert_eq!(linux_idr_remove(idr_ptr, id as usize), second);
            assert!(linux_idr_find(idr_ptr, id as usize).is_null());
            linux_idr_destroy(idr_ptr);
        }
    }

    unsafe extern "C" fn collect_until_second(id: i32, p: *mut c_void, data: *mut c_void) -> i32 {
        let seen = unsafe { &mut *(data as *mut Vec<(i32, usize)>) };
        seen.push((id, p as usize));
        if seen.len() == 2 { 7 } else { 0 }
    }

    #[test]
    fn idr_iteration_follows_next_populated_id_order() {
        unsafe {
            let mut idr = 0usize;
            let idr_ptr = core::ptr::addr_of_mut!(idr).cast::<c_void>();
            assert_eq!(
                linux_idr_alloc(idr_ptr, 0x10usize as *mut c_void, 10, 11, 0),
                10
            );
            assert_eq!(
                linux_idr_alloc(idr_ptr, 0x03usize as *mut c_void, 3, 4, 0),
                3
            );

            let mut next = 4;
            assert_eq!(
                linux_idr_get_next(idr_ptr, core::ptr::addr_of_mut!(next)) as usize,
                0x10
            );
            assert_eq!(next, 10);

            let mut next_ul = 0usize;
            assert_eq!(
                linux_idr_get_next_ul(idr_ptr, core::ptr::addr_of_mut!(next_ul)) as usize,
                0x03
            );
            assert_eq!(next_ul, 3);

            let mut seen: Vec<(i32, usize)> = Vec::new();
            let ret = linux_idr_for_each(
                idr_ptr,
                Some(collect_until_second),
                core::ptr::addr_of_mut!(seen).cast::<c_void>(),
            );
            assert_eq!(ret, 7);
            assert_eq!(seen.as_slice(), &[(3, 0x03), (10, 0x10)]);
            linux_idr_destroy(idr_ptr);
        }
    }

    #[test]
    fn radix_tree_tagged_reads_xarray_root_tag_bits() {
        let mut xarray = [0u8; 16];
        let flags = 1u32 << GFP_BITS_SHIFT;
        xarray[LINUX_XARRAY_XA_FLAGS_OFFSET..LINUX_XARRAY_XA_FLAGS_OFFSET + 4]
            .copy_from_slice(&flags.to_ne_bytes());

        unsafe {
            let root = xarray.as_ptr().cast::<c_void>();
            assert_eq!(linux_radix_tree_tagged(root, 0), flags as i32);
            assert_eq!(linux_radix_tree_tagged(root, 1), 0);
            assert_eq!(linux_radix_tree_tagged(root, XA_MAX_MARKS), 0);
        }
    }

    #[test]
    fn radix_tree_basic_wrappers_share_xarray_storage() {
        unsafe {
            let mut root = 0usize;
            let root_ptr = core::ptr::addr_of_mut!(root).cast::<c_void>();
            let first = 0x1234usize as *mut c_void;
            let second = 0x5678usize as *mut c_void;

            assert_eq!(linux_radix_tree_insert(root_ptr, 4, first), 0);
            assert_eq!(linux_radix_tree_lookup(root_ptr, 4), first);
            assert_eq!(linux_radix_tree_insert(root_ptr, 4, second), -EBUSY);
            assert!(linux_radix_tree_delete_item(root_ptr, 4, second).is_null());
            assert_eq!(linux_radix_tree_delete(root_ptr, 4), first);
            assert!(linux_radix_tree_lookup(root_ptr, 4).is_null());

            linux_xa_destroy(root_ptr);
        }
    }

    #[test]
    fn xa_alloc_uses_limit_and_reserves_null_entries() {
        unsafe {
            let mut xa = 0usize;
            let xa_ptr = core::ptr::addr_of_mut!(xa).cast::<c_void>();
            let limit = (1u64 << 32) | 2;
            let mut id = 0u32;

            assert_eq!(
                linux___xa_alloc(
                    xa_ptr,
                    core::ptr::addr_of_mut!(id),
                    core::ptr::null_mut(),
                    limit,
                    0
                ),
                0
            );
            assert_eq!(id, 1);
            assert!(linux_xa_load(xa_ptr, 1).is_null());
            assert_eq!(
                linux___xa_insert(xa_ptr, 1, 0xbeefusize as *mut c_void, 0),
                -EBUSY
            );

            assert_eq!(
                linux___xa_alloc(
                    xa_ptr,
                    core::ptr::addr_of_mut!(id),
                    0xcafeusize as *mut c_void,
                    limit,
                    0,
                ),
                0
            );
            assert_eq!(id, 2);
            assert_eq!(linux_xa_load(xa_ptr, 2) as usize, 0xcafe);

            assert!(linux_xa_erase(xa_ptr, 1).is_null());
            assert_eq!(
                linux___xa_insert(xa_ptr, 1, 0xbeefusize as *mut c_void, 0),
                0
            );
            assert_eq!(linux_xa_load(xa_ptr, 1) as usize, 0xbeef);
            linux_xa_destroy(xa_ptr);
        }
    }

    #[test]
    fn xa_unlocked_and_cyclic_exports_use_side_table() {
        unsafe {
            let mut xa = 0usize;
            let xa_ptr = core::ptr::addr_of_mut!(xa).cast::<c_void>();
            let mut id = 0u32;
            let mut next = 2u32;
            let limit = (1u64 << 32) | 3;

            assert!(linux___xa_store(xa_ptr, 2, 0x20usize as *mut c_void, 0).is_null());
            assert_eq!(linux___xa_erase(xa_ptr, 2) as usize, 0x20);
            assert_eq!(
                linux___xa_alloc_cyclic(
                    xa_ptr,
                    core::ptr::addr_of_mut!(id),
                    0x30usize as *mut c_void,
                    limit,
                    core::ptr::addr_of_mut!(next),
                    0,
                ),
                0
            );
            assert_eq!(id, 2);
            assert_eq!(next, 3);
            assert_eq!(
                linux___xa_alloc_cyclic(
                    xa_ptr,
                    core::ptr::addr_of_mut!(id),
                    0x10usize as *mut c_void,
                    limit,
                    core::ptr::addr_of_mut!(next),
                    0,
                ),
                0
            );
            assert_eq!(id, 3);
            assert_eq!(next, 4);
            assert_eq!(
                linux___xa_alloc_cyclic(
                    xa_ptr,
                    core::ptr::addr_of_mut!(id),
                    0x11usize as *mut c_void,
                    limit,
                    core::ptr::addr_of_mut!(next),
                    0,
                ),
                1
            );
            assert_eq!(id, 1);
            assert_eq!(next, 2);

            linux_xa_destroy(xa_ptr);
        }
    }

    #[test]
    fn xa_find_walks_present_entries_in_index_order() {
        unsafe {
            let mut xa = 0usize;
            let xa_ptr = core::ptr::addr_of_mut!(xa).cast::<c_void>();

            assert!(linux_xa_store(xa_ptr, 8, 0x80usize as *mut c_void, 0).is_null());
            assert!(linux_xa_store(xa_ptr, 2, 0x20usize as *mut c_void, 0).is_null());
            assert!(linux_xa_store(xa_ptr, 5, core::ptr::null_mut(), 0).is_null());
            assert_eq!(linux___xa_insert(xa_ptr, 5, core::ptr::null_mut(), 0), 0);

            let mut index = 0usize;
            assert_eq!(
                linux_xa_find(xa_ptr, core::ptr::addr_of_mut!(index), 8, 8) as usize,
                0x20
            );
            assert_eq!(index, 2);

            assert_eq!(
                linux_xa_find_after(xa_ptr, core::ptr::addr_of_mut!(index), 8, 8) as usize,
                0x80
            );
            assert_eq!(index, 8);
            assert!(linux_xa_find_after(xa_ptr, core::ptr::addr_of_mut!(index), 8, 8).is_null());

            index = 0;
            assert!(linux_xa_find(xa_ptr, core::ptr::addr_of_mut!(index), 8, 0).is_null());
            linux_xa_destroy(xa_ptr);
        }
    }
}
