//! linux-parity: partial
//! linux-source: vendor/linux/lib/maple_tree.c
//! test-origin: linux:vendor/linux/lib/maple_tree.c
//! Maple-tree ABI helpers for Linux-built modules.
//!
//! Lupos' native VMA maple tree lives in `mm::maple_tree` and uses Rust-owned
//! storage.  Vendor modules pass Linux-layout `struct ma_state *` values, so
//! these exports must not reinterpret them as native Lupos maple trees.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

const LINUX_MA_STATE_INDEX_OFFSET: usize = 8;
const LINUX_MA_STATE_LAST_OFFSET: usize = 16;
const LINUX_MA_STATE_STATUS_OFFSET: usize = 72;
const LINUX_MA_STATUS_OVERFLOW: i32 = 5;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mas_find", linux_mas_find as usize, true);
}

/// `mas_find` - `vendor/linux/lib/maple_tree.c:5381`.
///
/// The raw Linux maple tree owned by a vendor module is opaque to Lupos.  Mark
/// the state as exhausted and return NULL instead of walking unknown node
/// storage with incompatible ownership and RCU rules.
pub unsafe extern "C" fn linux_mas_find(mas: *mut c_void, max: usize) -> *mut c_void {
    if !mas.is_null() {
        unsafe {
            *mas.cast::<u8>()
                .add(LINUX_MA_STATE_INDEX_OFFSET)
                .cast::<usize>() = max;
            *mas.cast::<u8>()
                .add(LINUX_MA_STATE_LAST_OFFSET)
                .cast::<usize>() = max;
            *mas.cast::<u8>()
                .add(LINUX_MA_STATE_STATUS_OFFSET)
                .cast::<i32>() = LINUX_MA_STATUS_OVERFLOW;
        }
    }
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mas_find_marks_state_exhausted() {
        let mut state = [0u8; 88];
        let result = unsafe { linux_mas_find(state.as_mut_ptr().cast(), 42) };
        assert!(result.is_null());
        assert_eq!(
            unsafe {
                *state
                    .as_ptr()
                    .add(LINUX_MA_STATE_INDEX_OFFSET)
                    .cast::<usize>()
            },
            42
        );
        assert_eq!(
            unsafe {
                *state
                    .as_ptr()
                    .add(LINUX_MA_STATE_STATUS_OFFSET)
                    .cast::<i32>()
            },
            LINUX_MA_STATUS_OVERFLOW
        );
    }
}
