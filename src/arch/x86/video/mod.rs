//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/video/video-common.c
//! test-origin: linux:vendor/linux/arch/x86/video/video-common.c
//! x86 boot/video common helpers.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/video/video-common.c

use crate::arch::x86::mm::paging::{
    __pgprot, _PAGE_PAT, _PAGE_PCD, _PAGE_PWT, pgprot_t, pgprot_val,
};
use crate::arch::x86::mm::pat::{PageCacheMode, pgprot_with_cachemode};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::LinuxDevice;
use crate::linux_driver_abi::pci::device::{IORESOURCE_MEM, LinuxPciDev};
use crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr;
use spin::Mutex;

/// Persistent copy of `sysfb_primary_display.screen`'s memory resource.
/// Native DRM removal disables the firmware fbdev backend, but Linux retains
/// this boot-display identity for `video_is_primary_device()` and vgaarb.
static PRIMARY_DISPLAY_RESOURCE: Mutex<Option<(u64, u64)>> = Mutex::new(None);

pub fn set_primary_display_resource(base: u64, size: u64) {
    *PRIMARY_DISPLAY_RESOURCE.lock() = (base != 0 && size != 0).then_some((base, size));
}

pub fn primary_display_resource() -> Option<(u64, u64)> {
    *PRIMARY_DISPLAY_RESOURCE.lock()
}

/// Port of Linux `pgprot_framebuffer()`. The address arguments are part of the
/// architecture hook even though x86 currently needs only the protection and
/// boot CPU family.
#[unsafe(no_mangle)]
pub extern "C" fn pgprot_framebuffer(
    prot: pgprot_t,
    _vm_start: usize,
    _vm_end: usize,
    _offset: usize,
) -> pgprot_t {
    let cache_mask = _PAGE_PWT | _PAGE_PCD | _PAGE_PAT;
    let uncached = __pgprot(pgprot_val(prot) & !cache_mask);
    // Every x86_64 processor has family > 3, so the condition in Linux's
    // shared i386/x86_64 source is unconditionally true for this target.
    pgprot_with_cachemode(uncached, PageCacheMode::UncachedMinus)
}

fn pci_is_display(pdev: *const LinuxPciDev) -> bool {
    !pdev.is_null() && unsafe { (*pdev).class >> 16 == 0x03 }
}

fn resource_contains_framebuffer(pdev: *const LinuxPciDev) -> bool {
    let Some((base, size)) = primary_display_resource() else {
        return false;
    };
    let Some(end) = base.checked_add(size - 1) else {
        return false;
    };

    unsafe {
        (*pdev).resource.iter().take(6).any(|resource| {
            resource.flags & IORESOURCE_MEM != 0
                && resource.start != 0
                && resource.start <= base
                && resource.end >= end
        })
    }
}

/// `video_is_primary_device()` from `arch/x86/video/video-common.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn video_is_primary_device(dev: *mut LinuxDevice) -> bool {
    if dev.is_null() || unsafe { (*dev).bus } != linux_pci_bus_type_ptr() {
        return false;
    }
    let pdev = unsafe {
        crate::linux_driver_abi::pci::device::linux_pci_dev_from_device(dev.cast_const())
    };
    if !pci_is_display(pdev) {
        return false;
    }
    pdev == crate::linux_driver_abi::video::vgaarb::vga_default_device()
        || resource_contains_framebuffer(pdev)
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("pgprot_framebuffer", pgprot_framebuffer as usize, false);
    export_symbol_once(
        "video_is_primary_device",
        video_is_primary_device as usize,
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_video_exports_match_x86_source_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/video/video-common.c"
        ));
        assert!(source.contains("pgprot_t pgprot_framebuffer"));
        assert!(source.contains("pgprot_val(prot) &= ~_PAGE_CACHE_MASK;"));
        assert!(source.contains("boot_cpu_data.x86 > 3"));
        assert!(source.contains("cachemode2protval(_PAGE_CACHE_MODE_UC_MINUS)"));
        assert!(source.contains("bool video_is_primary_device"));
        assert!(source.contains("if (!dev_is_pci(dev))"));
        assert!(source.contains("if (!pci_is_display(pdev))"));
        assert!(source.contains("if (pdev == vga_default_device())"));
        assert!(source.contains("pci_find_resource(pdev, &res[i])"));
        assert!(source.contains("EXPORT_SYMBOL(video_is_primary_device);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));

        use crate::arch::x86::mm::paging::{_PAGE_NX, _PAGE_RW};
        let input = __pgprot(_PAGE_RW | _PAGE_NX | _PAGE_PAT | _PAGE_PWT);
        let modern = pgprot_framebuffer(input, 0x1000, 0x2000, 0);
        assert_eq!(
            pgprot_val(modern) & (_PAGE_RW | _PAGE_NX),
            _PAGE_RW | _PAGE_NX
        );
        assert_eq!(
            pgprot_val(modern) & (_PAGE_PAT | _PAGE_PCD | _PAGE_PWT),
            _PAGE_PCD
        );
        assert!(!unsafe { video_is_primary_device(core::ptr::null_mut()) });
    }
}
