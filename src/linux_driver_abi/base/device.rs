//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base
//! test-origin: linux:vendor/linux/drivers/base
//! `struct device` — `vendor/linux/include/linux/device.h:611`.
//!
//! Lupos `Device` carries the same shape and lifecycle Linux exposes:
//!   * `name`, `init_name`     — sysfs name and seed parent.
//!   * `parent`                — `Option<Arc<Device>>`, mirrors `dev->parent`.
//!   * `bus`                   — `Option<Arc<BusType>>`; bus dispatches probe.
//!   * `driver`                — `Option<Arc<DeviceDriver>>`; bound after probe.
//!   * `class`                 — `Option<Arc<Class>>`; appears under `/sys/class`.
//!   * `kobj`                  — kobject for sysfs.
//!   * `driver_data`           — per-driver opaque (atomic `usize`).
//!   * `compatible`            — match string (Linux uses OF/ACPI tables; we
//!                                accept a single compatible string for now).
//!
//! Lifecycle:
//!   `device_initialize` (in `Device::new`) → `device_add` →
//!   `bus_probe_device` → `driver_probe_device` →
//!   on unload: `device_del` → `device_unregister`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::kobject::{KObject, kobject_add};
use crate::linux_driver_abi::base::bus::{
    BusType, LinuxBusType, bus_probe_device, linux_bus_type_registered,
};
use crate::linux_driver_abi::base::class::Class;
use crate::linux_driver_abi::base::driver::{DeviceDriver, LinuxDeviceDriver};

pub struct Device {
    pub name: String,
    pub compatible: Mutex<Option<String>>,
    pub parent: Mutex<Option<Arc<Device>>>,
    pub bus: Mutex<Option<Arc<BusType>>>,
    pub driver: Mutex<Option<Arc<DeviceDriver>>>,
    pub class: Mutex<Option<Arc<Class>>>,
    pub kobj: Arc<KObject>,
    pub driver_data: AtomicUsize,
    pub state: Mutex<DeviceState>,
    me: Mutex<Weak<Device>>,
}

/// `struct list_head` — `vendor/linux/include/linux/types.h:204`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxListHead {
    pub next: *mut c_void,
    pub prev: *mut c_void,
}

/// Prefix of `struct kobject` — `vendor/linux/include/linux/kobject.h`.
#[repr(C)]
pub struct LinuxKObject {
    pub name: *const c_char,
    pub entry: LinuxListHead,
    pub parent: *mut LinuxKObject,
    pub kset: *mut c_void,
    pub ktype: *const c_void,
    pub sd: *mut c_void,
    pub kref: i32,
    pub state_flags: u32,
}

/// Prefix of `struct device` through `driver_data`.
///
/// Source: `vendor/linux/include/linux/device.h:628`.  Later fields are not
/// modeled yet; this prefix is enough for driver-core registration, bus
/// matching, `dev_name()`, and `dev_{get,set}_drvdata()`.
#[repr(C)]
pub struct LinuxDevice {
    pub kobj: LinuxKObject,
    pub parent: *mut LinuxDevice,
    pub p: *mut c_void,
    pub init_name: *const c_char,
    pub type_: *const c_void,
    pub bus: *const LinuxBusType,
    pub driver: *mut LinuxDeviceDriver,
    pub platform_data: *mut c_void,
    pub driver_data: *mut c_void,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxDeviceRegistration {
    pub device: usize,
    pub bus: usize,
    pub driver: usize,
    pub name: usize,
}

struct LinuxDevicePrivate {
    device: usize,
    bus: usize,
    driver: usize,
    name: [u8; 64],
}

const KOBJ_STATE_INITIALIZED: u32 = 1 << 0;
const KOBJ_STATE_IN_SYSFS: u32 = 1 << 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceState {
    Initialized,
    Added,
    Bound,
    Unbound,
    Removed,
}

impl Device {
    pub fn new(name: &str) -> Arc<Self> {
        let kobj = KObject::new(name, None);
        let dev = Arc::new(Self {
            name: String::from(name),
            compatible: Mutex::new(None),
            parent: Mutex::new(None),
            bus: Mutex::new(None),
            driver: Mutex::new(None),
            class: Mutex::new(None),
            kobj,
            driver_data: AtomicUsize::new(0),
            state: Mutex::new(DeviceState::Initialized),
            me: Mutex::new(Weak::new()),
        });
        *dev.me.lock() = Arc::downgrade(&dev);
        dev
    }

    /// Linux equivalent: `dev_set_drvdata(dev, p)`.
    pub fn set_drvdata(&self, ptr: usize) {
        self.driver_data.store(ptr, Ordering::Release);
    }
    /// Linux equivalent: `dev_get_drvdata(dev)`.
    pub fn get_drvdata(&self) -> usize {
        self.driver_data.load(Ordering::Acquire)
    }

    pub fn this(&self) -> Arc<Device> {
        self.me.lock().upgrade().expect("Device::this after drop")
    }
}

// ── Registry — mirrors `devices_kset` plus class/bus listings ────────────────

lazy_static! {
    /// Every registered device, keyed by name.  Linux indexes via the kobject
    /// glue; the key is the unique device name (e.g. `synthetic.0`,
    /// `0000:00:01.0`).
    pub(crate) static ref DEVICES: Mutex<BTreeMap<String, Arc<Device>>> =
        Mutex::new(BTreeMap::new());
    static ref LINUX_DEVICES: Mutex<Vec<LinuxDeviceRegistration>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("device_initialize", linux_device_initialize as usize, true);
    export_symbol_once("device_add", linux_device_add as usize, true);
    export_symbol_once("device_register", linux_device_register as usize, true);
    export_symbol_once("device_unregister", linux_device_unregister as usize, true);
    export_symbol_once("get_device", get_device as usize, true);
    export_symbol_once("put_device", put_device as usize, true);
}

pub fn registered_linux_device_count() -> usize {
    LINUX_DEVICES.lock().len()
}

pub fn linux_device_registered(dev: *const LinuxDevice) -> bool {
    LINUX_DEVICES
        .lock()
        .iter()
        .any(|registered| registered.device == dev as usize)
}

pub fn linux_device_driver(dev: *const LinuxDevice) -> *mut LinuxDeviceDriver {
    LINUX_DEVICES
        .lock()
        .iter()
        .find(|registered| registered.device == dev as usize)
        .map(|registered| registered.driver as *mut LinuxDeviceDriver)
        .unwrap_or(core::ptr::null_mut())
}

fn record_linux_device_driver(dev: *mut LinuxDevice, driver: *mut LinuxDeviceDriver) {
    let dev_addr = dev as usize;
    let driver_addr = driver as usize;
    if let Some(registered) = LINUX_DEVICES
        .lock()
        .iter_mut()
        .find(|registered| registered.device == dev_addr)
    {
        registered.driver = driver_addr;
    }
    unsafe {
        if !(*dev).p.is_null() {
            (*(*dev).p.cast::<LinuxDevicePrivate>()).driver = driver_addr;
        }
    }
}

unsafe fn linux_device_probe_driver(dev: *mut LinuxDevice, driver: *mut LinuxDeviceDriver) -> bool {
    if dev.is_null() || driver.is_null() {
        return false;
    }

    let bus = unsafe { (*dev).bus };
    if bus.is_null() || unsafe { (*driver).bus } != bus {
        return false;
    }
    if unsafe { !(*dev).driver.is_null() } {
        return false;
    }

    let matches = unsafe {
        (*bus)
            .match_fn
            .map(|match_fn| match_fn(dev.cast(), driver.cast_const().cast()))
            .unwrap_or(1)
    };
    if matches <= 0 {
        return false;
    }

    unsafe {
        (*dev).driver = driver;
    }
    let probe_ret = unsafe {
        if let Some(probe) = (*bus).probe {
            probe(dev.cast())
        } else if let Some(probe) = (*driver).probe {
            probe(dev.cast())
        } else {
            0
        }
    };
    if probe_ret == 0 {
        true
    } else {
        unsafe {
            (*dev).driver = core::ptr::null_mut();
        }
        false
    }
}

/// Attach a newly registered raw C driver to devices already present on its bus.
///
/// Linux performs this through `driver_attach()`/`__driver_attach()` after
/// `driver_register()` (`vendor/linux/drivers/base/driver.c` and
/// `vendor/linux/drivers/base/dd.c`). This helper keeps the raw C ABI path
/// symmetrical with `device_add()`: all matching and probing still goes through
/// the bus and driver callbacks supplied by Linux-built code.
pub unsafe fn linux_driver_attach_existing(driver: *mut LinuxDeviceDriver) -> usize {
    if driver.is_null() {
        return 0;
    }
    let bus = unsafe { (*driver).bus };
    if bus.is_null() {
        return 0;
    }

    let devices: Vec<*mut LinuxDevice> = LINUX_DEVICES
        .lock()
        .iter()
        .filter(|registered| registered.bus == bus as usize && registered.driver == 0)
        .map(|registered| registered.device as *mut LinuxDevice)
        .collect();

    let mut attached = 0usize;
    for dev in devices {
        if unsafe { linux_device_probe_driver(dev, driver) } {
            record_linux_device_driver(dev, driver);
            attached += 1;
        }
    }
    attached
}

fn linux_dev_name(dev: &LinuxDevice) -> *const c_char {
    if !dev.init_name.is_null() {
        dev.init_name
    } else {
        dev.kobj.name
    }
}

unsafe fn ensure_linux_device_private(dev: *mut LinuxDevice) {
    if unsafe { (*dev).p.is_null() } {
        let private = Box::into_raw(Box::new(LinuxDevicePrivate {
            device: dev as usize,
            bus: 0,
            driver: 0,
            name: [0; 64],
        }));
        unsafe {
            (*dev).p = private.cast();
        }
    }
}

pub unsafe fn linux_device_set_name_index(
    dev: *mut LinuxDevice,
    prefix: &[u8],
    mut index: i32,
) -> Result<(), i32> {
    if dev.is_null() || index < 0 {
        return Err(EINVAL);
    }
    unsafe {
        ensure_linux_device_private(dev);
        let private = (*dev).p.cast::<LinuxDevicePrivate>();
        let name = &mut (*private).name;
        if prefix.len() + 11 >= name.len() {
            return Err(EINVAL);
        }

        let mut pos = 0usize;
        for byte in prefix.iter().copied() {
            name[pos] = byte;
            pos += 1;
        }

        let mut digits = [0u8; 10];
        let mut len = 0usize;
        if index == 0 {
            digits[0] = b'0';
            len = 1;
        } else {
            while index > 0 {
                digits[len] = b'0' + (index % 10) as u8;
                index /= 10;
                len += 1;
            }
        }
        while len > 0 {
            len -= 1;
            name[pos] = digits[len];
            pos += 1;
        }
        name[pos] = 0;
        (*dev).kobj.name = name.as_ptr().cast::<c_char>();
        (*dev).init_name = core::ptr::null();
    }
    Ok(())
}

pub unsafe fn linux_device_set_name_bytes(dev: *mut LinuxDevice, bytes: &[u8]) -> Result<(), i32> {
    if dev.is_null() {
        return Err(EINVAL);
    }
    unsafe {
        ensure_linux_device_private(dev);
        let private = (*dev).p.cast::<LinuxDevicePrivate>();
        let name = &mut (*private).name;
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if len == 0 || len >= name.len() {
            return Err(EINVAL);
        }

        core::ptr::write_bytes(name.as_mut_ptr(), 0, name.len());
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), name.as_mut_ptr(), len);
        (*dev).kobj.name = name.as_ptr().cast::<c_char>();
        (*dev).init_name = core::ptr::null();
    }
    Ok(())
}

unsafe fn free_linux_device_private(dev: *mut LinuxDevice) {
    let private = unsafe { (*dev).p };
    if !private.is_null() {
        unsafe {
            let _ = Box::from_raw(private.cast::<LinuxDevicePrivate>());
            (*dev).p = core::ptr::null_mut();
        }
    }
}

/// `device_initialize` — `vendor/linux/drivers/base/core.c`.
#[unsafe(export_name = "device_initialize")]
pub unsafe extern "C" fn linux_device_initialize(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    unsafe {
        ensure_linux_device_private(dev);
        (*dev).kobj.state_flags |= KOBJ_STATE_INITIALIZED;
        if (*dev).kobj.kref == 0 {
            (*dev).kobj.kref = 1;
        }
    }
}

/// `device_add` — `vendor/linux/drivers/base/core.c:3573`.
///
/// This raw C ABI path registers the device on its raw bus and attempts
/// Linux-style bus matching. Probe callbacks are called only from raw Linux
/// bus/driver callbacks; no local Rust device driver behavior is synthesized.
#[unsafe(export_name = "device_add")]
pub unsafe extern "C" fn linux_device_add(dev: *mut LinuxDevice) -> i32 {
    if dev.is_null() {
        crate::log_warn!("device", "device_add: null device");
        return -EINVAL;
    }

    unsafe {
        if (*dev).p.is_null() {
            ensure_linux_device_private(dev);
        }
    }

    let bus = unsafe { (*dev).bus };
    if !bus.is_null() && !linux_bus_type_registered(bus) {
        crate::log_warn!(
            "device",
            "device_add: unregistered bus dev={:p} bus={:p}",
            dev,
            bus
        );
        return -EINVAL;
    }

    let name = unsafe { linux_dev_name(&*dev) };
    if name.is_null() {
        crate::log_warn!("device", "device_add: unnamed device {:p}", dev);
        return -EINVAL;
    }

    let dev_addr = dev as usize;
    {
        let devices = LINUX_DEVICES.lock();
        if devices
            .iter()
            .any(|registered| registered.device == dev_addr)
        {
            crate::log_warn!("device", "device_add: duplicate device {:p}", dev);
            return -EBUSY;
        }
    }

    let mut bound_driver = core::ptr::null_mut();
    if !bus.is_null() {
        for driver in crate::linux_driver_abi::base::linux_device_drivers_on_bus(bus) {
            if unsafe { linux_device_probe_driver(dev, driver) } {
                bound_driver = driver;
                break;
            }
        }
    }

    unsafe {
        if !(*dev).init_name.is_null() && (*dev).kobj.name.is_null() {
            (*dev).kobj.name = (*dev).init_name;
        }
        (*dev).init_name = core::ptr::null();
        (*dev).kobj.state_flags |= KOBJ_STATE_IN_SYSFS;
        if !(*dev).p.is_null() {
            let private = (*dev).p.cast::<LinuxDevicePrivate>();
            (*private).bus = bus as usize;
            (*private).driver = bound_driver as usize;
        }
    }
    let name = unsafe { linux_dev_name(&*dev) };

    let mut devices = LINUX_DEVICES.lock();
    if devices
        .iter()
        .any(|registered| registered.device == dev_addr)
    {
        crate::log_warn!("device", "device_add: duplicate device {:p}", dev);
        return -EBUSY;
    }
    devices.push(LinuxDeviceRegistration {
        device: dev_addr,
        bus: bus as usize,
        driver: bound_driver as usize,
        name: name as usize,
    });

    0
}

/// `device_register` — `vendor/linux/drivers/base/core.c:3795`.
#[unsafe(export_name = "device_register")]
pub unsafe extern "C" fn linux_device_register(dev: *mut LinuxDevice) -> i32 {
    unsafe {
        linux_device_initialize(dev);
        linux_device_add(dev)
    }
}

/// `device_unregister` — `vendor/linux/drivers/base/core.c:3918`.
#[unsafe(export_name = "device_unregister")]
pub unsafe extern "C" fn linux_device_unregister(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    LINUX_DEVICES
        .lock()
        .retain(|registered| registered.device != dev as usize);
    unsafe {
        (*dev).driver = core::ptr::null_mut();
        (*dev).kobj.state_flags &= !KOBJ_STATE_IN_SYSFS;
        free_linux_device_private(dev);
    }
}

/// `get_device` - `vendor/linux/drivers/base/core.c:3800`.
#[unsafe(export_name = "get_device")]
pub unsafe extern "C" fn get_device(dev: *mut LinuxDevice) -> *mut LinuxDevice {
    if dev.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        (*dev).kobj.kref = (*dev).kobj.kref.saturating_add(1);
    }
    dev
}

/// `put_device` - `vendor/linux/drivers/base/core.c:3810`.
#[unsafe(export_name = "put_device")]
pub unsafe extern "C" fn put_device(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    unsafe {
        if (*dev).kobj.kref > 0 {
            (*dev).kobj.kref -= 1;
        }
    }
}

/// `device_add` — `drivers/base/core.c:3573`.
///
/// Registers `dev` under its bus and class (if set), creates the sysfs
/// kobject, and dispatches to the driver model probe path.
pub fn device_add(dev: Arc<Device>) -> Result<(), i32> {
    {
        let mut s = dev.state.lock();
        if *s != DeviceState::Initialized && *s != DeviceState::Removed {
            return Err(EBUSY);
        }
        *s = DeviceState::Added;
    }

    if dev.name.is_empty() {
        return Err(EINVAL);
    }

    DEVICES.lock().insert(dev.name.clone(), dev.clone());
    let _ = kobject_add(dev.kobj.clone());

    if let Some(bus) = dev.bus.lock().clone() {
        bus.devices.lock().push(dev.clone());
        bus_probe_device(&bus, &dev);
    }
    if let Some(class) = dev.class.lock().clone() {
        class.devices.lock().push(dev.clone());
    }
    Ok(())
}

/// `device_register` — `drivers/base/core.c:3770`.
///
/// In Linux this is `device_initialize() + device_add()`.  Our `Device::new`
/// already initializes, so this is just `device_add`.
pub fn device_register(dev: Arc<Device>) -> Result<(), i32> {
    device_add(dev)
}

/// `device_del` — `drivers/base/core.c:3834`.
pub fn device_del(dev: &Arc<Device>) -> Result<(), i32> {
    {
        let s = *dev.state.lock();
        if s == DeviceState::Removed {
            return Ok(());
        }
    }

    // If bound, call driver->remove first.
    let bound_driver = { dev.driver.lock().clone() };
    if let Some(drv) = bound_driver {
        if let Some(remove) = drv.remove {
            remove(dev);
        }
        if let Some(bus) = { dev.bus.lock().clone() } {
            bus.devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
        }
        drv.bound_devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
        *dev.driver.lock() = None;
    }
    if let Some(class) = { dev.class.lock().clone() } {
        class.devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
    }

    DEVICES.lock().remove(&dev.name);
    *dev.state.lock() = DeviceState::Removed;
    Ok(())
}

/// `device_unregister` — `drivers/base/core.c:3918`.
pub fn device_unregister(dev: &Arc<Device>) -> Result<(), i32> {
    device_del(dev)
}

/// Diagnostic — count of currently registered devices.
pub fn registered_devices() -> Vec<String> {
    DEVICES.lock().keys().cloned().collect()
}

/// Look up a registered device by name.  Used by the platform-bus tests.
pub fn find_device(name: &str) -> Option<Arc<Device>> {
    DEVICES.lock().get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::base::{
        LinuxDeviceDriver, linux_driver_register, linux_driver_unregister, register_linux_bus_type,
        unregister_linux_bus_type,
    };
    use core::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn linux_device_c_layout_prefix_matches_vendor_header() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxKObject, name), 0);
        assert_eq!(offset_of!(LinuxKObject, entry), 8);
        assert_eq!(offset_of!(LinuxKObject, parent), 24);
        assert_eq!(offset_of!(LinuxKObject, kset), 32);
        assert_eq!(offset_of!(LinuxKObject, ktype), 40);
        assert_eq!(offset_of!(LinuxKObject, sd), 48);
        assert_eq!(offset_of!(LinuxKObject, kref), 56);
        assert_eq!(offset_of!(LinuxKObject, state_flags), 60);
        assert_eq!(size_of::<LinuxKObject>(), 64);

        assert_eq!(offset_of!(LinuxDevice, kobj), 0);
        assert_eq!(offset_of!(LinuxDevice, parent), 64);
        assert_eq!(offset_of!(LinuxDevice, p), 72);
        assert_eq!(offset_of!(LinuxDevice, init_name), 80);
        assert_eq!(offset_of!(LinuxDevice, type_), 88);
        assert_eq!(offset_of!(LinuxDevice, bus), 96);
        assert_eq!(offset_of!(LinuxDevice, driver), 104);
        assert_eq!(offset_of!(LinuxDevice, platform_data), 112);
        assert_eq!(offset_of!(LinuxDevice, driver_data), 120);
        assert_eq!(size_of::<LinuxDevice>(), 128);
    }

    #[test]
    fn linux_device_core_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("device_initialize"),
            Some(linux_device_initialize as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_add"),
            Some(linux_device_add as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_register"),
            Some(linux_device_register as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_unregister"),
            Some(linux_device_unregister as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("get_device"),
            Some(get_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("put_device"),
            Some(put_device as usize)
        );
    }

    #[test]
    fn linux_get_put_device_update_kobject_refcount() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();
            assert!(get_device(core::ptr::null_mut()).is_null());
            put_device(core::ptr::null_mut());

            linux_device_initialize(&mut dev);
            assert_eq!(dev.kobj.kref, 1);
            assert_eq!(get_device(&mut dev), &mut dev as *mut _);
            assert_eq!(dev.kobj.kref, 2);
            put_device(&mut dev);
            assert_eq!(dev.kobj.kref, 1);
            put_device(&mut dev);
            assert_eq!(dev.kobj.kref, 0);
            put_device(&mut dev);
            assert_eq!(dev.kobj.kref, 0);
            free_linux_device_private(&mut dev);
        }
    }

    #[test]
    fn linux_device_register_tracks_raw_device_and_dispatches_raw_probe() {
        static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn always_match(_dev: *mut c_void, _drv: *const c_void) -> i32 {
            1
        }

        unsafe extern "C" fn probe(_dev: *mut c_void) -> i32 {
            PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
            0
        }

        unsafe {
            assert_eq!(linux_device_add(core::ptr::null_mut()), -EINVAL);

            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"device-bus-test\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            bus.match_fn = Some(always_match);
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);
            register_linux_bus_type(bus_ptr);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"device-driver-test\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            driver.probe = Some(probe);
            assert_eq!(linux_driver_register(&mut driver), 0);

            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"device0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            dev.bus = bus_ptr;

            let before = registered_linux_device_count();
            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);
            assert_eq!(dev.driver, &mut driver as *mut _);
            assert_eq!(dev.kobj.name, dev_name.as_ptr().cast::<c_char>());
            assert!(dev.init_name.is_null());
            assert!(dev.kobj.state_flags & KOBJ_STATE_INITIALIZED != 0);
            assert!(dev.kobj.state_flags & KOBJ_STATE_IN_SYSFS != 0);
            assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1);
            assert_eq!(registered_linux_device_count(), before + 1);
            assert_eq!(linux_device_add(&mut dev), -EBUSY);

            linux_device_unregister(&mut dev);
            assert!(dev.p.is_null());
            assert!(dev.driver.is_null());
            assert!(!linux_device_registered(&dev));
            assert_eq!(registered_linux_device_count(), before);

            linux_driver_unregister(&mut driver);
            unregister_linux_bus_type(bus_ptr);
        }
    }

    #[test]
    fn linux_device_add_allows_probe_to_rename_device() {
        unsafe extern "C" fn always_match(_dev: *mut c_void, _drv: *const c_void) -> i32 {
            1
        }

        unsafe extern "C" fn probe_renames_device(dev: *mut c_void) -> i32 {
            unsafe {
                linux_device_set_name_bytes(dev.cast::<LinuxDevice>(), b"renamed0\0")
                    .map(|()| 0)
                    .unwrap_or_else(|errno| -errno)
            }
        }

        unsafe {
            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"device-rename-bus\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            bus.match_fn = Some(always_match);
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);
            register_linux_bus_type(bus_ptr);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"device-rename-driver\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            driver.probe = Some(probe_renames_device);
            assert_eq!(linux_driver_register(&mut driver), 0);

            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"rename-source0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            dev.bus = bus_ptr;

            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);
            let name = core::slice::from_raw_parts(dev.kobj.name.cast::<u8>(), 9);
            assert_eq!(name, b"renamed0\0");

            linux_device_unregister(&mut dev);
            linux_driver_unregister(&mut driver);
            unregister_linux_bus_type(bus_ptr);
        }
    }

    #[test]
    fn linux_device_add_accepts_named_busless_devices() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"class-device0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();

            let before = registered_linux_device_count();
            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert!(linux_device_driver(&dev).is_null());
            assert!(dev.driver.is_null());
            assert_eq!(dev.kobj.name, dev_name.as_ptr().cast::<c_char>());
            assert!(dev.init_name.is_null());
            assert_eq!(registered_linux_device_count(), before + 1);
            assert_eq!(linux_device_add(&mut dev), -EBUSY);

            linux_device_unregister(&mut dev);
            assert!(!linux_device_registered(&dev));
            assert_eq!(registered_linux_device_count(), before);
        }
    }

    #[test]
    fn linux_device_set_name_bytes_sets_exact_name() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();

            linux_device_initialize(&mut dev);
            assert_eq!(linux_device_set_name_bytes(&mut dev, b"host7\0"), Ok(()));
            assert_eq!(dev.kobj.name, linux_dev_name(&dev));
            let name = core::slice::from_raw_parts(dev.kobj.name.cast::<u8>(), 6);
            assert_eq!(name, b"host7\0");
            assert!(dev.init_name.is_null());

            free_linux_device_private(&mut dev);
        }
    }
}
