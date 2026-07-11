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
    IORESOURCE_MEM, IORESOURCE_PREFETCH, IORESOURCE_READONLY, IORESOURCE_ROM_ENABLE,
    IORESOURCE_SIZEALIGN, LINUX_PCI_CONFIG_IO_LOCK, LinuxPciBarResource, PCI_CONFIG_SPACE_EXP_SIZE,
    PCI_CONFIG_SPACE_SIZE, PciBar, PciBus, PciConfigBackend, PciDev, register_linux_pci_device,
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
const PCI_ROM_ADDRESS: u16 = 0x30;
const PCI_ROM_ADDRESS1: u16 = 0x38;
const PCI_ROM_ADDRESS_ENABLE: u32 = 0x01;
const PCI_ROM_ADDRESS_MASK: u32 = !0x7ff;
const PCI_CAP_ID_SSVID: u8 = 0x0d;
const PCI_SSVID_VENDOR_ID: u16 = 4;
const PCI_SSVID_DEVICE_ID: u16 = 6;

trait PciConfigAccess {
    fn segment(&self) -> u16;
    fn bus_start(&self) -> u8;
    fn bus_end(&self) -> u8;
    fn backend(&self) -> PciConfigBackend;
    fn max_config_size(&self) -> usize;

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

    fn backend(&self) -> PciConfigBackend {
        PciConfigBackend::Ecam(*self)
    }

    fn max_config_size(&self) -> usize {
        PCI_CONFIG_SPACE_EXP_SIZE
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

    fn backend(&self) -> PciConfigBackend {
        PciConfigBackend::LegacyCf8
    }

    fn max_config_size(&self) -> usize {
        PCI_CONFIG_SPACE_SIZE
    }

    unsafe fn read8(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u8 {
        let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inb(PCI_CONFIG_DATA_PORT + ((offset & 3) as u16))
        }
    }

    unsafe fn read16(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u16 {
        let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16))
        }
    }

    unsafe fn read32(&self, bus: u8, dev: u8, func: u8, offset: u16) -> u32 {
        let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            inl(PCI_CONFIG_DATA_PORT)
        }
    }

    unsafe fn write16(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u16) {
        let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
        unsafe {
            outl(
                PCI_CONFIG_ADDRESS_PORT,
                legacy_cf8_config_address(bus, dev, func, offset),
            );
            outw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16), value);
        }
    }

    unsafe fn write32(&self, bus: u8, dev: u8, func: u8, offset: u16, value: u32) {
        let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
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
                let header_type = config_space[PCI_HEADER_TYPE as usize] & 0x7f;

                let (subsystem_vendor, subsystem_device) =
                    read_subsystem_ids(access, bus_n, dev_n, func_n, header_type, &config_space);

                let (bars, expansion_rom) =
                    read_assigned_resources(access, bus_n, dev_n, func_n, header_type);

                let cfg_size = pci_cfg_space_size(
                    access,
                    bus_n,
                    dev_n,
                    func_n,
                    class,
                    subclass,
                    &config_space,
                );
                let pdev = PciDev::new_with_subsystem_bars_config_and_backend(
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
                    expansion_rom,
                    config_space,
                    access.backend(),
                    cfg_size,
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

fn read_config_u32(config: &[u8; PCI_CONFIG_SPACE_SIZE], offset: usize) -> u32 {
    u32::from_le_bytes(config[offset..offset + 4].try_into().unwrap())
}

/// `pci_cfg_space_size()` / `pci_cfg_space_size_ext()` from
/// `vendor/linux/drivers/pci/probe.c` for the x86 generic configuration.
fn pci_cfg_space_size<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
    class: u8,
    subclass: u8,
    config: &[u8; PCI_CONFIG_SPACE_SIZE],
) -> usize {
    if access.max_config_size() < PCI_CONFIG_SPACE_EXP_SIZE {
        return PCI_CONFIG_SPACE_SIZE;
    }

    if class == 0x06 && subclass == 0x00 {
        return pci_cfg_space_size_ext(access, bus, dev, func);
    }
    if pci_find_capability(config, 0x10).is_some() {
        return pci_cfg_space_size_ext(access, bus, dev, func);
    }
    let Some(pos) = pci_find_capability(config, 0x07) else {
        return PCI_CONFIG_SPACE_SIZE;
    };
    if pos + 8 > PCI_CONFIG_SPACE_SIZE {
        return PCI_CONFIG_SPACE_SIZE;
    }
    let status = read_config_u32(config, pos + 4);
    if status & (0x4000_0000 | 0x8000_0000) != 0 {
        pci_cfg_space_size_ext(access, bus, dev, func)
    } else {
        PCI_CONFIG_SPACE_SIZE
    }
}

fn pci_cfg_space_size_ext<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
) -> usize {
    let status = unsafe { access.read32(bus, dev, func, PCI_CONFIG_SPACE_SIZE as u16) };
    if status == u32::MAX || pci_ext_cfg_is_aliased(access, bus, dev, func) {
        PCI_CONFIG_SPACE_SIZE
    } else {
        PCI_CONFIG_SPACE_EXP_SIZE
    }
}

/// The target vendor configuration has `CONFIG_PCI_QUIRKS=y`, so retain the
/// reachability/aliasing guard from `probe.c:pci_ext_cfg_is_aliased()`.
fn pci_ext_cfg_is_aliased<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
) -> bool {
    let header = unsafe { access.read32(bus, dev, func, PCI_VENDOR_ID) };
    for pos in (PCI_CONFIG_SPACE_SIZE..PCI_CONFIG_SPACE_EXP_SIZE).step_by(PCI_CONFIG_SPACE_SIZE) {
        let value = unsafe { access.read32(bus, dev, func, pos as u16) };
        if value != header {
            return false;
        }
    }
    true
}

/// Standard capability-list walk matching `PCI_FIND_NEXT_CAP` in
/// `vendor/linux/drivers/pci/pci.h`.
fn pci_find_capability(config: &[u8; PCI_CONFIG_SPACE_SIZE], cap: u8) -> Option<usize> {
    if read_config_u16(config, 0x06) & 0x10 == 0 {
        return None;
    }

    let pointer = match config[PCI_HEADER_TYPE as usize] & 0x7f {
        0 | 1 => 0x34,
        2 => 0x14,
        _ => return None,
    };
    let mut pos = config[pointer] as usize;
    for _ in 0..48 {
        if pos < 0x40 {
            break;
        }
        pos &= !3;
        if pos + 1 >= PCI_CONFIG_SPACE_SIZE {
            break;
        }
        let entry = read_config_u16(config, pos);
        let id = entry as u8;
        if id == u8::MAX {
            break;
        }
        if id == cap {
            return Some(pos);
        }
        pos = ((entry >> 8) as usize) & !3;
    }
    None
}

/// Type-0/type-1 subsystem ID discovery from `probe.c:pci_setup_device()`.
fn read_subsystem_ids<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
    header_type: u8,
    config: &[u8; PCI_CONFIG_SPACE_SIZE],
) -> (u16, u16) {
    match header_type {
        0 => {
            return (
                read_config_u16(config, PCI_SUBSYSTEM_VID as usize),
                read_config_u16(config, PCI_SUBSYSTEM_ID as usize),
            );
        }
        1 => {}
        _ => return (0, 0),
    }

    let Some(pos) = pci_find_capability(config, PCI_CAP_ID_SSVID) else {
        return (0, 0);
    };
    if pos + PCI_SSVID_DEVICE_ID as usize + 2 > PCI_CONFIG_SPACE_SIZE {
        return (0, 0);
    }

    unsafe {
        (
            access.read16(bus, dev, func, pos as u16 + PCI_SSVID_VENDOR_ID),
            access.read16(bus, dev, func, pos as u16 + PCI_SSVID_DEVICE_ID),
        )
    }
}

fn read_assigned_resources<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
    header_type: u8,
) -> ([Option<PciBar>; 6], Option<LinuxPciBarResource>) {
    let mut bars = [None; 6];
    let bar_count = pci_header_bar_count(header_type);
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

    while index < bar_count {
        let offset = PCI_BASE_ADDRESS_0 + (index as u16 * 4);
        let raw = unsafe { access.read32(bus, dev, func, offset) };
        let next_raw = if index + 1 < bar_count {
            unsafe { access.read32(bus, dev, func, offset + 4) }
        } else {
            0
        };

        if let Some((mut bar, consumes_next)) = PciBar::decode_config(raw, next_raw) {
            if consumes_next && index + 1 >= bar_count {
                index += 1;
                continue;
            }
            bar.size = read_bar_size(access, bus, dev, func, offset, raw, next_raw, &bar);
            bars[index] = Some(bar);
            index += if consumes_next { 2 } else { 1 };
        } else {
            index += 1;
        }
    }

    let expansion_rom = read_expansion_rom(access, bus, dev, func, header_type);

    if decode_enabled {
        unsafe {
            access.write16(bus, dev, func, PCI_COMMAND, orig_cmd);
        }
    }

    (bars, expansion_rom)
}

/// Expansion-ROM half of `probe.c:pci_read_bases()`. The all-ones sizing
/// value keeps the ROM-enable bit clear, and the original register is restored
/// before the resource is published.
fn read_expansion_rom<A: PciConfigAccess + ?Sized>(
    access: &A,
    bus: u8,
    dev: u8,
    func: u8,
    header_type: u8,
) -> Option<LinuxPciBarResource> {
    let offset = match header_type {
        0 => PCI_ROM_ADDRESS,
        1 => PCI_ROM_ADDRESS1,
        _ => return None,
    };

    let original = unsafe { access.read32(bus, dev, func, offset) };
    unsafe {
        access.write32(bus, dev, func, offset, PCI_ROM_ADDRESS_MASK);
    }
    let mut size_mask = unsafe { access.read32(bus, dev, func, offset) };
    unsafe {
        access.write32(bus, dev, func, offset, original);
    }
    let mut raw = unsafe { access.read32(bus, dev, func, offset) };

    if raw == u32::MAX {
        raw = 0;
    }
    if size_mask == u32::MAX {
        size_mask = 0;
    }
    let base = (raw & PCI_ROM_ADDRESS_MASK) as u64;
    let size = pci_size(
        base,
        (size_mask & PCI_ROM_ADDRESS_MASK) as u64,
        PCI_ROM_ADDRESS_MASK as u64,
    );
    if size == 0 || base == 0 {
        // Linux can retain an unassigned ROM resource and allocate/claim it
        // later. Lupos has no equivalent resource allocator yet, so exposing
        // address zero to a driver would be unsafe; fail closed after the
        // non-destructive sizing probe.
        return None;
    }

    let mut flags =
        IORESOURCE_MEM | IORESOURCE_PREFETCH | IORESOURCE_READONLY | IORESOURCE_SIZEALIGN;
    if raw & PCI_ROM_ADDRESS_ENABLE != 0 {
        flags |= IORESOURCE_ROM_ENABLE;
    }
    Some(LinuxPciBarResource {
        start: base,
        len: size,
        flags,
    })
}

const fn pci_header_bar_count(header_type: u8) -> usize {
    match header_type & 0x7f {
        0 => 6,
        1 => 2,
        2 => 1,
        _ => 0,
    }
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
    fn bar_probe_count_respects_pci_header_type() {
        assert_eq!(pci_header_bar_count(0x00), 6);
        assert_eq!(pci_header_bar_count(0x80), 6);
        assert_eq!(pci_header_bar_count(0x01), 2);
        assert_eq!(pci_header_bar_count(0x81), 2);
        assert_eq!(pci_header_bar_count(0x02), 1);
        assert_eq!(pci_header_bar_count(0x7f), 0);
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
