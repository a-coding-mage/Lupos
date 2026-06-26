//! linux-parity: complete
//! linux-source: vendor/linux/drivers/base/platform.c
//! test-origin: linux:vendor/linux/drivers/base/platform.c
//! `platform_bus_type` — `vendor/linux/drivers/base/platform.c`.
//!
//! The platform bus is the synthetic bus Linux uses for non-discoverable
//! devices: SoC peripherals, ACPI platform devices, board-file devices.
//! Match is by `compatible` string equality, mirroring the OF/ACPI match
//! tables Linux walks at `platform_match`.
//!
//! M54 uses the platform bus as the acceptance fixture: register a
//! `synthetic_driver` and a `synthetic_device` with the same compatible
//! string and verify probe runs.

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;

use lazy_static::lazy_static;

use crate::linux_driver_abi::base::bus::{BusType, bus_register};
use crate::linux_driver_abi::base::device::{Device, device_register};
use crate::linux_driver_abi::base::driver::{DeviceDriver, ProbeFn, RemoveFn, driver_register};

/// Linux match: `platform_match` — compares OF compatible / ACPI _HID /
/// `platform_device_id` table.  We collapse to a compatible string compare.
fn platform_match(dev: &Arc<Device>, drv: &Arc<DeviceDriver>) -> bool {
    let g = dev.compatible.lock();
    match (g.as_deref(), drv.compatible) {
        (Some(d), Some(k)) => d == k,
        _ => false,
    }
}

lazy_static! {
    pub static ref PLATFORM_BUS: Arc<BusType> = {
        let bus = BusType::new("platform", platform_match);
        let _ = bus_register(bus.clone());
        bus
    };
}

/// Thin wrapper for documentation parity with Linux types.
pub struct PlatformDevice;
pub struct PlatformDriver;

/// `platform_device_register` — `drivers/base/platform.c`.
///
/// `name` is the sysfs name (e.g. `"synthetic.0"`); `compatible` is the
/// match string consumed by `platform_match`.
pub fn platform_device_register(name: &str, compatible: &'static str) -> Result<Arc<Device>, i32> {
    let dev = Device::new(name);
    *dev.compatible.lock() = Some(String::from(compatible));
    *dev.bus.lock() = Some(PLATFORM_BUS.clone());
    device_register(dev.clone())?;
    Ok(dev)
}

/// `platform_driver_register` — `drivers/base/platform.c`.
pub fn platform_driver_register(
    name: &'static str,
    compatible: &'static str,
    probe: Option<ProbeFn>,
    remove: Option<RemoveFn>,
) -> Result<Arc<DeviceDriver>, i32> {
    let drv = DeviceDriver::new(name, Some(compatible), probe, remove);
    *drv.bus.lock() = Some(PLATFORM_BUS.clone());
    driver_register(drv.clone())?;
    Ok(drv)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::boxed::Box;
    use core::sync::atomic::{AtomicU32, Ordering};

    use crate::linux_driver_abi::base::{device_unregister, driver_unregister, find_device};

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);
    static REMOVE_COUNT: AtomicU32 = AtomicU32::new(0);

    fn synth_probe(_dev: &Arc<Device>) -> Result<(), i32> {
        PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn synth_remove(_dev: &Arc<Device>) {
        REMOVE_COUNT.fetch_add(1, Ordering::AcqRel);
    }

    #[test]
    fn platform_device_unregister_unbinds_cleanly() {
        let _guard = TEST_LOCK.lock().unwrap();
        PROBE_COUNT.store(0, Ordering::Release);
        REMOVE_COUNT.store(0, Ordering::Release);

        let id = NEXT_ID.fetch_add(1, Ordering::AcqRel);
        let dev_name = std::format!("synthetic.{id}");
        let driver_name = Box::leak(std::format!("synth-drv-{id}").into_boxed_str());
        let compatible = Box::leak(std::format!("lupos,synthetic-{id}").into_boxed_str());

        let drv = platform_driver_register(
            driver_name,
            compatible,
            Some(synth_probe),
            Some(synth_remove),
        )
        .expect("platform_driver_register");
        let dev =
            platform_device_register(&dev_name, compatible).expect("platform_device_register");

        assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1, "probe count");
        assert!(dev.driver.lock().is_some(), "device should be bound");
        assert!(find_device(&dev_name).is_some(), "registry");

        device_unregister(&dev).expect("device_unregister");

        assert_eq!(REMOVE_COUNT.load(Ordering::Acquire), 1, "remove count");
        assert!(find_device(&dev_name).is_none(), "unregistered");
        assert!(
            drv.bound_devices.lock().is_empty(),
            "bound device list drained"
        );
        assert!(
            PLATFORM_BUS
                .devices
                .lock()
                .iter()
                .all(|registered| registered.name != dev_name),
            "bus device list drained"
        );

        driver_unregister(&drv);
        assert!(
            PLATFORM_BUS
                .drivers
                .lock()
                .iter()
                .all(|registered| !Arc::ptr_eq(registered, &drv)),
            "driver removed from platform bus"
        );
    }
}
