//! linux-parity: partial
//! linux-source: vendor/linux/lib/idr.c
//! test-origin: linux:vendor/linux/lib/idr.c
//! IDA exports used by Linux-built modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENOMEM, ENOSPC};
use crate::kernel::module::{export_symbol, find_symbol};

struct IdaState {
    ida: usize,
    allocated: Vec<bool>,
}

static IDA_STATES: Mutex<Vec<IdaState>> = Mutex::new(Vec::new());

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ida_alloc_range", linux_ida_alloc_range as usize, true);
    export_symbol_once("ida_free", linux_ida_free as usize, true);
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
}
