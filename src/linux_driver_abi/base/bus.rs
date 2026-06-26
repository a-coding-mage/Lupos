//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/bus.c
//! test-origin: linux:vendor/linux/drivers/base/bus.c
//! `struct bus_type` — `vendor/linux/include/linux/device/bus.h:83`.
//!
//! A bus owns a `match` callback that decides whether a given driver can
//! drive a given device.  When `device_add` runs, the bus walks its
//! registered drivers; when `driver_register` runs, the driver walks the
//! bus's registered devices.  The first successful match calls
//! `driver_probe_device`.
//!
//! Mirrors `drivers/base/bus.c` (registry) and `drivers/base/dd.c`
//! (`__driver_attach` / `driver_probe_device`).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EEXIST;
use crate::linux_driver_abi::base::device::{Device, DeviceState};
use crate::linux_driver_abi::base::driver::DeviceDriver;

pub type BusMatchFn = fn(dev: &Arc<Device>, drv: &Arc<DeviceDriver>) -> bool;

pub struct BusType {
    pub name: &'static str,
    pub match_fn: BusMatchFn,
    pub devices: Mutex<Vec<Arc<Device>>>,
    pub drivers: Mutex<Vec<Arc<DeviceDriver>>>,
}

impl BusType {
    pub fn new(name: &'static str, match_fn: BusMatchFn) -> Arc<Self> {
        Arc::new(Self {
            name,
            match_fn,
            devices: Mutex::new(Vec::new()),
            drivers: Mutex::new(Vec::new()),
        })
    }
}

lazy_static! {
    pub(crate) static ref BUSES: Mutex<BTreeMap<String, Arc<BusType>>> =
        Mutex::new(BTreeMap::new());
    static ref LINUX_BUS_TYPES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

/// `struct bus_type` — `vendor/linux/include/linux/device/bus.h:83`.
#[repr(C)]
pub struct LinuxBusType {
    pub name: *const c_char,
    pub dev_name: *const c_char,
    pub bus_groups: *const *const c_void,
    pub dev_groups: *const *const c_void,
    pub drv_groups: *const *const c_void,
    pub match_fn: Option<unsafe extern "C" fn(dev: *mut c_void, drv: *const c_void) -> i32>,
    pub uevent: Option<unsafe extern "C" fn(dev: *const c_void, env: *mut c_void) -> i32>,
    pub probe: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub sync_state: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub remove: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub shutdown: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub irq_get_affinity:
        Option<unsafe extern "C" fn(dev: *mut c_void, irq_vec: u32) -> *const c_void>,
    pub online: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub offline: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub suspend: Option<unsafe extern "C" fn(dev: *mut c_void, state: usize) -> i32>,
    pub resume: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub num_vf: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub dma_configure: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub dma_cleanup: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub pm: *const c_void,
    pub driver_override: bool,
    pub need_parent_lock: bool,
}

unsafe impl Sync for LinuxBusType {}

/// `bus_register` — `drivers/base/bus.c:934`.
pub fn bus_register(bus: Arc<BusType>) -> Result<(), i32> {
    let mut g = BUSES.lock();
    if g.contains_key(bus.name) {
        return Err(EEXIST);
    }
    g.insert(String::from(bus.name), bus);
    Ok(())
}

pub fn bus_unregister(name: &str) {
    BUSES.lock().remove(name);
}

pub fn registered_buses() -> Vec<&'static str> {
    BUSES.lock().values().map(|b| b.name).collect()
}

pub fn find_bus(name: &str) -> Option<Arc<BusType>> {
    BUSES.lock().get(name).cloned()
}

pub fn register_linux_bus_type(bus: *const LinuxBusType) {
    if bus.is_null() {
        return;
    }
    let mut buses = LINUX_BUS_TYPES.lock();
    let addr = bus as usize;
    if !buses.contains(&addr) {
        buses.push(addr);
    }
}

pub fn unregister_linux_bus_type(bus: *const LinuxBusType) {
    LINUX_BUS_TYPES
        .lock()
        .retain(|registered| *registered != bus as usize);
}

pub fn linux_bus_type_registered(bus: *const LinuxBusType) -> bool {
    LINUX_BUS_TYPES
        .lock()
        .iter()
        .any(|registered| *registered == bus as usize)
}

/// `bus_probe_device` — `drivers/base/bus.c`.
///
/// On `device_add`, walk every driver registered on this bus and try to
/// match.  First match wins — call `driver_probe_device`.
pub fn bus_probe_device(bus: &Arc<BusType>, dev: &Arc<Device>) {
    let candidates: Vec<Arc<DeviceDriver>> = bus.drivers.lock().iter().cloned().collect();
    for drv in candidates.iter() {
        if (bus.match_fn)(dev, drv) {
            if driver_probe_device(drv, dev).is_ok() {
                return;
            }
        }
    }
}

/// `__driver_attach` — `drivers/base/dd.c:1215`.
///
/// On `driver_register`, walk every device on this bus and try to match this
/// driver.  Each device that matches is bound (probe called).
pub fn __driver_attach(bus: &Arc<BusType>, drv: &Arc<DeviceDriver>) {
    let candidates: Vec<Arc<Device>> = bus.devices.lock().iter().cloned().collect();
    for dev in candidates.iter() {
        if dev.driver.lock().is_some() {
            continue;
        }
        if (bus.match_fn)(dev, drv) {
            let _ = driver_probe_device(drv, dev);
        }
    }
}

/// `driver_probe_device` — `drivers/base/dd.c`.
///
/// Calls `drv.probe(dev)`.  On success, binds the device to the driver and
/// flips the device into the `Bound` state.
pub fn driver_probe_device(drv: &Arc<DeviceDriver>, dev: &Arc<Device>) -> Result<(), i32> {
    if let Some(probe) = drv.probe {
        probe(dev)?;
    }
    *dev.driver.lock() = Some(drv.clone());
    *dev.state.lock() = DeviceState::Bound;
    drv.bound_devices.lock().push(dev.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_bus_type_c_layout_matches_vendor_header_prefix() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxBusType, name), 0);
        assert_eq!(offset_of!(LinuxBusType, dev_name), 8);
        assert_eq!(offset_of!(LinuxBusType, bus_groups), 16);
        assert_eq!(offset_of!(LinuxBusType, dev_groups), 24);
        assert_eq!(offset_of!(LinuxBusType, drv_groups), 32);
        assert_eq!(offset_of!(LinuxBusType, match_fn), 40);
        assert_eq!(offset_of!(LinuxBusType, uevent), 48);
        assert_eq!(offset_of!(LinuxBusType, probe), 56);
        assert_eq!(offset_of!(LinuxBusType, sync_state), 64);
        assert_eq!(offset_of!(LinuxBusType, remove), 72);
        assert_eq!(offset_of!(LinuxBusType, shutdown), 80);
        assert_eq!(offset_of!(LinuxBusType, irq_get_affinity), 88);
        assert_eq!(offset_of!(LinuxBusType, online), 96);
        assert_eq!(offset_of!(LinuxBusType, offline), 104);
        assert_eq!(offset_of!(LinuxBusType, suspend), 112);
        assert_eq!(offset_of!(LinuxBusType, resume), 120);
        assert_eq!(offset_of!(LinuxBusType, num_vf), 128);
        assert_eq!(offset_of!(LinuxBusType, dma_configure), 136);
        assert_eq!(offset_of!(LinuxBusType, dma_cleanup), 144);
        assert_eq!(offset_of!(LinuxBusType, pm), 152);
        assert_eq!(offset_of!(LinuxBusType, driver_override), 160);
        assert_eq!(offset_of!(LinuxBusType, need_parent_lock), 161);
        assert_eq!(size_of::<LinuxBusType>(), 168);
    }

    #[test]
    fn linux_bus_type_registry_tracks_raw_c_bus_pointers() {
        let mut bus = unsafe { core::mem::zeroed::<LinuxBusType>() };
        let name = b"bus-test-raw\0";
        bus.name = name.as_ptr().cast::<c_char>();
        let bus = &bus as *const LinuxBusType;

        unregister_linux_bus_type(bus);
        assert!(!linux_bus_type_registered(bus));
        register_linux_bus_type(bus);
        register_linux_bus_type(bus);
        assert!(linux_bus_type_registered(bus));
        unregister_linux_bus_type(bus);
        assert!(!linux_bus_type_registered(bus));
    }
}
