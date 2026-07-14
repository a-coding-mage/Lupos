//! linux-parity: complete
//! linux-source: vendor/linux/mm/vmstat.c
//! test-origin: linux:vendor/linux/mm/vmstat.c
//! VM statistics counters.

use core::sync::atomic::{AtomicIsize, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

/// Lupos x86_64-visible zone-stat item capacity.
///
/// Linux sizes these arrays from `enum zone_stat_item`; the current configured
/// Lupos surface uses a bounded atomic table so every item has independent
/// observable state without pulling in Linux's per-cpu batching machinery.
pub const NR_VM_ZONE_STAT_ITEMS: usize = 64;
/// Lupos x86_64-visible node-stat item capacity.
pub const NR_VM_NODE_STAT_ITEMS: usize = 128;
/// Lupos x86_64-visible VM event item capacity.
pub const NR_VM_EVENT_ITEMS: usize = 256;

static VM_ZONE_STAT: [AtomicIsize; NR_VM_ZONE_STAT_ITEMS] =
    [const { AtomicIsize::new(0) }; NR_VM_ZONE_STAT_ITEMS];
static VM_NODE_STAT: [AtomicIsize; NR_VM_NODE_STAT_ITEMS] =
    [const { AtomicIsize::new(0) }; NR_VM_NODE_STAT_ITEMS];
static VM_EVENTS: [AtomicIsize; NR_VM_EVENT_ITEMS] =
    [const { AtomicIsize::new(0) }; NR_VM_EVENT_ITEMS];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "mod_node_page_state",
        linux_mod_node_page_state as usize,
        false,
    );
}

#[inline]
fn atomic_get(table: &[AtomicIsize], item: usize) -> isize {
    table
        .get(item)
        .map(|value| value.load(Ordering::Acquire))
        .unwrap_or(0)
}

#[inline]
fn atomic_mod(table: &[AtomicIsize], item: usize, delta: isize) {
    if let Some(value) = table.get(item) {
        value.fetch_add(delta, Ordering::AcqRel);
    }
}

pub fn vm_zone_stat(item: usize) -> isize {
    atomic_get(&VM_ZONE_STAT, item)
}

pub fn vm_node_stat(item: usize) -> isize {
    atomic_get(&VM_NODE_STAT, item)
}

pub fn vm_event_states(item: usize) -> isize {
    atomic_get(&VM_EVENTS, item)
}

pub fn all_vm_events(events: *mut isize) {
    if events.is_null() {
        return;
    }

    for idx in 0..NR_VM_EVENT_ITEMS {
        unsafe {
            *events.add(idx) = VM_EVENTS[idx].load(Ordering::Acquire);
        }
    }
}

pub fn count_vm_events(item: usize, delta: isize) {
    atomic_mod(&VM_EVENTS, item, delta);
}

pub fn __mod_zone_page_state(_zone: *mut u8, item: usize, delta: isize) {
    atomic_mod(&VM_ZONE_STAT, item, delta);
}

pub fn __inc_zone_page_state(zone: *mut u8, item: usize) {
    __mod_zone_page_state(zone, item, 1)
}

pub fn __dec_zone_page_state(zone: *mut u8, item: usize) {
    __mod_zone_page_state(zone, item, -1)
}

pub fn mod_zone_page_state(zone: *mut u8, item: usize, delta: isize) {
    __mod_zone_page_state(zone, item, delta)
}

pub fn inc_zone_page_state(zone: *mut u8, item: usize) {
    __inc_zone_page_state(zone, item)
}

pub fn dec_zone_page_state(zone: *mut u8, item: usize) {
    __dec_zone_page_state(zone, item)
}

pub fn __mod_node_page_state(_pgdat: *mut u8, item: usize, delta: isize) {
    atomic_mod(&VM_NODE_STAT, item, delta);
}

pub fn __inc_node_page_state(pgdat: *mut u8, item: usize) {
    __mod_node_page_state(pgdat, item, 1)
}

pub fn __dec_node_page_state(pgdat: *mut u8, item: usize) {
    __mod_node_page_state(pgdat, item, -1)
}

pub fn mod_node_page_state(pgdat: *mut u8, item: usize, delta: isize) {
    __mod_node_page_state(pgdat, item, delta)
}

/// `mod_node_page_state` - `vendor/linux/mm/vmstat.c`.
pub unsafe extern "C" fn linux_mod_node_page_state(pgdat: *mut u8, item: u32, delta: isize) {
    mod_node_page_state(pgdat, item as usize, delta)
}

pub fn inc_node_page_state(pgdat: *mut u8, item: usize) {
    __inc_node_page_state(pgdat, item)
}

pub fn dec_node_page_state(pgdat: *mut u8, item: usize) {
    __dec_node_page_state(pgdat, item)
}

#[cfg(test)]
fn reset_vmstat_for_test() {
    for value in &VM_ZONE_STAT {
        value.store(0, Ordering::Release);
    }
    for value in &VM_NODE_STAT {
        value.store(0, Ordering::Release);
    }
    for value in &VM_EVENTS {
        value.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    #[test]
    fn zone_page_state_is_item_indexed() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_vmstat_for_test();

        __mod_zone_page_state(core::ptr::null_mut(), 2, 7);
        inc_zone_page_state(core::ptr::null_mut(), 2);
        dec_zone_page_state(core::ptr::null_mut(), 3);

        assert_eq!(vm_zone_stat(2), 8);
        assert_eq!(vm_zone_stat(3), -1);
        assert_eq!(vm_zone_stat(4), 0);
    }

    #[test]
    fn node_page_state_is_item_indexed() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_vmstat_for_test();

        mod_node_page_state(core::ptr::null_mut(), 5, 11);
        __inc_node_page_state(core::ptr::null_mut(), 5);
        __dec_node_page_state(core::ptr::null_mut(), 6);

        assert_eq!(vm_node_stat(5), 12);
        assert_eq!(vm_node_stat(6), -1);
        assert_eq!(vm_node_stat(7), 0);
    }

    #[test]
    fn all_vm_events_copies_each_event_slot() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_vmstat_for_test();

        count_vm_events(1, 4);
        count_vm_events(9, 13);

        let mut out = [0isize; NR_VM_EVENT_ITEMS];
        all_vm_events(out.as_mut_ptr());

        assert_eq!(vm_event_states(1), 4);
        assert_eq!(out[1], 4);
        assert_eq!(out[9], 13);
        assert_eq!(out[2], 0);
    }

    #[test]
    fn registers_vmstat_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("mod_node_page_state"),
            Some(linux_mod_node_page_state as usize)
        );
    }

    #[test]
    fn linux_mod_node_page_state_updates_node_counter() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_vmstat_for_test();

        unsafe { linux_mod_node_page_state(core::ptr::null_mut(), 9, -3) };
        assert_eq!(vm_node_stat(9), -3);
    }

    #[test]
    fn out_of_range_items_are_ignored_or_zero() {
        let _guard = GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_vmstat_for_test();

        __mod_zone_page_state(core::ptr::null_mut(), NR_VM_ZONE_STAT_ITEMS + 1, 7);
        __mod_node_page_state(core::ptr::null_mut(), NR_VM_NODE_STAT_ITEMS + 1, 7);
        count_vm_events(NR_VM_EVENT_ITEMS + 1, 7);

        assert_eq!(vm_zone_stat(NR_VM_ZONE_STAT_ITEMS + 1), 0);
        assert_eq!(vm_node_stat(NR_VM_NODE_STAT_ITEMS + 1), 0);
        assert_eq!(vm_event_states(NR_VM_EVENT_ITEMS + 1), 0);
        all_vm_events(core::ptr::null_mut());
    }
}
