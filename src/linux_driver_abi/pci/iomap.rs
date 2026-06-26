//! linux-parity: complete
//! linux-source: vendor/linux/drivers/pci/iomap.c
//! test-origin: linux:vendor/linux/drivers/pci/iomap.c
//! PCI BAR ioremap helpers for Linux-built PCI modules.

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::mm::ioremap::{IoremapMapping, ioremap, ioremap_wc, iounmap};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::pci::device::{
    IORESOURCE_IO, IORESOURCE_MEM, LinuxPciBarResource, PCI_STD_NUM_BARS, linux_pci_bar_resource,
};

pub const PCI_IOBASE: usize = 0;
pub const IO_SPACE_LIMIT: usize = 0xffff;
pub const PIO_INDIRECT_SIZE: usize = 0;
pub const MMIO_UPPER_LIMIT: usize = IO_SPACE_LIMIT - PIO_INDIRECT_SIZE;

#[derive(Clone, Copy)]
struct RegisteredPciIomap {
    addr: usize,
    mapping: IoremapMapping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PciIomapPlan {
    Null,
    IoPort {
        addr: usize,
        len: usize,
    },
    Mmio {
        start: u64,
        len: u64,
        write_combining: bool,
    },
}

lazy_static! {
    static ref PCI_IOMAPS: Mutex<Vec<RegisteredPciIomap>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("pci_iomap_range", pci_iomap_range as usize, false);
    export_symbol_once("pci_iomap_wc_range", pci_iomap_wc_range as usize, true);
    export_symbol_once("pci_iomap", pci_iomap as usize, false);
    export_symbol_once("pci_iomap_wc", pci_iomap_wc as usize, true);
    export_symbol_once("pci_ioremap_bar", pci_ioremap_bar as usize, false);
    export_symbol_once("pci_iounmap", pci_iounmap as usize, false);
}

fn pci_ioport_map(port: u64, _nr: u64) -> Option<usize> {
    let port = (port as usize) & IO_SPACE_LIMIT;
    if port > MMIO_UPPER_LIMIT {
        return None;
    }
    Some(PCI_IOBASE + port)
}

fn generic_ioport_cookie(addr: usize) -> bool {
    let start = PCI_IOBASE;
    addr >= start && addr < start + IO_SPACE_LIMIT
}

fn pci_iomap_range_plan(
    resource: Option<LinuxPciBarResource>,
    bar_valid: bool,
    offset: usize,
    maxlen: usize,
    write_combining: bool,
) -> PciIomapPlan {
    if !bar_valid {
        return PciIomapPlan::Null;
    }
    let Some(resource) = resource else {
        return PciIomapPlan::Null;
    };

    let offset = offset as u64;
    if resource.len <= offset || resource.start == 0 {
        return PciIomapPlan::Null;
    }
    if write_combining && resource.flags & IORESOURCE_IO != 0 {
        return PciIomapPlan::Null;
    }

    let mut len = resource.len - offset;
    let start = resource.start + offset;
    if maxlen != 0 && len > maxlen as u64 {
        len = maxlen as u64;
    }

    if resource.flags & IORESOURCE_IO != 0 {
        return match pci_ioport_map(start, len) {
            Some(addr) => PciIomapPlan::IoPort {
                addr,
                len: len as usize,
            },
            None => PciIomapPlan::Null,
        };
    }
    if resource.flags & IORESOURCE_MEM != 0 {
        return PciIomapPlan::Mmio {
            start,
            len,
            write_combining,
        };
    }

    PciIomapPlan::Null
}

unsafe fn realize_pci_iomap_plan(plan: PciIomapPlan) -> *mut c_void {
    let mapping = match plan {
        PciIomapPlan::Null => return core::ptr::null_mut(),
        PciIomapPlan::IoPort { addr, .. } => return addr as *mut c_void,
        PciIomapPlan::Mmio {
            start,
            len,
            write_combining: false,
        } => unsafe { ioremap(start, len) },
        PciIomapPlan::Mmio {
            start,
            len,
            write_combining: true,
        } => unsafe { ioremap_wc(start, len) },
    };

    let Ok(mapping) = mapping else {
        return core::ptr::null_mut();
    };
    let addr = mapping.virt as usize;
    PCI_IOMAPS.lock().push(RegisteredPciIomap { addr, mapping });
    addr as *mut c_void
}

/// `pci_iomap_range` - `vendor/linux/drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_iomap_range(
    dev: *mut c_void,
    bar: i32,
    offset: usize,
    maxlen: usize,
) -> *mut c_void {
    let Ok(bar) = usize::try_from(bar) else {
        return core::ptr::null_mut();
    };
    let bar_valid = bar < PCI_STD_NUM_BARS;
    let resource = bar_valid
        .then(|| linux_pci_bar_resource(dev.cast_const(), bar))
        .flatten();
    let plan = pci_iomap_range_plan(resource, bar_valid, offset, maxlen, false);
    unsafe { realize_pci_iomap_plan(plan) }
}

/// `pci_iomap_wc_range` - `vendor/linux/drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_iomap_wc_range(
    dev: *mut c_void,
    bar: i32,
    offset: usize,
    maxlen: usize,
) -> *mut c_void {
    let Ok(bar) = usize::try_from(bar) else {
        return core::ptr::null_mut();
    };
    let bar_valid = bar < PCI_STD_NUM_BARS;
    let resource = bar_valid
        .then(|| linux_pci_bar_resource(dev.cast_const(), bar))
        .flatten();
    let plan = pci_iomap_range_plan(resource, bar_valid, offset, maxlen, true);
    unsafe { realize_pci_iomap_plan(plan) }
}

/// `pci_iomap` - `vendor/linux/drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_iomap(dev: *mut c_void, bar: i32, maxlen: usize) -> *mut c_void {
    unsafe { pci_iomap_range(dev, bar, 0, maxlen) }
}

/// `pci_iomap_wc` - `vendor/linux/drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_iomap_wc(dev: *mut c_void, bar: i32, maxlen: usize) -> *mut c_void {
    unsafe { pci_iomap_wc_range(dev, bar, 0, maxlen) }
}

/// `pci_ioremap_bar` - `vendor/linux/drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_ioremap_bar(dev: *mut c_void, bar: i32) -> *mut c_void {
    unsafe { pci_iomap(dev, bar, 0) }
}

/// `pci_iounmap` - `vendor/linux/lib/iomap.c` / `drivers/pci/iomap.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_iounmap(_dev: *mut c_void, addr: *mut c_void) {
    if addr.is_null() {
        return;
    }
    if generic_ioport_cookie(addr as usize) {
        return;
    }
    let mut maps = PCI_IOMAPS.lock();
    if let Some(index) = maps
        .iter()
        .position(|mapping| mapping.addr == addr as usize)
    {
        let mapping = maps.swap_remove(index).mapping;
        drop(maps);
        unsafe { iounmap(mapping) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::ioremap::prot_for_cachemode;
    use crate::arch::x86::mm::pat::PageCacheMode;
    use crate::linux_driver_abi::pci::device::{
        IORESOURCE_IO, IORESOURCE_MEM, LinuxPciBarResource, LinuxPciDeviceAbiState,
        PCI_CONFIG_SPACE_SIZE, PCI_STD_NUM_BARS, register_linux_pci_device_state,
        unregister_linux_pci_device_state,
    };

    fn register_test_device(
        dev: *mut c_void,
        bars: [Option<LinuxPciBarResource>; PCI_STD_NUM_BARS],
    ) {
        register_linux_pci_device_state(
            dev,
            LinuxPciDeviceAbiState {
                config_space: [0; PCI_CONFIG_SPACE_SIZE],
                bars,
            },
        );
    }

    fn mapping_for(addr: *mut c_void) -> Option<IoremapMapping> {
        PCI_IOMAPS
            .lock()
            .iter()
            .find(|mapping| mapping.addr == addr as usize)
            .map(|mapping| mapping.mapping)
    }

    #[test]
    fn pci_iomap_matches_linux_source() {
        let iomap_c = include_str!("../../../vendor/linux/drivers/pci/iomap.c");
        let pci_iomap_h = include_str!("../../../vendor/linux/include/asm-generic/pci_iomap.h");
        let generic_io_h = include_str!("../../../vendor/linux/include/asm-generic/io.h");
        let x86_io_h = include_str!("../../../vendor/linux/arch/x86/include/asm/io.h");

        assert!(iomap_c.contains("void __iomem *pci_iomap_range"));
        assert!(iomap_c.contains("return __pci_ioport_map(dev, start, len);"));
        assert!(iomap_c.contains("void __iomem *pci_iomap_wc_range"));
        assert!(iomap_c.contains("return ioremap_wc(start, len);"));
        assert!(iomap_c.contains("EXPORT_SYMBOL_GPL(pci_iomap_wc);"));
        assert!(
            pci_iomap_h
                .contains("#define __pci_ioport_map(dev, port, nr) ioport_map((port), (nr))")
        );
        assert!(generic_io_h.contains("port &= IO_SPACE_LIMIT;"));
        assert!(generic_io_h.contains("#define ARCH_WANTS_GENERIC_PCI_IOUNMAP"));
        assert!(x86_io_h.contains("#define IO_SPACE_LIMIT 0xffff"));
    }

    #[test]
    fn pci_iomap_exports_module_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("pci_iomap_range"),
            Some(pci_iomap_range as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_iomap_wc_range"),
            Some(pci_iomap_wc_range as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_iomap_wc"),
            Some(pci_iomap_wc as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_iounmap"),
            Some(pci_iounmap as usize)
        );
    }

    #[test]
    fn pci_iomap_maps_registered_mmio_bar_and_unmaps_cookie() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut bars = [None; PCI_STD_NUM_BARS];
        bars[0] = Some(LinuxPciBarResource {
            start: 0x200000,
            len: 0x1000,
            flags: IORESOURCE_MEM,
        });
        register_test_device(dev, bars);

        let addr = unsafe { pci_iomap_range(dev, 0, 0x20, 0x40) };
        assert!(!addr.is_null());
        let mapping = mapping_for(addr).expect("registered pci_iomap mapping");
        assert_eq!(mapping.phys, 0x200020);
        assert_eq!(mapping.size, 0x40);
        unsafe { pci_iounmap(dev, addr) };
        assert!(mapping_for(addr).is_none());

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_iomap_wc_maps_mmio_with_write_combining() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut bars = [None; PCI_STD_NUM_BARS];
        bars[1] = Some(LinuxPciBarResource {
            start: 0x300000,
            len: 0x2000,
            flags: IORESOURCE_MEM,
        });
        register_test_device(dev, bars);

        let addr = unsafe { pci_iomap_wc(dev, 1, 0x80) };
        assert!(!addr.is_null());
        let mapping = mapping_for(addr).expect("registered wc pci_iomap mapping");
        assert_eq!(mapping.phys, 0x300000);
        assert_eq!(mapping.size, 0x80);
        assert_eq!(
            mapping.prot,
            prot_for_cachemode(PageCacheMode::WriteCombining)
        );
        unsafe { pci_iounmap(dev, addr) };

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_iomap_maps_io_bar_to_generic_ioport_cookie() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut bars = [None; PCI_STD_NUM_BARS];
        bars[2] = Some(LinuxPciBarResource {
            start: 0x3f0,
            len: 0x20,
            flags: IORESOURCE_IO,
        });
        register_test_device(dev, bars);

        let addr = unsafe { pci_iomap_range(dev, 2, 0x8, 0x4) };
        assert_eq!(addr as usize, 0x3f8);
        assert_eq!(
            pci_iomap_range_plan(bars[2], true, 0x8, 0x4, false),
            PciIomapPlan::IoPort {
                addr: 0x3f8,
                len: 0x4
            }
        );
        unsafe { pci_iounmap(dev, addr) };
        assert!(mapping_for(addr).is_none());

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_iomap_wc_rejects_io_bars() {
        let resource = Some(LinuxPciBarResource {
            start: 0x3f0,
            len: 0x20,
            flags: IORESOURCE_IO,
        });
        assert_eq!(
            pci_iomap_range_plan(resource, true, 0, 0x10, true),
            PciIomapPlan::Null
        );
    }

    #[test]
    fn pci_iomap_rejects_invalid_or_empty_resources() {
        let mmio = Some(LinuxPciBarResource {
            start: 0x400000,
            len: 0x100,
            flags: IORESOURCE_MEM,
        });
        assert_eq!(
            pci_iomap_range_plan(mmio, false, 0, 0, false),
            PciIomapPlan::Null
        );
        assert_eq!(
            pci_iomap_range_plan(mmio, true, 0x100, 0, false),
            PciIomapPlan::Null
        );
        assert_eq!(
            pci_iomap_range_plan(
                Some(LinuxPciBarResource {
                    start: 0,
                    len: 0x100,
                    flags: IORESOURCE_MEM
                }),
                true,
                0,
                0,
                false
            ),
            PciIomapPlan::Null
        );
    }
}
