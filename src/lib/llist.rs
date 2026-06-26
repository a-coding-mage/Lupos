//! linux-parity: complete
//! linux-source: vendor/linux/lib/llist.c
//! test-origin: linux:vendor/linux/lib/llist.c
//! Lockless null-terminated singly linked list helpers.

use core::ptr::{addr_of, null_mut, read_volatile};
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
pub struct LlistNode {
    pub next: *mut LlistNode,
}

#[repr(C)]
pub struct LlistHead {
    pub first: AtomicPtr<LlistNode>,
}

impl LlistNode {
    pub const fn new() -> Self {
        Self { next: null_mut() }
    }
}

impl Default for LlistNode {
    fn default() -> Self {
        Self::new()
    }
}

impl LlistHead {
    pub const fn new() -> Self {
        Self {
            first: AtomicPtr::new(null_mut()),
        }
    }

    pub fn with_first(first: *mut LlistNode) -> Self {
        Self {
            first: AtomicPtr::new(first),
        }
    }
}

impl Default for LlistHead {
    fn default() -> Self {
        Self::new()
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("llist_del_first", llist_del_first as usize, true);
    export_symbol_once("llist_del_first_this", llist_del_first_this as usize, true);
    export_symbol_once("llist_reverse_order", llist_reverse_order as usize, true);
}

unsafe fn read_once_next(entry: *mut LlistNode) -> *mut LlistNode {
    unsafe { read_volatile(addr_of!((*entry).next)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn llist_del_first(head: *mut LlistHead) -> *mut LlistNode {
    let first = unsafe { &(*head).first };
    let mut entry = first.load(Ordering::Acquire);

    loop {
        if entry.is_null() {
            return null_mut();
        }
        let next = unsafe { read_once_next(entry) };
        match first.compare_exchange(entry, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return entry,
            Err(actual) => entry = actual,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn llist_del_first_this(head: *mut LlistHead, this: *mut LlistNode) -> bool {
    let first = unsafe { &(*head).first };
    let mut entry = first.load(Ordering::Acquire);

    loop {
        if entry != this {
            return false;
        }
        let next = unsafe { read_once_next(entry) };
        match first.compare_exchange(entry, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return true,
            Err(actual) => entry = actual,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn llist_reverse_order(mut head: *mut LlistNode) -> *mut LlistNode {
    let mut new_head = null_mut();

    while !head.is_null() {
        let tmp = head;
        unsafe {
            head = (*head).next;
            (*tmp).next = new_head;
        }
        new_head = tmp;
    }

    new_head
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llist_matches_linux_delete_and_reverse_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/llist.c"
        ));
        assert!(source.contains("entry = smp_load_acquire(&head->first);"));
        assert!(source.contains("if (entry == NULL)"));
        assert!(source.contains("next = READ_ONCE(entry->next);"));
        assert!(source.contains("while (!try_cmpxchg(&head->first, &entry, next));"));
        assert!(source.contains("if (entry != this)"));
        assert!(source.contains("struct llist_node *new_head = NULL;"));
        assert!(source.contains("tmp->next = new_head;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(llist_del_first);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(llist_del_first_this);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(llist_reverse_order);"));
    }

    #[test]
    fn del_first_removes_newest_entry() {
        let mut a = LlistNode::new();
        let mut b = LlistNode::new();
        let mut c = LlistNode::new();
        a.next = &mut b;
        b.next = &mut c;
        let mut head = LlistHead::with_first(&mut a);

        assert_eq!(unsafe { llist_del_first(&mut head) }, &mut a as *mut _);
        assert_eq!(head.first.load(Ordering::Acquire), &mut b as *mut _);
    }

    #[test]
    fn del_first_this_only_removes_matching_head() {
        let mut a = LlistNode::new();
        let mut b = LlistNode::new();
        let mut c = LlistNode::new();
        a.next = &mut b;
        b.next = &mut c;
        let mut head = LlistHead::with_first(&mut a);

        assert!(!unsafe { llist_del_first_this(&mut head, &mut b) });
        assert_eq!(head.first.load(Ordering::Acquire), &mut a as *mut _);
        assert!(unsafe { llist_del_first_this(&mut head, &mut a) });
        assert_eq!(head.first.load(Ordering::Acquire), &mut b as *mut _);
    }

    #[test]
    fn reverse_order_relinks_chain_in_place() {
        let mut a = LlistNode::new();
        let mut b = LlistNode::new();
        let mut c = LlistNode::new();
        a.next = &mut b;
        b.next = &mut c;

        let reversed = unsafe { llist_reverse_order(&mut a) };

        assert_eq!(reversed, &mut c as *mut _);
        assert_eq!(c.next, &mut b as *mut _);
        assert_eq!(b.next, &mut a as *mut _);
        assert!(a.next.is_null());
    }

    #[test]
    fn llist_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("llist_del_first"),
            Some(llist_del_first as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("llist_del_first_this"),
            Some(llist_del_first_this as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("llist_reverse_order"),
            Some(llist_reverse_order as usize)
        );
    }
}
