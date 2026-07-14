//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/firmware_loader
//! Firmware loader ABI for vendor-built modules.
//!
//! Lupos does not yet provide a firmware filesystem loader to Linux modules.
//! The direct request path therefore fails closed with `-ENOENT` while still
//! exporting the core firmware-loader entry points those modules link against.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::include::uapi::errno::{EINVAL, ENOENT};
use crate::kernel::module::{export_symbol, find_symbol};

#[repr(C)]
pub struct LinuxFirmware {
    pub size: usize,
    pub data: *const u8,
    pub priv_: *mut c_void,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("request_firmware", linux_request_firmware as usize, true);
    export_symbol_once(
        "firmware_request_nowarn",
        linux_firmware_request_nowarn as usize,
        true,
    );
    export_symbol_once(
        "request_firmware_direct",
        linux_request_firmware_direct as usize,
        false,
    );
    export_symbol_once("release_firmware", linux_release_firmware as usize, false);
}

/// `request_firmware` - `vendor/linux/drivers/base/firmware_loader/main.c:941`.
pub unsafe extern "C" fn linux_request_firmware(
    firmware_p: *mut *const LinuxFirmware,
    name: *const c_char,
    device: *mut c_void,
) -> i32 {
    unsafe { linux_request_firmware_direct(firmware_p, name, device) }
}

/// `firmware_request_nowarn` - `vendor/linux/drivers/base/firmware_loader/main.c:968`.
pub unsafe extern "C" fn linux_firmware_request_nowarn(
    firmware_p: *mut *const LinuxFirmware,
    name: *const c_char,
    device: *mut c_void,
) -> i32 {
    unsafe { linux_request_firmware_direct(firmware_p, name, device) }
}

/// `request_firmware_direct` - `vendor/linux/drivers/base/firmware_loader/main.c:999`.
pub unsafe extern "C" fn linux_request_firmware_direct(
    firmware_p: *mut *const LinuxFirmware,
    _name: *const c_char,
    _device: *mut c_void,
) -> i32 {
    if firmware_p.is_null() {
        return -(EINVAL as i32);
    }
    unsafe {
        ptr::write(firmware_p, ptr::null());
    }
    -(ENOENT as i32)
}

/// `release_firmware` - `vendor/linux/drivers/base/firmware_loader/main.c:1122`.
pub unsafe extern "C" fn linux_release_firmware(_fw: *const LinuxFirmware) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("request_firmware_direct"),
            Some(linux_request_firmware_direct as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("request_firmware"),
            Some(linux_request_firmware as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("firmware_request_nowarn"),
            Some(linux_firmware_request_nowarn as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("release_firmware"),
            Some(linux_release_firmware as usize)
        );
    }

    #[test]
    fn direct_firmware_request_fails_closed() {
        let mut fw = ptr::dangling::<LinuxFirmware>();
        let ret = unsafe {
            linux_request_firmware_direct(
                &mut fw as *mut *const LinuxFirmware,
                ptr::null(),
                ptr::null_mut(),
            )
        };

        assert_eq!(ret, -(ENOENT as i32));
        assert!(fw.is_null());
    }
}
