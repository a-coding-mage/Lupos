//! linux-parity: partial
//! linux-source: vendor/linux/drivers/hwmon/hwmon.c
//! test-origin: linux:vendor/linux/drivers/hwmon/hwmon.c
//! Hardware-monitoring class ABI used by Linux-built drivers.
//!
//! Lupos does not yet expose the Linux hwmon class or sensor sysfs files.  The
//! registration helpers therefore fail closed while keeping optional driver
//! paths, such as i915 telemetry setup, from blocking module relocation.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::{EINVAL, ENODEV};
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "hwmon_device_register_with_groups",
        hwmon_device_register_with_groups as usize,
        true,
    );
    export_symbol_once(
        "hwmon_device_register_with_info",
        hwmon_device_register_with_info as usize,
        true,
    );
    export_symbol_once(
        "hwmon_device_unregister",
        hwmon_device_unregister as usize,
        true,
    );
}

fn err_ptr(errno: i32) -> *mut c_void {
    let errno = if errno < 0 { errno } else { -errno };
    errno as isize as usize as *mut c_void
}

/// `hwmon_device_register_with_groups` — `vendor/linux/drivers/hwmon/hwmon.c`.
///
/// The core hwmon class is absent, so valid requests return `ERR_PTR(-ENODEV)`.
/// Linux rejects a missing name before entering the shared registration path.
pub unsafe extern "C" fn hwmon_device_register_with_groups(
    _dev: *mut c_void,
    name: *const c_char,
    _drvdata: *mut c_void,
    _groups: *const *const c_void,
) -> *mut c_void {
    if name.is_null() {
        return err_ptr(EINVAL);
    }
    err_ptr(ENODEV)
}

/// `hwmon_device_register_with_info` — `vendor/linux/drivers/hwmon/hwmon.c`.
///
/// The core hwmon class is absent, so valid requests return `ERR_PTR(-ENODEV)`.
/// Invalid mandatory arguments keep Linux's `-EINVAL` edge.
pub unsafe extern "C" fn hwmon_device_register_with_info(
    dev: *mut c_void,
    name: *const c_char,
    _drvdata: *mut c_void,
    chip: *const c_void,
    _extra_groups: *const *const c_void,
) -> *mut c_void {
    if dev.is_null() || name.is_null() || chip.is_null() {
        return err_ptr(EINVAL);
    }
    err_ptr(ENODEV)
}

/// `hwmon_device_unregister` — `vendor/linux/drivers/hwmon/hwmon.c`.
///
/// No hwmon devices are registered by this shim, so unregister has no core
/// state to tear down.
pub unsafe extern "C" fn hwmon_device_unregister(_dev: *mut c_void) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwmon_group_registration_export_tracks_vendor_source() {
        let source = include_str!("../../vendor/linux/drivers/hwmon/hwmon.c");

        assert!(source.contains("EXPORT_SYMBOL_GPL(hwmon_device_register_with_groups);"));

        register_module_exports();
        assert_eq!(
            find_symbol("hwmon_device_register_with_groups"),
            Some(hwmon_device_register_with_groups as usize)
        );
    }

    #[test]
    fn hwmon_group_registration_fails_closed_without_hwmon_class() {
        let name = c"tg3";

        let null_name = unsafe {
            hwmon_device_register_with_groups(
                core::ptr::null_mut(),
                core::ptr::null(),
                core::ptr::null_mut(),
                core::ptr::null(),
            )
        };
        let valid_name = unsafe {
            hwmon_device_register_with_groups(
                core::ptr::null_mut(),
                name.as_ptr(),
                core::ptr::null_mut(),
                core::ptr::null(),
            )
        };

        assert_eq!(null_name as isize, -(EINVAL as isize));
        assert_eq!(valid_name as isize, -(ENODEV as isize));
    }
}
