//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video
//! Linux `drivers/video/` ABI and the legacy framebuffer device.

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::ENODEV;
use crate::kernel::module::{export_symbol, find_symbol};

pub mod aperture;
pub mod cmdline;
pub mod console;
pub mod fbdev;
pub mod hdmi;
pub mod logo;
pub mod mipi_dsi;
pub mod nomodeset;
pub mod vgaarb;

pub fn configure_from_cmdline(cmdline: &str) {
    cmdline::configure_from_cmdline(cmdline);
    nomodeset::configure_from_cmdline(cmdline);
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    hdmi::register_module_exports();
    mipi_dsi::register_module_exports();
    export_symbol_once(
        "devm_of_find_backlight",
        devm_of_find_backlight as usize,
        false,
    );
    export_symbol_once(
        "devm_backlight_device_register",
        devm_backlight_device_register as usize,
        false,
    );
    export_symbol_once(
        "backlight_device_get_by_name",
        backlight_device_get_by_name as usize,
        false,
    );
    export_symbol_once(
        "backlight_device_register",
        backlight_device_register as usize,
        false,
    );
    export_symbol_once(
        "backlight_device_unregister",
        backlight_device_unregister as usize,
        false,
    );
    export_symbol_once(
        "video_firmware_drivers_only",
        nomodeset::video_firmware_drivers_only as usize,
        false,
    );
    export_symbol_once(
        "video_get_options",
        cmdline::video_get_options as usize,
        false,
    );
    // Linux exports __video_get_options only with CONFIG_FB_CORE.  The
    // x86_64 vendor-module policy keeps FB_CORE/DRM fbdev emulation disabled;
    // the unconditional video_get_options export above is the one DRM uses.
    export_symbol_once(
        "vga_default_device",
        vgaarb::vga_default_device as usize,
        true,
    );
    export_symbol_once(
        "vga_client_register",
        vgaarb::vga_client_register as usize,
        false,
    );
    export_symbol_once("vga_get", vgaarb::vga_get as usize, false);
    export_symbol_once("vga_put", vgaarb::vga_put as usize, false);
    export_symbol_once(
        "vga_set_legacy_decoding",
        vgaarb::vga_set_legacy_decoding as usize,
        false,
    );
    export_symbol_once(
        "vga_remove_vgacon",
        vgaarb::vga_remove_vgacon as usize,
        false,
    );
    export_symbol_once(
        "aperture_remove_conflicting_devices",
        aperture::aperture_remove_conflicting_devices as usize,
        false,
    );
    export_symbol_once(
        "__aperture_remove_legacy_vga_devices",
        aperture::__aperture_remove_legacy_vga_devices as usize,
        false,
    );
    export_symbol_once(
        "aperture_remove_conflicting_pci_devices",
        aperture::aperture_remove_conflicting_pci_devices as usize,
        false,
    );
}

/// Linux `devm_of_find_backlight()`.
///
/// Lupos does not currently publish OF-backed backlight devices. Match the
/// Linux result for a device without a `backlight` phandle.
pub unsafe extern "C" fn devm_of_find_backlight(_dev: *mut c_void) -> *mut c_void {
    core::ptr::null_mut()
}

/// Linux `devm_backlight_device_register()`.
///
/// The generic backlight class is not instantiated in Lupos yet. Return the
/// standard error-pointer form so callers can keep their Linux error handling.
pub unsafe extern "C" fn devm_backlight_device_register(
    _dev: *mut c_void,
    _name: *const c_char,
    _parent: *mut c_void,
    _devdata: *mut c_void,
    _ops: *const c_void,
    _props: *const c_void,
) -> *mut c_void {
    (-(ENODEV as isize)) as *mut c_void
}

/// Linux `backlight_device_get_by_name()`.
///
/// No generic backlight class devices are registered by Lupos yet, so every
/// class lookup misses just as it would on Linux before any backlight device
/// has been published.
pub unsafe extern "C" fn backlight_device_get_by_name(_name: *const c_char) -> *mut c_void {
    core::ptr::null_mut()
}

/// Linux `backlight_device_register()`.
///
/// Lupos does not yet instantiate the generic backlight class. Return the
/// standard error-pointer form instead of fabricating a partial class device.
pub unsafe extern "C" fn backlight_device_register(
    _name: *const c_char,
    _parent: *mut c_void,
    _devdata: *mut c_void,
    _ops: *const c_void,
    _props: *const c_void,
) -> *mut c_void {
    (-(ENODEV as isize)) as *mut c_void
}

/// Linux `backlight_device_unregister()`.
pub unsafe extern "C" fn backlight_device_unregister(_bd: *mut c_void) {}
