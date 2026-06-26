//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/driver.c
//! test-origin: linux:vendor/linux/drivers/base/driver.c
//! `struct device_driver` — `vendor/linux/include/linux/device/driver.h:98`.
//!
//! A driver carries a `name`, an owning `bus`, and a pair of probe/remove
//! callbacks that the bus dispatches.  `driver_register` adds the driver to
//! the bus's driver list and walks the bus's devices to bind any that match
//! (`__driver_attach`).

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::bus::{
    __driver_attach, BusType, LinuxBusType, linux_bus_type_registered,
};
use crate::linux_driver_abi::base::device::{Device, linux_driver_attach_existing};

pub type ProbeFn = fn(dev: &Arc<Device>) -> Result<(), i32>;
pub type RemoveFn = fn(dev: &Arc<Device>);

pub struct DeviceDriver {
    pub name: &'static str,
    pub compatible: Option<&'static str>,
    pub bus: Mutex<Option<Arc<BusType>>>,
    pub probe: Option<ProbeFn>,
    pub remove: Option<RemoveFn>,
    pub bound_devices: Mutex<Vec<Arc<Device>>>,
}

/// `struct device_driver` — `vendor/linux/include/linux/device/driver.h:98`.
#[repr(C)]
pub struct LinuxDeviceDriver {
    pub name: *const c_char,
    pub bus: *const LinuxBusType,
    pub owner: *mut c_void,
    pub mod_name: *const c_char,
    pub suppress_bind_attrs: bool,
    pub probe_type: i32,
    pub of_match_table: *const c_void,
    pub acpi_match_table: *const c_void,
    pub probe: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub sync_state: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub remove: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub shutdown: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub suspend: Option<unsafe extern "C" fn(dev: *mut c_void, state: usize) -> i32>,
    pub resume: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub groups: *const *const c_void,
    pub dev_groups: *const *const c_void,
    pub pm: *const c_void,
    pub coredump: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub p: *mut c_void,
    pub p_cb: LinuxDeviceDriverPrivateCallbacks,
}

#[repr(C)]
pub struct LinuxDeviceDriverPrivateCallbacks {
    pub post_unbind_rust: Option<unsafe extern "C" fn(dev: *mut c_void)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxDeviceDriverRegistration {
    pub driver: usize,
    pub bus: usize,
    pub name: usize,
}

struct LinuxDriverPrivate {
    driver: usize,
    bus: usize,
}

lazy_static! {
    static ref LINUX_DEVICE_DRIVERS: Mutex<Vec<LinuxDeviceDriverRegistration>> =
        Mutex::new(Vec::new());
}

impl DeviceDriver {
    pub fn new(
        name: &'static str,
        compatible: Option<&'static str>,
        probe: Option<ProbeFn>,
        remove: Option<RemoveFn>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            compatible,
            bus: Mutex::new(None),
            probe,
            remove,
            bound_devices: Mutex::new(Vec::new()),
        })
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("driver_register", linux_driver_register as usize, true);
    export_symbol_once("driver_unregister", linux_driver_unregister as usize, true);
}

pub fn registered_linux_device_driver_count() -> usize {
    LINUX_DEVICE_DRIVERS.lock().len()
}

pub fn linux_device_driver_registered(driver: *const LinuxDeviceDriver) -> bool {
    LINUX_DEVICE_DRIVERS
        .lock()
        .iter()
        .any(|registered| registered.driver == driver as usize)
}

pub fn linux_device_drivers_on_bus(bus: *const LinuxBusType) -> Vec<*mut LinuxDeviceDriver> {
    LINUX_DEVICE_DRIVERS
        .lock()
        .iter()
        .filter(|registered| registered.bus == bus as usize)
        .map(|registered| registered.driver as *mut LinuxDeviceDriver)
        .collect()
}

unsafe fn cstr_eq(mut a: *const c_char, mut b: *const c_char) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }

    loop {
        let ac = unsafe { core::ptr::read(a) };
        let bc = unsafe { core::ptr::read(b) };
        if ac != bc {
            return false;
        }
        if ac == 0 {
            return true;
        }
        a = unsafe { a.add(1) };
        b = unsafe { b.add(1) };
    }
}

/// `driver_register` — `vendor/linux/drivers/base/driver.c:225`.
///
/// This is the C/module ABI path for Linux-built drivers. It records the raw
/// `struct device_driver` on a registered raw `struct bus_type`, then mirrors
/// Linux's `driver_attach()` pass over devices already present on that bus.
#[unsafe(export_name = "driver_register")]
pub unsafe extern "C" fn linux_driver_register(driver: *mut LinuxDeviceDriver) -> i32 {
    if driver.is_null() {
        return -EINVAL;
    }

    let bus = unsafe { (*driver).bus };
    let name = unsafe { (*driver).name };
    if bus.is_null() || name.is_null() || !linux_bus_type_registered(bus) {
        return -EINVAL;
    }

    let mut drivers = LINUX_DEVICE_DRIVERS.lock();
    if drivers.iter().any(|registered| {
        registered.bus == bus as usize && unsafe { cstr_eq(registered.name as *const c_char, name) }
    }) {
        return -EBUSY;
    }

    let private = Box::into_raw(Box::new(LinuxDriverPrivate {
        driver: driver as usize,
        bus: bus as usize,
    }));
    unsafe {
        (*driver).p = private.cast();
    }
    drivers.push(LinuxDeviceDriverRegistration {
        driver: driver as usize,
        bus: bus as usize,
        name: name as usize,
    });
    drop(drivers);

    unsafe {
        linux_driver_attach_existing(driver);
    }

    0
}

/// `driver_unregister` — `vendor/linux/drivers/base/driver.c:257`.
#[unsafe(export_name = "driver_unregister")]
pub unsafe extern "C" fn linux_driver_unregister(driver: *mut LinuxDeviceDriver) {
    if driver.is_null() {
        return;
    }

    LINUX_DEVICE_DRIVERS
        .lock()
        .retain(|registered| registered.driver != driver as usize);

    let private = unsafe { (*driver).p };
    if !private.is_null() {
        let raw = private.cast::<LinuxDriverPrivate>();
        unsafe {
            let _ = Box::from_raw(raw);
            (*driver).p = core::ptr::null_mut();
        }
    }
}

/// `driver_register` — `drivers/base/driver.c:225`.
///
/// `drv.bus` must be set before calling.  After insertion into the bus's
/// driver list, walk the bus's devices and bind any that match.
pub fn driver_register(drv: Arc<DeviceDriver>) -> Result<(), i32> {
    let bus = drv.bus.lock().clone().ok_or(EINVAL)?;
    bus.drivers.lock().push(drv.clone());
    __driver_attach(&bus, &drv);
    Ok(())
}

/// `driver_unregister` — `drivers/base/driver.c`.
///
/// Calls `remove` on every bound device, drops the driver out of the bus's
/// driver list.
pub fn driver_unregister(drv: &Arc<DeviceDriver>) {
    if let Some(bus) = drv.bus.lock().clone() {
        bus.drivers.lock().retain(|d| !Arc::ptr_eq(d, drv));
    }
    let bound: Vec<Arc<Device>> = drv.bound_devices.lock().drain(..).collect();
    for dev in bound.iter() {
        if let Some(remove) = drv.remove {
            remove(dev);
        }
        *dev.driver.lock() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::base::bus::{register_linux_bus_type, unregister_linux_bus_type};
    use crate::linux_driver_abi::base::device::{
        linux_device_driver, linux_device_register, linux_device_unregister,
    };
    use core::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn linux_driver_core_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("driver_register"),
            Some(linux_driver_register as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("driver_unregister"),
            Some(linux_driver_unregister as usize)
        );
    }

    #[test]
    fn linux_driver_register_tracks_raw_c_driver_on_registered_bus() {
        unsafe {
            assert_eq!(linux_driver_register(core::ptr::null_mut()), -EINVAL);

            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"driver-bus-test\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"driver-core-test\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            assert_eq!(linux_driver_register(&mut driver), -EINVAL);

            register_linux_bus_type(bus_ptr);
            let before = registered_linux_device_driver_count();
            assert_eq!(linux_driver_register(&mut driver), 0);
            assert!(linux_device_driver_registered(&driver));
            assert!(!driver.p.is_null());
            assert_eq!(registered_linux_device_driver_count(), before + 1);

            let mut duplicate = core::mem::zeroed::<LinuxDeviceDriver>();
            duplicate.name = driver_name.as_ptr().cast::<c_char>();
            duplicate.bus = bus_ptr;
            assert_eq!(linux_driver_register(&mut duplicate), -EBUSY);
            assert!(duplicate.p.is_null());

            linux_driver_unregister(&mut driver);
            assert!(driver.p.is_null());
            assert!(!linux_device_driver_registered(&driver));
            assert_eq!(registered_linux_device_driver_count(), before);
            unregister_linux_bus_type(bus_ptr);
        }
    }

    #[test]
    fn linux_driver_register_attaches_existing_raw_devices() {
        static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn always_match(_dev: *mut c_void, _drv: *const c_void) -> i32 {
            1
        }

        unsafe extern "C" fn probe(_dev: *mut c_void) -> i32 {
            PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
            0
        }

        unsafe {
            PROBE_COUNT.store(0, Ordering::Release);

            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"driver-attach-bus-test\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            bus.match_fn = Some(always_match);
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);
            register_linux_bus_type(bus_ptr);

            let mut dev = core::mem::zeroed::<crate::linux_driver_abi::base::device::LinuxDevice>();
            let dev_name = b"driver-attach-device0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            dev.bus = bus_ptr;
            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_driver(&dev).is_null());

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"driver-attach-test\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            driver.probe = Some(probe);

            assert_eq!(linux_driver_register(&mut driver), 0);
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);
            assert_eq!(dev.driver, &mut driver as *mut _);
            assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1);

            linux_driver_unregister(&mut driver);
            linux_device_unregister(&mut dev);
            unregister_linux_bus_type(bus_ptr);
        }
    }
}
