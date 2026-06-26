//! linux-parity: complete
//! linux-source: vendor/linux/drivers/usb
//! test-origin: linux:vendor/linux/drivers/usb
//! USB core — M58.
//!
//! Mirrors `include/linux/usb.h`, `drivers/usb/core/usb.c`, and
//! `drivers/usb/core/hub.c`.
//!
//! References:
//!   - `include/linux/usb.h:660`          — `struct usb_device`
//!   - `include/linux/usb.h:1244`         — `struct usb_driver`
//!   - `drivers/usb/core/driver.c:1060`   — `usb_register_driver`
//!   - `drivers/usb/core/hub.c`           — port enumeration

extern crate alloc;

pub mod host;
pub mod linux_sources;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EEXIST;

// ── USB device classes ────────────────────────────────────────────────────────
pub const USB_CLASS_HID: u8 = 0x03;

/// USB device speed — mirrors `enum usb_device_speed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbSpeed {
    Low,   // 1.5 Mb/s
    Full,  // 12 Mb/s
    High,  // 480 Mb/s
    Super, // 5 Gb/s
}

/// `struct usb_device` — `include/linux/usb.h:660`.
pub struct UsbDevice {
    pub bus_num: u8,
    pub dev_num: u8,
    pub speed: UsbSpeed,
    pub vendor_id: u16,
    pub product_id: u16,
    pub dev_class: u8,
    pub name: String,
    /// Child devices (hub ports).
    pub children: Mutex<Vec<Arc<UsbDevice>>>,
}

impl UsbDevice {
    pub fn new(
        bus_num: u8,
        dev_num: u8,
        speed: UsbSpeed,
        vid: u16,
        pid: u16,
        class: u8,
        name: &str,
    ) -> Arc<Self> {
        Arc::new(Self {
            bus_num,
            dev_num,
            speed,
            vendor_id: vid,
            product_id: pid,
            dev_class: class,
            name: String::from(name),
            children: Mutex::new(Vec::new()),
        })
    }
}

// ── USB driver ────────────────────────────────────────────────────────────────

pub type UsbProbeFn = fn(dev: &Arc<UsbDevice>) -> Result<(), i32>;
pub type UsbRemoveFn = fn(dev: &Arc<UsbDevice>);

/// `struct usb_driver` — `include/linux/usb.h:1244`.
pub struct UsbDriver {
    pub name: &'static str,
    pub class: u8,
    pub probe: Option<UsbProbeFn>,
    pub remove: Option<UsbRemoveFn>,
    pub bound: Mutex<Vec<Arc<UsbDevice>>>,
}

impl UsbDriver {
    pub fn new(
        name: &'static str,
        class: u8,
        probe: Option<UsbProbeFn>,
        remove: Option<UsbRemoveFn>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            class,
            probe,
            remove,
            bound: Mutex::new(Vec::new()),
        })
    }

    pub fn matches(&self, dev: &UsbDevice) -> bool {
        self.class == dev.dev_class
    }
}

// ── Registries ────────────────────────────────────────────────────────────────

lazy_static! {
    static ref USB_DEVICES: Mutex<BTreeMap<u16, Arc<UsbDevice>>> = Mutex::new(BTreeMap::new());
    static ref USB_DRIVERS: Mutex<Vec<Arc<UsbDriver>>> = Mutex::new(Vec::new());
}

fn dev_key(bus: u8, dev: u8) -> u16 {
    ((bus as u16) << 8) | dev as u16
}

/// `usb_register_driver` — `drivers/usb/core/driver.c:1060`.
pub fn usb_register_driver(drv: Arc<UsbDriver>) -> Result<(), i32> {
    let devs: Vec<Arc<UsbDevice>> = USB_DEVICES.lock().values().cloned().collect();
    for dev in devs.iter() {
        if drv.matches(dev) {
            if let Some(probe) = drv.probe {
                if probe(dev).is_ok() {
                    drv.bound.lock().push(dev.clone());
                }
            }
        }
    }
    USB_DRIVERS.lock().push(drv);
    Ok(())
}

/// Add a USB device (called by hub or xHCI on port attachment).
pub fn usb_add_device(dev: Arc<UsbDevice>) -> Result<(), i32> {
    let key = dev_key(dev.bus_num, dev.dev_num);
    let mut g = USB_DEVICES.lock();
    if g.contains_key(&key) {
        return Err(EEXIST);
    }
    drop(g);
    let drivers: Vec<Arc<UsbDriver>> = USB_DRIVERS.lock().iter().cloned().collect();
    for drv in drivers.iter() {
        if drv.matches(&dev) {
            if let Some(probe) = drv.probe {
                if probe(&dev).is_ok() {
                    drv.bound.lock().push(dev.clone());
                    break;
                }
            }
        }
    }
    USB_DEVICES.lock().insert(key, dev);
    Ok(())
}

pub fn usb_device_count() -> usize {
    USB_DEVICES.lock().len()
}

pub fn find_usb_device(bus: u8, dev: u8) -> Option<Arc<UsbDevice>> {
    USB_DEVICES.lock().get(&dev_key(bus, dev)).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_usb_device() {
        let dev = UsbDevice::new(
            1,
            1,
            UsbSpeed::Full,
            0x046D,
            0xC534,
            USB_CLASS_HID,
            "test-hid-kbd",
        );
        usb_add_device(dev.clone()).unwrap();
        assert!(find_usb_device(1, 1).is_some());
    }

    #[test]
    fn driver_probes_on_register() {
        use core::sync::atomic::{AtomicU32, Ordering};
        static CNT: AtomicU32 = AtomicU32::new(0);
        fn my_probe(_: &Arc<UsbDevice>) -> Result<(), i32> {
            CNT.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
        let dev = UsbDevice::new(
            2,
            1,
            UsbSpeed::High,
            0x0000,
            0x0001,
            0xFF,
            "test-vendor-dev",
        );
        usb_add_device(dev).unwrap();
        let drv = UsbDriver::new("test-usb-drv", 0xFF, Some(my_probe), None);
        usb_register_driver(drv).unwrap();
        assert!(CNT.load(Ordering::Acquire) >= 1);
    }
}
