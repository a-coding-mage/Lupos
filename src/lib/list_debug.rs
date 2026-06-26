//! linux-parity: complete
//! linux-source: vendor/linux/lib/list_debug.c
//! test-origin: linux:vendor/linux/lib/list_debug.c
//! Linked-list hardening validation helpers.

use crate::kernel::module::{export_symbol, find_symbol};

pub const LIST_POISON1: usize = 0x100;
pub const LIST_POISON2: usize = 0x122;

#[repr(C)]
#[derive(Debug)]
pub struct ListHead {
    pub next: *mut ListHead,
    pub prev: *mut ListHead,
}

impl ListHead {
    pub const fn uninit() -> Self {
        Self {
            next: core::ptr::null_mut(),
            prev: core::ptr::null_mut(),
        }
    }

    pub fn init(&mut self) {
        let ptr = self as *mut Self;
        self.next = ptr;
        self.prev = ptr;
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__list_add_valid_or_report",
        __list_add_valid_or_report as usize,
        false,
    );
    export_symbol_once(
        "__list_del_entry_valid_or_report",
        __list_del_entry_valid_or_report as usize,
        false,
    );
}

fn is_poison(ptr: *const ListHead, value: usize) -> bool {
    ptr as usize == value
}

pub unsafe extern "C" fn __list_add_valid_or_report(
    new: *mut ListHead,
    prev: *mut ListHead,
    next: *mut ListHead,
) -> bool {
    if prev.is_null() || next.is_null() {
        return false;
    }

    if unsafe { (*next).prev } != prev {
        return false;
    }
    if unsafe { (*prev).next } != next {
        return false;
    }
    if new == prev || new == next {
        return false;
    }

    true
}

pub unsafe extern "C" fn __list_del_entry_valid_or_report(entry: *mut ListHead) -> bool {
    if entry.is_null() {
        return false;
    }

    let prev = unsafe { (*entry).prev };
    let next = unsafe { (*entry).next };

    if next.is_null() || prev.is_null() {
        return false;
    }
    if is_poison(next, LIST_POISON1) || is_poison(prev, LIST_POISON2) {
        return false;
    }
    if unsafe { (*prev).next } != entry {
        return false;
    }
    if unsafe { (*next).prev } != entry {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_debug_validates_linux_add_and_del_invariants() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/list_debug.c"
        ));
        assert!(source.contains("prev == NULL"));
        assert!(source.contains("next == NULL"));
        assert!(source.contains("next->prev != prev"));
        assert!(source.contains("prev->next != next"));
        assert!(source.contains("new == prev || new == next"));
        assert!(source.contains("LIST_POISON1"));
        assert!(source.contains("LIST_POISON2"));
        assert!(source.contains("EXPORT_SYMBOL(__list_del_entry_valid_or_report);"));

        let mut head = ListHead::uninit();
        let mut entry = ListHead::uninit();
        head.init();
        entry.next = &mut head;
        entry.prev = &mut head;
        head.next = &mut entry;
        head.prev = &mut entry;

        unsafe {
            assert!(__list_del_entry_valid_or_report(&mut entry));
            assert!(!__list_add_valid_or_report(
                &mut entry,
                core::ptr::null_mut(),
                &mut head
            ));
            entry.next = LIST_POISON1 as *mut ListHead;
            assert!(!__list_del_entry_valid_or_report(&mut entry));
        }
    }
}
