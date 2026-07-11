//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video
//! Linux `drivers/video/` ABI and the legacy framebuffer device.

use crate::kernel::module::{export_symbol, find_symbol};

pub mod aperture;
pub mod cmdline;
pub mod console;
pub mod fbdev;
pub mod logo;
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
