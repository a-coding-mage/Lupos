//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base
//! Linux device model — M54.
//!
//! Mirrors `vendor/linux/drivers/base/` and `vendor/linux/include/linux/device.h`.
//! Provides the `Device` / `BusType` / `DeviceDriver` / `Class` quartet, the
//! `device_register` / `driver_register` / `bus_register` / `class_register`
//! entry points, and the probe/match dispatch (`__driver_attach`,
//! `driver_probe_device`).
//!
//! Sysfs surface (mirrors Linux's `/sys` layout):
//!   * `bus_register("foo")`   →  `/sys/bus/foo/{devices,drivers}`
//!   * `class_register("bar")` →  `/sys/class/bar/`
//!   * `device_add(dev)`       →  `/sys/devices/<bus>/<dev_name>` and a
//!                                symlink under `/sys/bus/<bus>/devices/<dev_name>`.
//!
//! References:
//!   - `include/linux/device.h:611` — `struct device`
//!   - `include/linux/device/bus.h:83` — `struct bus_type`
//!   - `include/linux/device/driver.h:98` — `struct device_driver`
//!   - `drivers/base/core.c:3573` — `device_add`
//!   - `drivers/base/driver.c:225` — `driver_register`
//!   - `drivers/base/dd.c:1215` — `__driver_attach`

pub mod bus;
pub mod class;
pub mod device;
pub mod driver;
pub mod linux_sources;
pub mod platform;
pub(crate) mod printf;

pub use bus::{
    BusType, LinuxBusType, bus_register, bus_unregister, linux_bus_type_registered,
    register_linux_bus_type, registered_buses, unregister_linux_bus_type,
};
pub use class::{Class, class_register, registered_classes};
pub use device::{
    Device, LinuxDevice, LinuxKObject, LinuxListHead, device_add, device_del, device_register,
    device_unregister, find_device, get_device, linux_device_add, linux_device_driver,
    linux_device_initialize, linux_device_register, linux_device_registered,
    linux_device_set_name_bytes, linux_device_set_name_index, linux_device_unregister, put_device,
    register_module_exports as register_device_module_exports, registered_linux_device_count,
};
pub use driver::{
    DeviceDriver, LinuxDeviceDriver, LinuxDeviceDriverPrivateCallbacks, ProbeFn, RemoveFn,
    driver_register, driver_unregister, linux_device_driver_registered,
    linux_device_drivers_on_bus, linux_driver_register, linux_driver_unregister,
    register_module_exports as register_driver_module_exports,
    registered_linux_device_driver_count,
};
pub use platform::{
    PLATFORM_BUS, PlatformDevice, PlatformDriver, platform_device_register,
    platform_driver_register,
};

pub fn register_module_exports() {
    device::register_module_exports();
    driver::register_module_exports();
}
