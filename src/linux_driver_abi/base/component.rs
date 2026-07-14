//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/component.c
//! test-origin: vendor/linux/drivers/base/component.c
//! Linux component helper registry.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::ptr;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::device::LinuxDevice;

#[repr(C)]
pub struct LinuxComponentOps {
    pub bind: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut LinuxDevice, *mut c_void) -> i32>,
    pub unbind: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut LinuxDevice, *mut c_void)>,
}

#[repr(C)]
pub struct LinuxComponentMasterOps {
    pub bind: Option<unsafe extern "C" fn(*mut LinuxDevice) -> i32>,
    pub unbind: Option<unsafe extern "C" fn(*mut LinuxDevice)>,
}

pub struct LinuxComponentMatch {
    entries: Vec<ComponentMatchEntry>,
}

#[derive(Clone, Copy)]
struct ComponentMatchEntry {
    data: usize,
    compare: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void) -> i32>,
    compare_typed: Option<unsafe extern "C" fn(*mut LinuxDevice, i32, *mut c_void) -> i32>,
    release: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void)>,
    component: Option<ComponentKey>,
    duplicate: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentKey {
    dev: usize,
    ops: usize,
    subcomponent: i32,
}

struct Component {
    key: ComponentKey,
    bound: bool,
    aggregate: Option<AggregateKey>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AggregateKey {
    parent: usize,
    ops: usize,
}

struct AggregateDevice {
    key: AggregateKey,
    match_ptr: usize,
    bound: bool,
}

lazy_static! {
    static ref COMPONENTS: Mutex<Vec<Component>> = Mutex::new(Vec::new());
    static ref AGGREGATES: Mutex<Vec<AggregateDevice>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "component_compare_of",
        linux_component_compare_of as usize,
        true,
    );
    export_symbol_once(
        "component_release_of",
        linux_component_release_of as usize,
        true,
    );
    export_symbol_once(
        "component_compare_dev",
        linux_component_compare_dev as usize,
        true,
    );
    export_symbol_once(
        "component_compare_dev_name",
        linux_component_compare_dev_name as usize,
        true,
    );
    export_symbol_once(
        "component_match_add_release",
        linux_component_match_add_release as usize,
        false,
    );
    export_symbol_once(
        "component_match_add_typed",
        linux_component_match_add_typed as usize,
        false,
    );
    export_symbol_once(
        "component_master_add_with_match",
        linux_component_master_add_with_match as usize,
        true,
    );
    export_symbol_once(
        "component_master_del",
        linux_component_master_del as usize,
        true,
    );
    export_symbol_once(
        "component_master_is_bound",
        linux_component_master_is_bound as usize,
        true,
    );
    export_symbol_once(
        "component_bind_all",
        linux_component_bind_all as usize,
        true,
    );
    export_symbol_once(
        "component_unbind_all",
        linux_component_unbind_all as usize,
        true,
    );
    export_symbol_once("component_add", linux_component_add as usize, true);
    export_symbol_once(
        "component_add_typed",
        linux_component_add_typed as usize,
        true,
    );
    export_symbol_once("component_del", linux_component_del as usize, true);
}

fn is_err_ptr<T>(ptr: *const T) -> bool {
    let value = ptr as usize;
    value >= usize::MAX - 4095
}

fn match_from_ptr<'a>(ptr: *mut LinuxComponentMatch) -> Option<&'a mut LinuxComponentMatch> {
    if ptr.is_null() || is_err_ptr(ptr) {
        None
    } else {
        Some(unsafe { &mut *ptr })
    }
}

fn component_match_alloc() -> *mut LinuxComponentMatch {
    Box::into_raw(Box::new(LinuxComponentMatch {
        entries: Vec::new(),
    }))
}

fn component_match_add_entry(
    matchptr: *mut *mut LinuxComponentMatch,
    release: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void)>,
    compare: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void) -> i32>,
    compare_typed: Option<unsafe extern "C" fn(*mut LinuxDevice, i32, *mut c_void) -> i32>,
    compare_data: *mut c_void,
) {
    if matchptr.is_null() {
        return;
    }

    let mut match_ptr = unsafe { *matchptr };
    if is_err_ptr(match_ptr) {
        return;
    }
    if match_ptr.is_null() {
        match_ptr = component_match_alloc();
        unsafe {
            *matchptr = match_ptr;
        }
    }

    if let Some(component_match) = match_from_ptr(match_ptr) {
        component_match.entries.push(ComponentMatchEntry {
            data: compare_data as usize,
            compare,
            compare_typed,
            release,
            component: None,
            duplicate: false,
        });
    }
}

fn aggregate_index(aggregates: &[AggregateDevice], key: AggregateKey) -> Option<usize> {
    aggregates.iter().position(|aggregate| aggregate.key == key)
}

fn component_index(components: &[Component], key: ComponentKey) -> Option<usize> {
    components.iter().position(|component| component.key == key)
}

fn match_component(entry: &ComponentMatchEntry, component: &Component) -> bool {
    let dev = component.key.dev as *mut LinuxDevice;
    let data = entry.data as *mut c_void;
    if let Some(compare) = entry.compare {
        return unsafe { compare(dev, data) } != 0;
    }
    if let Some(compare_typed) = entry.compare_typed {
        return unsafe { compare_typed(dev, component.key.subcomponent, data) } != 0;
    }
    false
}

fn find_component_for_entry(
    components: &[Component],
    aggregate: AggregateKey,
    entry: &ComponentMatchEntry,
) -> Option<(usize, bool)> {
    components.iter().enumerate().find_map(|(idx, component)| {
        if component
            .aggregate
            .is_some_and(|existing| existing != aggregate)
        {
            return None;
        }
        if match_component(entry, component) {
            Some((idx, component.aggregate.is_some()))
        } else {
            None
        }
    })
}

fn resolve_components_for_aggregate(key: AggregateKey, match_ptr: usize) -> bool {
    let Some(component_match) = match_from_ptr(match_ptr as *mut LinuxComponentMatch) else {
        return true;
    };
    let mut components = COMPONENTS.lock();
    for entry in &mut component_match.entries {
        if entry.component.is_some() {
            continue;
        }
        let Some((idx, duplicate)) = find_component_for_entry(&components, key, entry) else {
            return false;
        };
        entry.duplicate = duplicate;
        entry.component = Some(components[idx].key);
        components[idx].aggregate = Some(key);
    }
    true
}

fn try_to_bring_up_aggregate(key: AggregateKey, match_ptr: usize) -> i32 {
    if !resolve_components_for_aggregate(key, match_ptr) {
        return 0;
    }

    let bind = unsafe { (*(key.ops as *const LinuxComponentMasterOps)).bind };
    let ret = bind
        .map(|bind| unsafe { bind(key.parent as *mut LinuxDevice) })
        .unwrap_or(0);
    if ret < 0 {
        detach_aggregate_components(key, match_ptr);
        return ret;
    }

    let mut aggregates = AGGREGATES.lock();
    if let Some(idx) = aggregate_index(&aggregates, key) {
        aggregates[idx].bound = true;
    }
    1
}

fn detach_aggregate_components(key: AggregateKey, match_ptr: usize) {
    if let Some(component_match) = match_from_ptr(match_ptr as *mut LinuxComponentMatch) {
        let mut components = COMPONENTS.lock();
        for entry in &mut component_match.entries {
            if let Some(component_key) = entry.component.take() {
                if let Some(idx) = component_index(&components, component_key) {
                    if components[idx].aggregate == Some(key) {
                        components[idx].aggregate = None;
                        components[idx].bound = false;
                    }
                }
            }
        }
    }
}

fn try_to_bring_up_matching_aggregates(component_key: ComponentKey) -> i32 {
    let aggregate_keys: Vec<(AggregateKey, usize)> = {
        let aggregates = AGGREGATES.lock();
        aggregates
            .iter()
            .filter(|aggregate| !aggregate.bound)
            .map(|aggregate| (aggregate.key, aggregate.match_ptr))
            .collect()
    };

    for (key, match_ptr) in aggregate_keys {
        let interested = match_from_ptr(match_ptr as *mut LinuxComponentMatch)
            .map(|component_match| {
                component_match.entries.iter().any(|entry| {
                    entry.component == Some(component_key) || {
                        let components = COMPONENTS.lock();
                        component_index(&components, component_key)
                            .map(|idx| match_component(entry, &components[idx]))
                            .unwrap_or(false)
                    }
                })
            })
            .unwrap_or(false);
        if interested {
            let ret = try_to_bring_up_aggregate(key, match_ptr);
            if ret < 0 {
                return ret;
            }
        }
    }
    0
}

fn linux_dev_name_ptr(dev: *mut LinuxDevice) -> *const c_char {
    if dev.is_null() {
        return ptr::null();
    }
    let dev = unsafe { &*dev };
    if !dev.kobj.name.is_null() {
        dev.kobj.name
    } else {
        dev.init_name
    }
}

/// `component_compare_of` - `vendor/linux/drivers/base/component.c:293`.
pub unsafe extern "C" fn linux_component_compare_of(
    _dev: *mut LinuxDevice,
    _data: *mut c_void,
) -> i32 {
    0
}

/// `component_release_of` - `vendor/linux/drivers/base/component.c:306`.
pub unsafe extern "C" fn linux_component_release_of(_dev: *mut LinuxDevice, _data: *mut c_void) {}

/// `component_compare_dev` - `vendor/linux/drivers/base/component.c:321`.
pub unsafe extern "C" fn linux_component_compare_dev(
    dev: *mut LinuxDevice,
    data: *mut c_void,
) -> i32 {
    (dev.cast::<c_void>() == data) as i32
}

/// `component_compare_dev_name` - `vendor/linux/drivers/base/component.c:335`.
pub unsafe extern "C" fn linux_component_compare_dev_name(
    dev: *mut LinuxDevice,
    data: *mut c_void,
) -> i32 {
    let name = linux_dev_name_ptr(dev);
    if name.is_null() || data.is_null() {
        return 0;
    }
    (unsafe { crate::lib::string::linux_strcmp(name, data.cast::<c_char>()) } == 0) as i32
}

/// `component_match_add_release` - `vendor/linux/drivers/base/component.c:445`.
pub unsafe extern "C" fn linux_component_match_add_release(
    _parent: *mut LinuxDevice,
    matchptr: *mut *mut LinuxComponentMatch,
    release: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void)>,
    compare: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void) -> i32>,
    compare_data: *mut c_void,
) {
    component_match_add_entry(matchptr, release, compare, None, compare_data);
}

/// `component_match_add_typed` - `vendor/linux/drivers/base/component.c:472`.
pub unsafe extern "C" fn linux_component_match_add_typed(
    _parent: *mut LinuxDevice,
    matchptr: *mut *mut LinuxComponentMatch,
    compare_typed: Option<unsafe extern "C" fn(*mut LinuxDevice, i32, *mut c_void) -> i32>,
    compare_data: *mut c_void,
) {
    component_match_add_entry(matchptr, None, None, compare_typed, compare_data);
}

/// `component_master_add_with_match` - `vendor/linux/drivers/base/component.c:512`.
pub unsafe extern "C" fn linux_component_master_add_with_match(
    parent: *mut LinuxDevice,
    ops: *const LinuxComponentMasterOps,
    match_ptr: *mut LinuxComponentMatch,
) -> i32 {
    if parent.is_null() || ops.is_null() || is_err_ptr(match_ptr) {
        return -EINVAL;
    }

    let key = AggregateKey {
        parent: parent as usize,
        ops: ops as usize,
    };
    {
        let mut aggregates = AGGREGATES.lock();
        if aggregate_index(&aggregates, key).is_none() {
            aggregates.push(AggregateDevice {
                key,
                match_ptr: match_ptr as usize,
                bound: false,
            });
        }
    }

    let ret = try_to_bring_up_aggregate(key, match_ptr as usize);
    if ret < 0 {
        linux_component_master_del(parent, ops);
        ret
    } else {
        0
    }
}

/// `component_master_del` - `vendor/linux/drivers/base/component.c:557`.
pub unsafe extern "C" fn linux_component_master_del(
    parent: *mut LinuxDevice,
    ops: *const LinuxComponentMasterOps,
) {
    let key = AggregateKey {
        parent: parent as usize,
        ops: ops as usize,
    };
    let removed = {
        let mut aggregates = AGGREGATES.lock();
        aggregate_index(&aggregates, key).map(|idx| aggregates.remove(idx))
    };

    if let Some(aggregate) = removed {
        if aggregate.bound {
            if let Some(unbind) = unsafe { (*(key.ops as *const LinuxComponentMasterOps)).unbind } {
                unsafe { unbind(key.parent as *mut LinuxDevice) };
            }
        }
        detach_aggregate_components(key, aggregate.match_ptr);
    }
}

/// `component_master_is_bound` - `vendor/linux/drivers/base/component.c:572`.
pub unsafe extern "C" fn linux_component_master_is_bound(
    parent: *mut LinuxDevice,
    ops: *const LinuxComponentMasterOps,
) -> bool {
    let key = AggregateKey {
        parent: parent as usize,
        ops: ops as usize,
    };
    AGGREGATES
        .lock()
        .iter()
        .find(|aggregate| aggregate.key == key)
        .map(|aggregate| aggregate.bound)
        .unwrap_or(false)
}

/// `component_bind_all` - `vendor/linux/drivers/base/component.c:664`.
pub unsafe extern "C" fn linux_component_bind_all(
    parent: *mut LinuxDevice,
    data: *mut c_void,
) -> i32 {
    let aggregate = {
        let aggregates = AGGREGATES.lock();
        aggregates
            .iter()
            .find(|aggregate| aggregate.key.parent == parent as usize)
            .map(|aggregate| (aggregate.key, aggregate.match_ptr))
    };
    let Some((aggregate_key, match_ptr)) = aggregate else {
        return -EINVAL;
    };
    let Some(component_match) = match_from_ptr(match_ptr as *mut LinuxComponentMatch) else {
        return 0;
    };

    let keys: Vec<ComponentKey> = component_match
        .entries
        .iter()
        .filter(|entry| !entry.duplicate)
        .filter_map(|entry| entry.component)
        .collect();

    let mut bound: Vec<ComponentKey> = Vec::new();
    for key in keys {
        let ops = key.ops as *const LinuxComponentOps;
        let ret = unsafe {
            (*ops)
                .bind
                .map(|bind| bind(key.dev as *mut LinuxDevice, parent, data))
                .unwrap_or(0)
        };
        if ret != 0 {
            for key in bound.into_iter().rev() {
                let ops = key.ops as *const LinuxComponentOps;
                if let Some(unbind) = unsafe { (*ops).unbind } {
                    unsafe { unbind(key.dev as *mut LinuxDevice, parent, data) };
                }
                mark_component_bound(key, false);
            }
            return ret;
        }
        mark_component_bound(key, true);
        bound.push(key);
    }

    let mut aggregates = AGGREGATES.lock();
    if let Some(idx) = aggregate_index(&aggregates, aggregate_key) {
        aggregates[idx].bound = true;
    }
    0
}

fn mark_component_bound(key: ComponentKey, bound: bool) {
    let mut components = COMPONENTS.lock();
    if let Some(idx) = component_index(&components, key) {
        components[idx].bound = bound;
    }
}

/// `component_unbind_all` - `vendor/linux/drivers/base/component.c:626`.
pub unsafe extern "C" fn linux_component_unbind_all(parent: *mut LinuxDevice, data: *mut c_void) {
    let aggregate = {
        let aggregates = AGGREGATES.lock();
        aggregates
            .iter()
            .find(|aggregate| aggregate.key.parent == parent as usize)
            .map(|aggregate| (aggregate.key, aggregate.match_ptr))
    };
    let Some((_aggregate_key, match_ptr)) = aggregate else {
        return;
    };
    let Some(component_match) = match_from_ptr(match_ptr as *mut LinuxComponentMatch) else {
        return;
    };

    let keys: Vec<ComponentKey> = component_match
        .entries
        .iter()
        .rev()
        .filter(|entry| !entry.duplicate)
        .filter_map(|entry| entry.component)
        .collect();

    for key in keys {
        let ops = key.ops as *const LinuxComponentOps;
        if let Some(unbind) = unsafe { (*ops).unbind } {
            unsafe { unbind(key.dev as *mut LinuxDevice, parent, data) };
        }
        mark_component_bound(key, false);
    }
}

fn component_add(dev: *mut LinuxDevice, ops: *const LinuxComponentOps, subcomponent: i32) -> i32 {
    if dev.is_null() || ops.is_null() {
        return -EINVAL;
    }
    let key = ComponentKey {
        dev: dev as usize,
        ops: ops as usize,
        subcomponent,
    };
    {
        let mut components = COMPONENTS.lock();
        if component_index(&components, key).is_none() {
            components.push(Component {
                key,
                bound: false,
                aggregate: None,
            });
        }
    }
    let ret = try_to_bring_up_matching_aggregates(key);
    if ret < 0 { ret } else { 0 }
}

/// `component_add_typed` - `vendor/linux/drivers/base/component.c:780`.
pub unsafe extern "C" fn linux_component_add_typed(
    dev: *mut LinuxDevice,
    ops: *const LinuxComponentOps,
    subcomponent: i32,
) -> i32 {
    if subcomponent == 0 {
        return -EINVAL;
    }
    component_add(dev, ops, subcomponent)
}

/// `component_add` - `vendor/linux/drivers/base/component.c:805`.
pub unsafe extern "C" fn linux_component_add(
    dev: *mut LinuxDevice,
    ops: *const LinuxComponentOps,
) -> i32 {
    component_add(dev, ops, 0)
}

/// `component_del` - `vendor/linux/drivers/base/component.c:820`.
pub unsafe extern "C" fn linux_component_del(dev: *mut LinuxDevice, ops: *const LinuxComponentOps) {
    let removed = {
        let mut components = COMPONENTS.lock();
        components
            .iter()
            .position(|component| {
                component.key.dev == dev as usize && component.key.ops == ops as usize
            })
            .map(|idx| components.remove(idx))
    };

    if let Some(component) = removed {
        if let Some(aggregate_key) = component.aggregate {
            let match_ptr = AGGREGATES
                .lock()
                .iter()
                .find(|aggregate| aggregate.key == aggregate_key)
                .map(|aggregate| aggregate.match_ptr)
                .unwrap_or(0);
            if match_ptr != 0 {
                if let Some(component_match) = match_from_ptr(match_ptr as *mut LinuxComponentMatch)
                {
                    for entry in &mut component_match.entries {
                        if entry.component == Some(component.key) {
                            entry.component = None;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    COMPONENTS.lock().clear();
    AGGREGATES.lock().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn compare_typed(
        dev: *mut LinuxDevice,
        subcomponent: i32,
        data: *mut c_void,
    ) -> i32 {
        (subcomponent == 7 && dev.cast::<c_void>() == data) as i32
    }

    #[test]
    fn component_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            find_symbol("component_master_add_with_match"),
            Some(linux_component_master_add_with_match as usize)
        );
        assert_eq!(
            find_symbol("component_match_add_typed"),
            Some(linux_component_match_add_typed as usize)
        );
    }

    #[test]
    fn component_master_registration_defers_until_component_exists() {
        reset_for_tests();
        let dev = 0x1000usize as *mut LinuxDevice;
        let ops = 0x2000usize as *const LinuxComponentMasterOps;
        let mut match_ptr: *mut LinuxComponentMatch = ptr::null_mut();

        unsafe {
            linux_component_match_add_typed(dev, &mut match_ptr, Some(compare_typed), dev.cast());
            assert!(!match_ptr.is_null());
            assert_eq!(
                linux_component_master_add_with_match(dev, ops, match_ptr),
                0
            );
            assert!(!linux_component_master_is_bound(dev, ops));
        }
    }
}
