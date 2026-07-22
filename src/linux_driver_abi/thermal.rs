//! linux-parity: partial
//! linux-source: vendor/linux/drivers/thermal/thermal_core.c
//! linux-source: vendor/linux/drivers/thermal/intel/intel_tcc.c
//! linux-source: vendor/linux/drivers/thermal/intel/therm_throt.c
//! test-origin: linux:vendor/linux/drivers/thermal
//! Thermal core ABI used by Linux-built thermal drivers.
//!
//! Lupos does not yet expose Linux's full thermal class, netlink, hwmon, or
//! interrupt reporting stack.  This module preserves the C ABI, symbol export
//! policy, and object lifetime expected by vendor-built modules.

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::{c_char, c_void};
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::arch::x86::kernel::msr;
use crate::include::uapi::errno::{EINVAL, ENODATA, ENODEV, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};

const THERMAL_NAME_LENGTH: usize = 20;
const MSR_IA32_THERM_STATUS: u32 = 0x0000_019c;
const MSR_IA32_TEMPERATURE_TARGET: u32 = 0x0000_01a2;
const MSR_IA32_PACKAGE_THERM_STATUS: u32 = 0x0000_01b1;
const THERM_STATUS_VALID: u64 = 1 << 31;

#[repr(C)]
struct LinuxThermalZoneDevice {
    devdata: *mut c_void,
    ops: *const c_void,
    num_trips: i32,
    enabled: AtomicU32,
    type_name: [u8; THERMAL_NAME_LENGTH],
}

static mut PLATFORM_THERMAL_PACKAGE_NOTIFY: usize = 0;
static mut PLATFORM_THERMAL_PACKAGE_RATE_CONTROL: usize = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "thermal_zone_device_register_with_trips",
        thermal_zone_device_register_with_trips as usize,
        true,
    );
    export_symbol_once(
        "thermal_zone_device_enable",
        thermal_zone_device_enable as usize,
        true,
    );
    export_symbol_once(
        "thermal_zone_device_update",
        thermal_zone_device_update as usize,
        true,
    );
    export_symbol_once(
        "thermal_zone_device_priv",
        thermal_zone_device_priv as usize,
        true,
    );
    export_symbol_once(
        "thermal_zone_device_unregister",
        thermal_zone_device_unregister as usize,
        true,
    );
    export_symbol_once("intel_tcc_get_tjmax", intel_tcc_get_tjmax as usize, true);
    export_symbol_once("intel_tcc_get_temp", intel_tcc_get_temp as usize, true);
    export_symbol_once(
        "thermal_clear_package_intr_status",
        thermal_clear_package_intr_status as usize,
        true,
    );
    export_symbol_once(
        "platform_thermal_package_notify",
        core::ptr::addr_of_mut!(PLATFORM_THERMAL_PACKAGE_NOTIFY) as usize,
        true,
    );
    export_symbol_once(
        "platform_thermal_package_rate_control",
        core::ptr::addr_of_mut!(PLATFORM_THERMAL_PACKAGE_RATE_CONTROL) as usize,
        true,
    );
}

fn err_ptr(errno: i32) -> *mut c_void {
    let errno = if errno < 0 { errno } else { -errno };
    errno as isize as usize as *mut c_void
}

fn linux_errno(errno: i32) -> i32 {
    if errno < 0 { errno } else { -errno }
}

unsafe fn strnlen(ptr: *const c_char, limit: usize) -> usize {
    let mut len = 0usize;
    while len < limit {
        if unsafe { ptr.add(len).read() } == 0 {
            break;
        }
        len += 1;
    }
    len
}

/// `thermal_zone_device_register_with_trips` -
/// `vendor/linux/drivers/thermal/thermal_core.c:1405`.
#[unsafe(export_name = "thermal_zone_device_register_with_trips")]
pub unsafe extern "C" fn thermal_zone_device_register_with_trips(
    type_name: *const c_char,
    trips: *const c_void,
    num_trips: i32,
    devdata: *mut c_void,
    ops: *const c_void,
    _tzp: *const c_void,
    passive_delay: u32,
    polling_delay: u32,
) -> *mut c_void {
    if type_name.is_null() {
        return err_ptr(EINVAL);
    }
    let type_len = unsafe { strnlen(type_name, THERMAL_NAME_LENGTH) };
    if type_len == 0 || type_len == THERMAL_NAME_LENGTH {
        return err_ptr(EINVAL);
    }
    if num_trips < 0 || ops.is_null() || (num_trips > 0 && trips.is_null()) {
        return err_ptr(EINVAL);
    }
    if polling_delay != 0 && passive_delay > polling_delay {
        return err_ptr(EINVAL);
    }

    let mut zone = Box::new(LinuxThermalZoneDevice {
        devdata,
        ops,
        num_trips,
        enabled: AtomicU32::new(0),
        type_name: [0; THERMAL_NAME_LENGTH],
    });
    unsafe {
        ptr::copy_nonoverlapping(
            type_name.cast::<u8>(),
            zone.type_name.as_mut_ptr(),
            type_len,
        );
    }
    let raw = Box::into_raw(zone);
    if raw.is_null() {
        err_ptr(ENOMEM)
    } else {
        raw.cast()
    }
}

/// `thermal_zone_device_enable` -
/// `vendor/linux/drivers/thermal/thermal_core.c:626`.
#[unsafe(export_name = "thermal_zone_device_enable")]
pub unsafe extern "C" fn thermal_zone_device_enable(tz: *mut c_void) -> i32 {
    if tz.is_null() {
        return -EINVAL;
    }
    unsafe {
        (*(tz.cast::<LinuxThermalZoneDevice>()))
            .enabled
            .store(1, Ordering::Release);
    }
    0
}

/// `thermal_zone_device_update` -
/// `vendor/linux/drivers/thermal/thermal_core.c:638`.
#[unsafe(export_name = "thermal_zone_device_update")]
pub unsafe extern "C" fn thermal_zone_device_update(_tz: *mut c_void, _event: i32) {}

/// `thermal_zone_device_priv` -
/// `vendor/linux/drivers/thermal/thermal_core.c:1576`.
#[unsafe(export_name = "thermal_zone_device_priv")]
pub unsafe extern "C" fn thermal_zone_device_priv(tz: *mut c_void) -> *mut c_void {
    if tz.is_null() {
        return ptr::null_mut();
    }
    unsafe { (*(tz.cast::<LinuxThermalZoneDevice>())).devdata }
}

/// `thermal_zone_device_unregister` -
/// `vendor/linux/drivers/thermal/thermal_core.c:1625`.
#[unsafe(export_name = "thermal_zone_device_unregister")]
pub unsafe extern "C" fn thermal_zone_device_unregister(tz: *mut c_void) {
    if tz.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(tz.cast::<LinuxThermalZoneDevice>()));
    }
}

/// `intel_tcc_get_tjmax` -
/// `vendor/linux/drivers/thermal/intel/intel_tcc.c:152`.
#[unsafe(export_name = "intel_tcc_get_tjmax")]
pub unsafe extern "C" fn intel_tcc_get_tjmax(cpu: i32) -> i32 {
    if cpu >= 0 && cpu as u32 >= crate::kernel::cpuhotplug::nr_cpu_ids() {
        return -ENODEV;
    }
    let mut value = 0u64;
    let result = if cpu < 0 {
        unsafe { msr::rdmsr_safe(MSR_IA32_TEMPERATURE_TARGET) }
    } else {
        msr::rdmsrq_safe_on_cpu(cpu as u32, MSR_IA32_TEMPERATURE_TARGET, &mut value).map(|()| value)
    };

    match result {
        Ok(msr_value) => {
            let tjmax = ((msr_value >> 16) & 0xff) as i32;
            if tjmax == 0 { -ENODATA } else { tjmax }
        }
        Err(errno) => linux_errno(errno),
    }
}

/// `intel_tcc_get_temp` -
/// `vendor/linux/drivers/thermal/intel/intel_tcc.c:234`.
#[unsafe(export_name = "intel_tcc_get_temp")]
pub unsafe extern "C" fn intel_tcc_get_temp(cpu: i32, temp: *mut i32, pkg: bool) -> i32 {
    if temp.is_null() {
        return -EINVAL;
    }
    let tjmax = unsafe { intel_tcc_get_tjmax(cpu) };
    if tjmax < 0 {
        return tjmax;
    }

    let msr_no = if pkg {
        MSR_IA32_PACKAGE_THERM_STATUS
    } else {
        MSR_IA32_THERM_STATUS
    };
    let mut value = 0u64;
    let result = if cpu < 0 {
        unsafe { msr::rdmsr_safe(msr_no) }
    } else {
        msr::rdmsrq_safe_on_cpu(cpu as u32, msr_no, &mut value).map(|()| value)
    };

    match result {
        Ok(msr_value) if msr_value & THERM_STATUS_VALID != 0 => {
            unsafe {
                temp.write(tjmax - ((msr_value >> 16) & 0xff) as i32);
            }
            0
        }
        Ok(_) => -ENODATA,
        Err(errno) => linux_errno(errno),
    }
}

/// `thermal_clear_package_intr_status` -
/// `vendor/linux/drivers/thermal/intel/therm_throt.c:263`.
#[unsafe(export_name = "thermal_clear_package_intr_status")]
pub unsafe extern "C" fn thermal_clear_package_intr_status(_level: i32, _bit_mask: u64) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_core_exports_track_vendor_source() {
        let source = include_str!("../../vendor/linux/drivers/thermal/thermal_core.c");

        assert!(source.contains("EXPORT_SYMBOL_GPL(thermal_zone_device_register_with_trips);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(thermal_zone_device_enable);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(thermal_zone_device_update);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(thermal_zone_device_priv);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(thermal_zone_device_unregister);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("thermal_zone_device_register_with_trips"),
            Some(thermal_zone_device_register_with_trips as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("thermal_zone_device_register_with_trips"),
            Some(true)
        );
    }

    #[test]
    fn intel_tcc_and_therm_throt_exports_track_vendor_source() {
        let tcc_source = include_str!("../../vendor/linux/drivers/thermal/intel/intel_tcc.c");
        let throt_source = include_str!("../../vendor/linux/drivers/thermal/intel/therm_throt.c");

        assert!(tcc_source.contains("EXPORT_SYMBOL_NS_GPL(intel_tcc_get_tjmax"));
        assert!(tcc_source.contains("EXPORT_SYMBOL_NS_GPL(intel_tcc_get_temp"));
        assert!(throt_source.contains("EXPORT_SYMBOL_GPL(platform_thermal_package_notify);"));
        assert!(throt_source.contains("EXPORT_SYMBOL_GPL(platform_thermal_package_rate_control);"));
        assert!(throt_source.contains("EXPORT_SYMBOL_GPL(thermal_clear_package_intr_status);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("intel_tcc_get_tjmax"),
            Some(intel_tcc_get_tjmax as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("intel_tcc_get_tjmax"),
            Some(true)
        );
        assert!(crate::kernel::module::find_symbol("platform_thermal_package_notify").is_some());
    }

    #[test]
    fn thermal_zone_registration_preserves_private_data() {
        let name = c"x86_pkg_temp";
        let mut private = 42u32;
        let ops = 1usize as *const c_void;
        let trips = 1usize as *const c_void;

        let zone = unsafe {
            thermal_zone_device_register_with_trips(
                name.as_ptr(),
                trips,
                1,
                (&mut private as *mut u32).cast(),
                ops,
                ptr::null(),
                0,
                0,
            )
        };

        assert!(zone as isize > 0);
        assert_eq!(
            unsafe { thermal_zone_device_priv(zone) },
            (&mut private as *mut u32).cast()
        );
        assert_eq!(unsafe { thermal_zone_device_enable(zone) }, 0);
        unsafe { thermal_zone_device_unregister(zone) };
    }

    #[test]
    fn thermal_zone_registration_rejects_linux_invalid_edges() {
        let name = c"x86_pkg_temp";
        let ops = 1usize as *const c_void;

        assert_eq!(
            unsafe {
                thermal_zone_device_register_with_trips(
                    ptr::null(),
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    ops,
                    ptr::null(),
                    0,
                    0,
                )
            } as isize,
            -(EINVAL as isize)
        );
        assert_eq!(
            unsafe {
                thermal_zone_device_register_with_trips(
                    name.as_ptr(),
                    ptr::null(),
                    1,
                    ptr::null_mut(),
                    ops,
                    ptr::null(),
                    0,
                    0,
                )
            } as isize,
            -(EINVAL as isize)
        );
    }
}
