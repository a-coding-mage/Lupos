//! linux-parity: stub
//! linux-source: vendor/linux/drivers/gpu/drm
//! test-origin: linux:vendor/linux/drivers/gpu/drm
//! DRM device/minor registry stub — M57.
//!
//! Provides a small `DrmDevice` and minor registry for existing acceptance
//! tests. It does not yet implement Linux's DRM core, KMS, file operations, or
//! the module ABI required by vendor-built GPU drivers.
//!
//! References:
//!   - `include/drm/drm_device.h:75`  — `struct drm_device`
//!   - `drivers/gpu/drm/drm_drv.c:1057` — `drm_dev_register`

extern crate alloc;

pub mod linux_sources;
pub mod module_abi;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EEXIST;

/// `struct drm_device` — `include/drm/drm_device.h:75`.
pub struct DrmDevice {
    pub name: String,
    pub minor: u32,
    pub mode: Mutex<Option<DrmMode>>,
}

impl DrmDevice {
    pub fn new(name: &str, minor: u32) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            minor,
            mode: Mutex::new(None),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrmMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

pub fn drm_set_mode(dev: &Arc<DrmDevice>, mode: DrmMode) -> Result<(), i32> {
    if mode.width == 0 || mode.height == 0 || mode.refresh_hz == 0 {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    *dev.mode.lock() = Some(mode);
    Ok(())
}

lazy_static! {
    static ref DRM_DEVICES: Mutex<BTreeMap<u32, Arc<DrmDevice>>> = Mutex::new(BTreeMap::new());
}

/// `drm_dev_register` — `drivers/gpu/drm/drm_drv.c:1057`.
///
/// Assigns a minor number and registers the device in the DRM table.
/// Returns the assigned minor.
pub fn drm_dev_register(dev: Arc<DrmDevice>) -> Result<u32, i32> {
    let mut g = DRM_DEVICES.lock();
    if g.contains_key(&dev.minor) {
        return Err(EEXIST);
    }
    let minor = dev.minor;
    g.insert(minor, dev);
    Ok(minor)
}

pub fn drm_dev_count() -> usize {
    DRM_DEVICES.lock().len()
}

pub fn drm_find_device(minor: u32) -> Option<Arc<DrmDevice>> {
    DRM_DEVICES.lock().get(&minor).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_drm_device() {
        let dev = DrmDevice::new("lupos-test-drm", 99);
        let minor = drm_dev_register(dev).unwrap();
        assert_eq!(minor, 99);
        assert!(drm_find_device(99).is_some());
    }

    #[test]
    fn duplicate_minor_returns_eexist() {
        let dev1 = DrmDevice::new("card-a", 88);
        let dev2 = DrmDevice::new("card-b", 88);
        drm_dev_register(dev1).unwrap();
        assert_eq!(drm_dev_register(dev2), Err(EEXIST));
    }

    #[test]
    fn mode_setting_records_active_mode() {
        let dev = DrmDevice::new("card-mode", 77);
        let mode = DrmMode {
            width: 1024,
            height: 768,
            refresh_hz: 60,
        };
        drm_set_mode(&dev, mode).unwrap();
        assert_eq!(*dev.mode.lock(), Some(mode));
    }
}
