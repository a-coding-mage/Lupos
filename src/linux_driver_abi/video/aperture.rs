//! linux-parity: partial
//! linux-source: vendor/linux/drivers/video/aperture.c
//! Firmware-framebuffer aperture ownership and native-driver handoff.
//!
//! Lupos currently publishes one firmware framebuffer rather than Linux
//! platform-device aperture owners. These exported entry points retain Linux's
//! half-open overlap rule, PCI BAR traversal, and legacy-vgacon removal for
//! that owner. General `devm_aperture_acquire*` ownership remains unimplemented.

use core::ffi::c_char;

use crate::linux_driver_abi::pci::device::{IORESOURCE_MEM, LinuxPciDev};

const PCI_STD_NUM_BARS: usize = 6;

const fn overlap(base1: u64, end1: u64, base2: u64, end2: u64) -> bool {
    base1 < end2 && end1 > base2
}

fn detach_if_overlapping(base: u64, size: u64) {
    if crate::linux_driver_abi::video::fbdev::core::fb_info().is_none() {
        return;
    }
    let Some((framebuffer_base, framebuffer_size)) =
        crate::arch::x86::video::primary_display_resource()
    else {
        return;
    };
    let Some(end) = base.checked_add(size) else {
        return;
    };
    let Some(framebuffer_end) = framebuffer_base.checked_add(framebuffer_size) else {
        return;
    };
    if overlap(base, end, framebuffer_base, framebuffer_end) {
        crate::linux_driver_abi::video::fbdev::detach_firmware_framebuffer();
    }
}

/// Linux `aperture_remove_conflicting_devices()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aperture_remove_conflicting_devices(
    base: u64,
    size: u64,
    _name: *const c_char,
) -> i32 {
    detach_if_overlapping(base, size);
    0
}

/// Linux `__aperture_remove_legacy_vga_devices()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __aperture_remove_legacy_vga_devices(pdev: *mut LinuxPciDev) -> i32 {
    // Lupos has no separate vga16fb device. The legacy text console is the
    // remaining VGA owner and follows Linux's vga_remove_vgacon() gate.
    unsafe { super::vgaarb::vga_remove_vgacon(pdev) }
}

/// Linux `aperture_remove_conflicting_pci_devices()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aperture_remove_conflicting_pci_devices(
    pdev: *mut LinuxPciDev,
    name: *const c_char,
) -> i32 {
    if pdev.is_null() {
        return 0;
    }

    let resources = unsafe { &(*pdev).resource };
    for resource in &resources[..PCI_STD_NUM_BARS] {
        if resource.flags & IORESOURCE_MEM == 0 || resource.end < resource.start {
            continue;
        }
        let size = resource.end - resource.start + 1;
        unsafe {
            aperture_remove_conflicting_devices(resource.start, size, name);
        }
    }

    if pdev == super::vgaarb::vga_default_device() {
        return unsafe { __aperture_remove_legacy_vga_devices(pdev) };
    }
    0
}
