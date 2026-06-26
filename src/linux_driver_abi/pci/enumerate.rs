//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci
//! test-origin: linux:vendor/linux/drivers/pci
//! PCI bus enumeration — `drivers/pci/probe.c`.
//!
//! Scans segment/bus/dev/func combinations via ECAM, identifies valid
//! devices (vendor != 0xFFFF), decodes class and IDs.
//!
//! References:
//!   - `drivers/pci/probe.c` — `pci_scan_root_bus`, `pci_scan_bus`
//!   - `drivers/pci/bus.c`   — bus object management
//!   - PCI 3.0 §6.1          — configuration space header layout

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::arch::x86::include::asm::io::{inb, inl, inw, outl, outw};
use crate::arch::x86::pci::early::{PCI_CONFIG_ADDRESS_PORT, PCI_CONFIG_DATA_PORT};
use crate::linux_driver_abi::pci::PCI_BUSES;
use crate::linux_driver_abi::pci::device::{
    PCI_CONFIG_SPACE_SIZE, PciBar, PciBus, PciDev, register_linux_pci_device,
};
use crate::linux_driver_abi::pci::ecam::McfgEntry;

// PCI config-space header offsets (Type-0 / Type-1 common header).
const PCI_VENDOR_ID: u16 = 0x00;
const PCI_DEVICE_ID: u16 = 0x02;
const PCI_CLASS_PROG: u16 = 0x09; // prog-if byte
const PCI_CLASS_DEVICE: u16 = 0x0A; // class code word
const PCI_REVISION_ID: u16 = 0x08;
const PCI_HEADER_TYPE: u16 = 0x0E;
const PCI_COMMAND: u16 = 0x04;
const PCI_COMMAND_IO: u16 = 0x1;
const PCI_COMMAND_MEMORY: u16 = 0x2;
const PCI_COMMAND_DECODE_ENABLE: u16 = PCI_COMMAND_IO | PCI_COMMAND_MEMORY;
const PCI_BASE_ADDRESS_0: u16 = 0x10;
const PCI_BASE_ADDRESS_MEM_MASK: u64 = 0xffff_fff0;
const PCI_BASE_ADDRESS_IO_MASK: u64 = 0xffff_fffc;
const PCI_SUBSYSTEM_VID: u16 = 0x2C;
const PCI_SUBSYSTEM_ID: u16 = 0x2E;

static LEGACY_CF8_CONFIG_LOCK: spin::Mutex<()> = spin::Mutex::new(());

trait PciConfigAccess {
    fn segment(&self) -> u16;
    fn bus_start(&self) -> u8;
    fn bus_end(&self) -> u8;

    unsafe fn read8(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u8;
    unsafe fn read16(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u16;
    unsafe fn read32(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u32;
    unsafe fn write16(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u16);
    unsafe fn write32(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u32);
}

impl PciConfigAccess for McfgEntry {
    fn segment(&self) -> u16 {
        self.segment
    }

    fn bus_start(&self) -> u8 {
        self.bus_start
    }

    fn bus_end(&self) -> u8 {
        self.bus_end
    }

    unsafe fn read8(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u8 {
        unsafe { McfgEntry::read8(self, bus, dev, func, offset) }
    }

    unsafe fn read16(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u16 {
        unsafe { McfgEntry::read16(self, bus, dev, func, offset) }
    }

    unsafe fn read32(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u32 {
        unsafe { McfgEntry::read32(self, bus, dev, func, offset) }
    }

    unsafe fn write16(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u16) {
        unsafe { McfgEntry::write16(self, bus, dev, func, offset, value) };
    }

    unsafe fn write32(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u32) {
        unsafe { McfgEntry::write32(self, bus, dev, func, offset, value) };
    }
}

/// Linux x86 type-1 PCI config-space access over CF8/CFC.
///
/// This is the fallback Linux uses on conventional PC firmware when ACPI MCFG
/// is absent. It covers segment 0 and the first 256 bytes of each function's
/// config space, which is enough for PCI header enumeration and BAR sizing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyCf8ConfigAccess {
    bus_start: u8,
    bus_end: u8,
}

impl LegacyCf8ConfigAccess {
    pub const fn new(bus_start: u8, bus_end: u8) -> Self {
        Self { bus_start, bus_end }
    }
}

impl PciConfigAccess for LegacyCf8ConfigAccess {
    fn segment(&self) -> u16 {
        0
    }

    fn bus_start(&self) -> u8 {
        self.bus_start
    }

    fn bus_end(&self) -> u8 {
        self.bus_end
    }

    unsafe fn read8(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u8 {
        let _guard = LEGACY_CF8_CONFIG_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inb(PCI_CONFIG_DATA_PORT + ((offset & 3) as u16))
        }
    }

    unsafe fn read16(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u16 {
        let _guard = LEGACY_CF8_CONFIG_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16))
        }
    }

    unsafe fn read32(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u32 {
        let _guard = LEGACY_CF8_CONFIG_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inl(PCI_CONFIG_DATA_PORT)
        }
    }

    unsafe fn write16(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u16) {
        let _guard = LEGACY_CF8_CONFIG_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            outw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16), value);
        }
    }

    unsafe fn write32(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u32) {
        let _guard = LEGACY_CF8_CONFIG_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            outl(PCI_CONFIG_DATA_PORT, value);
        }
    }
}

pub const fn legacy_cf8_config_address(bus: u8, dev: u8, func: u8, offset: u16) -> u32 {
    0x8000_0000
        | (((offset as u32) & 0x0f00) << 16)
        | ((bus as u32) << 16)
        | ((((dev as u32) & 0x1f) << 3 | ((func as u32) & 0x07)) << 8)
        | ((offset as u32) & 0xfc)
}

/// Entry point — enumerate all PCI devices reachable through the supplied
/// MCFG entries and push them into `PCI_BUSES`.
///
/// In real Linux this is `pci_scan_root_bus` → `pci_scan_bus` →
/// `pci_scan_child_bus` → `pci_scan_device` → `pci_scan_slot`.
/// We flatten the bridge recursion (no bridge walk yet) and just hit all
/// 32 device × 8 function slots on each bus in each MCFG entry.
pub fn pci_enumerate(mcfg: &[McfgEntry]) {
    for entry in mcfg.iter() {
        enumerate_config_access(entry);
    }

    // Runtime binding is intentionally left to Linux-built modules calling
    // `__pci_register_driver()`, which dispatches through the Linux-shaped
    // driver core. Enumeration only publishes discovered PCI device state.
}

/// Enumerate segment-0 PCI devices through legacy x86 CF8/CFC config access.
pub fn pci_enumerate_legacy_cf8() {
    let access = LegacyCf8ConfigAccess::new(0, u8::MAX);
    enumerate_config_access(&access);
}

fn enumerate_config_access<A: PciConfigAccess + ?Sized>(access: &A) {
    for bus_n in access.bus_start()..=access.bus_end() {
        let (bus, bus_is_new) = match pci_find_bus(access.segment(), bus_n) {
            Some(bus) => (bus, false),
            None => (PciBus::new(access.segment(), bus_n), true),
        };
        let mut found = false;

        for dev_n in 0u8..32 {
            for func_n in 0u8..8 {
                // SAFETY: the selected access method owns the low-level
                // firmware/hardware validation for this config-space route.
                let vendor = unsafe { access.read16(bus_n, dev_n, func_n, PCI_VENDOR_ID) };
                if vendor == 0xFFFF {
                    // Empty slot; if func 0 is missing the whole device is absent.
                    if func_n == 0 {
                        break;
                    }
                    continue;
                }

                found = true;
                if pci_bus_has_device(&bus, dev_n, func_n) {
                    if func_n == 0 {
                        let hdr = unsafe { access.read8(bus_n, dev_n, 0, PCI_HEADER_TYPE) };
                        if hdr & 0x80 == 0 {
                            break;
                        }
                    }
                    continue;
                }

                let config_space = read_config_space(access, bus_n, dev_n, func_n);
                let device = read_config_u16(&config_space, PCI_DEVICE_ID as usize);
                let class_w = read_config_u16(&config_space, PCI_CLASS_DEVICE as usize);
                let class = (class_w >> 8) as u8;
                let subclass = (class_w & 0xFF) as u8;
                let prog_if = config_space[PCI_CLASS_PROG as usize];
                let rev = config_space[PCI_REVISION_ID as usize];

                let subsystem_vendor = read_config_u16(&config_space, PCI_SUBSYSTEM_VID as usize);
                let subsystem_device = read_config_u16(&config_space, PCI_SUBSYSTEM_ID as usize);

                let bars = read_assigned_bars(access, bus_n, dev_n, func_n);

                let pdev = PciDev::new_with_subsystem_bars_and_config(
                    access.segment(),
                    bus_n,
                    dev_n,
                    func_n,
                    vendor,
                    device,
                    class,
                    subclass,
                    prog_if,
                    rev,
                    subsystem_vendor,
                    subsystem_device,
                    bars,
                    config_space,
                );

                crate::linux_driver_abi::pci::driver::register_module_exports();
                register_linux_pci_device(
                    &pdev,
                    crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr(),
                );
                bus.devices.lock().push(pdev);

                // Multi-function: header-type bit 7.
                if func_n == 0 {
                    let hdr = unsafe { access.read8(bus_n, dev_n, 0, PCI_HEADER_TYPE) };
                    if hdr & 0x80 == 0 {
                        break;
                    } // single-function device
                }
            }
        }

        if found && bus_is_new {
            PCI_BUSES.lock().push(bus);
        }
    }
}

fn pci_find_bus(seg: u16, number: u8) -> Option<Arc<PciBus>> {
    PCI_BUSES
        .lock()
        .iter()
        .find(|bus| bus.seg == seg && bus.number == number)
        .cloned()
}

fn pci_bus_has_device(bus: &Arc<PciBus>, dev: u8, func: u8) -> bool {
    bus.devices
        .lock()
        .iter()
        .any(|pdev| pdev.dev == dev && pdev.func == func)
}

fn read_config_space<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
) -> [u8; PCI_CONFIG_SPACE_SIZE] {
    let mut config = [0u8; PCI_CONFIG_SPACE_SIZE];
    for (offset, byte) in config.iter_mut().enumerate() {
        *byte = unsafe { access.read8(bus, dev, func, offset as u16) };
    }
    config
}

fn read_config_u16(config: &[u8; PCI_CONFIG_SPACE_SIZE], offset: usize) -> u16 {
    u16::from_le_bytes([config[offset], config[offset + 1]])
}

fn read_assigned_bars<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
) -> [Option<PciBar>; 6] {
    let mut bars = [None; 6];
    let mut index = 0usize;
    let orig_cmd = unsafe { access.read16(bus, dev, func, PCI_COMMAND) };
    let decode_enabled = orig_cmd & PCI_COMMAND_DECODE_ENABLE != 0;
    if decode_enabled {
        unsafe {
            access.write16(
                bus,
                dev,
                func,
                PCI_COMMAND,
                orig_cmd & !PCI_COMMAND_DECODE_ENABLE,
            );
        }
    }

    while index < bars.len() {
        let offset = PCI_BASE_ADDRESS_0 + (index as u16 * 4);
        let raw = unsafe { access.read32(bus, dev, func, offset) };
        let next_raw = if index + 1 < bars.len() {
            unsafe { access.read32(bus, dev, func, offset + 4) }
        } else {
            0
        };

        if let Some((mut bar, consumes_next)) = PciBar::decode_config(raw, next_raw) {
            bar.size = read_bar_size(access, bus, dev, func, offset, raw, next_raw, &bar);
            bars[index] = Some(bar);
            index += if consumes_next { 2 } else { 1 };
        } else {
            index += 1;
        }
    }

    if decode_enabled {
        unsafe {
            access.write16(bus, dev, func, PCI_COMMAND, orig_cmd);
        }
    }

    bars
}

fn read_bar_size<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
    offset: u16,
    raw: u32,
    next_raw: u32,
    bar: &PciBar,
) -> u64 {
    if bar.is_64bit {
        unsafe {
            access.write32(bus, dev, func, offset, u32::MAX);
            access.write32(bus, dev, func, offset + 4, u32::MAX);
        }
        let size_lo = unsafe { access.read32(bus, dev, func, offset) } as u64;
        let size_hi = unsafe { access.read32(bus, dev, func, offset + 4) } as u64;
        unsafe {
            access.write32(bus, dev, func, offset + 4, next_raw);
            access.write32(bus, dev, func, offset, raw);
        }
        let base = ((raw as u64) & PCI_BASE_ADDRESS_MEM_MASK) | ((next_raw as u64) << 32);
        let maxbase = (size_lo & PCI_BASE_ADDRESS_MEM_MASK) | (size_hi << 32);
        return pci_size(base, maxbase, !0xf);
    }

    unsafe {
        access.write32(bus, dev, func, offset, u32::MAX);
    }
    let size = unsafe { access.read32(bus, dev, func, offset) } as u64;
    unsafe {
        access.write32(bus, dev, func, offset, raw);
    }

    if bar.is_mmio {
        pci_size(
            (raw as u64) & PCI_BASE_ADDRESS_MEM_MASK,
            size & PCI_BASE_ADDRESS_MEM_MASK,
            PCI_BASE_ADDRESS_MEM_MASK,
        )
    } else {
        pci_size(
            (raw as u64) & PCI_BASE_ADDRESS_IO_MASK,
            size & PCI_BASE_ADDRESS_IO_MASK,
            PCI_BASE_ADDRESS_IO_MASK,
        )
    }
}

fn pci_size(base: u64, maxbase: u64, mask: u64) -> u64 {
    let size = mask & maxbase;
    if size == 0 {
        return 0;
    }
    let size = size & size.wrapping_neg();
    if base == maxbase && ((base | (size - 1)) & mask) != mask {
        return 0;
    }
    size
}

/// Look up a PCI device by (segment, bus, device, function).
pub fn pci_find_device(seg: u16, bus: u8, dev: u8, func: u8) -> Option<Arc<PciDev>> {
    for b in PCI_BUSES.lock().iter() {
        if b.seg != seg || b.number != bus {
            continue;
        }
        for d in b.devices.lock().iter() {
            if d.dev == dev && d.func == func {
                return Some(d.clone());
            }
        }
    }
    None
}

/// Return a snapshot of all enumerated PCI devices.
pub fn pci_devices() -> Vec<Arc<PciDev>> {
    PCI_BUSES
        .lock()
        .iter()
        .flat_map(|b| b.devices.lock().iter().cloned().collect::<Vec<_>>())
        .collect()
}

/// Return the total number of devices enumerated across all buses.
pub fn pci_device_count() -> usize {
    pci_devices().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mcfg_yields_no_devices() {
        let before = pci_device_count();
        pci_enumerate(&[]);
        assert_eq!(pci_device_count(), before);
    }

    #[test]
    fn pci_size_matches_linux_bar_mask_decode() {
        assert_eq!(
            pci_size(0x8000_0000, 0xffff_f000, PCI_BASE_ADDRESS_MEM_MASK),
            0x1000
        );
        assert_eq!(
            pci_size(0x0000_c000, 0xffff_ff00, PCI_BASE_ADDRESS_IO_MASK),
            0x100
        );
        assert_eq!(pci_size(0, 0, PCI_BASE_ADDRESS_MEM_MASK), 0);
    }

    #[test]
    fn legacy_cf8_address_matches_linux_conf1() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/pci/direct.c"
        ));
        assert!(source.contains("#define PCI_CONF1_ADDRESS(bus, devfn, reg)"));
        assert!(source.contains("outl(PCI_CONF1_ADDRESS(bus, devfn, reg), 0xCF8);"));
        assert!(source.contains("inl(0xCFC);"));
        assert!(source.contains("outw((u16)value, 0xCFC + (reg & 2));"));

        assert_eq!(legacy_cf8_config_address(0, 0, 0, 0), 0x8000_0000);
        assert_eq!(legacy_cf8_config_address(0, 31, 2, 0x10), 0x8000_fa10);
        assert_eq!(legacy_cf8_config_address(3, 4, 5, 0x11), 0x8003_2510);
        assert_eq!(legacy_cf8_config_address(1, 2, 3, 0x234), 0x8201_1334);
    }
}
