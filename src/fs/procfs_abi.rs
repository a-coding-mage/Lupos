//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/generic.c
//! Opaque procfs registration ABI for Linux-built modules.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::ENOENT;
use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
struct ProcDirEntry {
    name: *const c_char,
    parent: usize,
    data: usize,
    size: u64,
}

lazy_static! {
    static ref PROC_ENTRIES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("proc_create", proc_create as usize, false);
    export_symbol_once("proc_create_data", proc_create_data as usize, false);
    export_symbol_once(
        "proc_create_seq_private",
        proc_create_seq_private as usize,
        false,
    );
    export_symbol_once(
        "proc_create_single_data",
        proc_create_single_data as usize,
        false,
    );
    export_symbol_once("proc_mkdir", proc_mkdir as usize, false);
    export_symbol_once("proc_mkdir_mode", proc_mkdir_mode as usize, false);
    export_symbol_once("proc_remove", proc_remove as usize, false);
    export_symbol_once("remove_proc_entry", remove_proc_entry as usize, false);
    export_symbol_once("remove_proc_subtree", remove_proc_subtree as usize, false);
    export_symbol_once("proc_set_size", proc_set_size as usize, false);
    export_symbol_once("proc_symlink", proc_symlink as usize, false);
}

fn alloc_entry(name: *const c_char, parent: *mut c_void, data: *mut c_void) -> *mut ProcDirEntry {
    if name.is_null() {
        return core::ptr::null_mut();
    }
    let entry = Box::into_raw(Box::new(ProcDirEntry {
        name,
        parent: parent as usize,
        data: data as usize,
        size: 0,
    }));
    PROC_ENTRIES.lock().push(entry as usize);
    entry
}

unsafe fn proc_name_eq(entry: *const ProcDirEntry, name: *const c_char) -> bool {
    if entry.is_null() || name.is_null() {
        return false;
    }

    let entry_name = unsafe { (*entry).name };
    if entry_name.is_null() {
        return false;
    }

    let entry_len = unsafe { crate::lib::string::c_strlen(entry_name, 512) };
    let name_len = unsafe { crate::lib::string::c_strlen(name, 512) };
    if entry_len != name_len {
        return false;
    }

    let entry_bytes = unsafe { core::slice::from_raw_parts(entry_name.cast::<u8>(), entry_len) };
    let name_bytes = unsafe { core::slice::from_raw_parts(name.cast::<u8>(), name_len) };
    entry_bytes == name_bytes
}

unsafe fn free_proc_entry(entry: *mut c_void) {
    unsafe {
        let _ = Box::from_raw(entry.cast::<ProcDirEntry>());
    }
}

#[unsafe(export_name = "proc_create")]
pub unsafe extern "C" fn proc_create(
    name: *const c_char,
    _mode: u16,
    parent: *mut c_void,
    _proc_ops: *const c_void,
) -> *mut c_void {
    alloc_entry(name, parent, core::ptr::null_mut()).cast()
}

#[unsafe(export_name = "proc_create_data")]
pub unsafe extern "C" fn proc_create_data(
    name: *const c_char,
    _mode: u16,
    parent: *mut c_void,
    _proc_ops: *const c_void,
    data: *mut c_void,
) -> *mut c_void {
    alloc_entry(name, parent, data).cast()
}

#[unsafe(export_name = "proc_create_seq_private")]
pub unsafe extern "C" fn proc_create_seq_private(
    name: *const c_char,
    _mode: u16,
    parent: *mut c_void,
    _seq_ops: *const c_void,
    _state_size: usize,
    data: *mut c_void,
) -> *mut c_void {
    alloc_entry(name, parent, data).cast()
}

#[unsafe(export_name = "proc_create_single_data")]
pub unsafe extern "C" fn proc_create_single_data(
    name: *const c_char,
    _mode: u16,
    parent: *mut c_void,
    _show: *const c_void,
    data: *mut c_void,
) -> *mut c_void {
    alloc_entry(name, parent, data).cast()
}

#[unsafe(export_name = "proc_mkdir")]
pub unsafe extern "C" fn proc_mkdir(name: *const c_char, parent: *mut c_void) -> *mut c_void {
    alloc_entry(name, parent, core::ptr::null_mut()).cast()
}

#[unsafe(export_name = "proc_mkdir_mode")]
pub unsafe extern "C" fn proc_mkdir_mode(
    name: *const c_char,
    _mode: u16,
    parent: *mut c_void,
) -> *mut c_void {
    alloc_entry(name, parent, core::ptr::null_mut()).cast()
}

#[unsafe(export_name = "proc_symlink")]
pub unsafe extern "C" fn proc_symlink(
    name: *const c_char,
    parent: *mut c_void,
    _dest: *const c_char,
) -> *mut c_void {
    alloc_entry(name, parent, core::ptr::null_mut()).cast()
}

#[unsafe(export_name = "proc_set_size")]
unsafe extern "C" fn proc_set_size(entry: *mut ProcDirEntry, size: u64) {
    if !entry.is_null() {
        unsafe {
            (*entry).size = size;
        }
    }
}

#[unsafe(export_name = "proc_remove")]
pub unsafe extern "C" fn proc_remove(entry: *mut c_void) {
    if entry.is_null() {
        return;
    }
    let mut entries = PROC_ENTRIES.lock();
    if let Some(pos) = entries.iter().position(|ptr| *ptr == entry as usize) {
        entries.swap_remove(pos);
        unsafe { free_proc_entry(entry) };
    }
}

#[unsafe(export_name = "remove_proc_entry")]
pub unsafe extern "C" fn remove_proc_entry(_name: *const c_char, _parent: *mut c_void) {}

#[unsafe(export_name = "remove_proc_subtree")]
pub unsafe extern "C" fn remove_proc_subtree(name: *const c_char, parent: *mut c_void) -> i32 {
    let mut entries = PROC_ENTRIES.lock();
    let Some(pos) = entries.iter().position(|ptr| {
        let entry = *ptr as *const ProcDirEntry;
        unsafe { (*entry).parent == parent as usize && proc_name_eq(entry, name) }
    }) else {
        return -ENOENT;
    };

    let root = entries.swap_remove(pos);
    let mut removed = Vec::new();
    removed.push(root);

    let mut idx = 0;
    while idx < entries.len() {
        let entry = entries[idx] as *const ProcDirEntry;
        let parent_removed = unsafe { removed.contains(&(*entry).parent) };
        if parent_removed {
            removed.push(entries.swap_remove(idx));
        } else {
            idx += 1;
        }
    }
    drop(entries);

    for entry in removed {
        unsafe { free_proc_entry(entry as *mut c_void) };
    }
    0
}
