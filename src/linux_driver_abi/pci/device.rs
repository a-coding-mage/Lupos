//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci
//! test-origin: linux:vendor/linux/drivers/pci
//! `struct pci_dev` and `struct pci_bus` — `include/linux/pci.h:351,700`.
//!
//! PCI configuration space layout mirrors the PCI 3.0 spec §6.1.
//! BAR decode mirrors `drivers/pci/probe.c:pci_read_bases`.

extern crate alloc;

use core::ffi::{c_char, c_void};
use core::mem::size_of;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

#[cfg(not(test))]
use crate::arch::x86::include::asm::io::{outb, outl, outw};
#[cfg(not(test))]
use crate::arch::x86::pci::early::{PCI_CONFIG_ADDRESS_PORT, PCI_CONFIG_DATA_PORT};
use crate::linux_driver_abi::base::{
    LinuxBusType, LinuxDevice, LinuxListHead, linux_device_register,
};

pub const PCI_CONFIG_SPACE_SIZE: usize = 256;
pub const PCI_STD_NUM_BARS: usize = 6;
pub const PCI_DEVICE_RESOURCE_COUNT: usize = 12;
pub const LINUX_PCI_DEV_DEVICE_OFFSET: usize = 0xb8;
pub const LINUX_PCI_DEV_ERROR_STATE_OFFSET: usize = LINUX_PCI_DEV_DEVICE_OFFSET - 4;
pub const LINUX_PCI_DEV_CFG_SIZE_OFFSET: usize = 0x268;
pub const LINUX_PCI_DEV_IRQ_OFFSET: usize = 0x26c;
pub const LINUX_PCI_DEV_RESOURCE_OFFSET: usize = 0x270;
pub const LINUX_PCI_CHANNEL_IO_NORMAL: u32 = 1;

/// PCI class code for host bridge (`vendor/linux/include/linux/pci_ids.h`).
pub const PCI_CLASS_BRIDGE_HOST: u8 = 0x06;
pub const PCI_CLASS_BRIDGE_PCI: u8 = 0x06;

pub const IORESOURCE_IO: usize = 0x0000_0100;
pub const IORESOURCE_MEM: usize = 0x0000_0200;
pub const IORESOURCE_PREFETCH: usize = 0x0000_2000;
pub const IORESOURCE_MEM_64: usize = 0x0010_0000;

/// PCI Base Address Register decoded form.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PciBar {
    /// Physical base address of the BAR window.
    pub base: u64,
    /// Size of the BAR window in bytes. Zero means firmware sizing has not
    /// been probed yet; the assigned base address is still usable.
    pub size: u64,
    /// `true` for MMIO, `false` for I/O port.
    pub is_mmio: bool,
    /// `true` for 64-bit MMIO BAR.
    pub is_64bit: bool,
    /// `true` if marked prefetchable in config space.
    pub prefetchable: bool,
}

/// `struct resource` - `vendor/linux/include/linux/ioport.h:22`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxResource {
    pub start: u64,
    pub end: u64,
    pub name: *const c_char,
    pub flags: usize,
    pub desc: usize,
    pub parent: *mut LinuxResource,
    pub sibling: *mut LinuxResource,
    pub child: *mut LinuxResource,
}

/// BAR resource view consumed by Linux-built PCI driver modules.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LinuxPciBarResource {
    pub start: u64,
    pub len: u64,
    pub flags: usize,
}

impl LinuxPciBarResource {
    pub const fn end(self) -> u64 {
        self.start.saturating_add(self.len.saturating_sub(1))
    }
}

/// Per-raw-`struct pci_dev` ABI state used by helper exports.
///
/// The Rust PCI enumerator still owns discovery. Linux-built modules only see
/// opaque `struct pci_dev *` pointers, so helpers such as
/// `pci_read_config_dword()` and `pci_iomap_range()` consult this registry
/// instead of dereferencing a guessed Rust layout.
#[derive(Clone, Copy)]
pub struct LinuxPciDeviceAbiState {
    pub config_space: [u8; PCI_CONFIG_SPACE_SIZE],
    pub bars: [Option<LinuxPciBarResource>; PCI_STD_NUM_BARS],
}

/// Prefix of `struct pci_dev` through the resource array.
///
/// Source: `vendor/linux/include/linux/pci.h:351`. The important ABI invariant
/// for Linux-built PCI modules is that the embedded `struct device` lives at
/// byte offset `0xb8`; the vendor-built `virtio_pci.ko` uses that exact
/// `container_of()` offset when moving between `struct device` and
/// `struct pci_dev`.
#[repr(C)]
pub struct LinuxPciDev {
    pub bus_list: LinuxListHead,
    pub bus: *mut c_void,
    pub subordinate: *mut c_void,
    pub sysdata: *mut c_void,
    pub procent: *mut c_void,
    pub slot: *mut c_void,
    pub devfn: u32,
    pub vendor: u16,
    pub device: u16,
    pub subsystem_vendor: u16,
    pub subsystem_device: u16,
    pub class: u32,
    pub revision: u8,
    pub hdr_type: u8,
    pub _pad_to_driver: [u8; 0x68 - 0x4a],
    pub driver: *mut c_void,
    pub dma_mask: u64,
    pub msi_addr_mask: u64,
    pub _pad_to_error_state: [u8; LINUX_PCI_DEV_ERROR_STATE_OFFSET - 0x80],
    pub error_state: u32,
    pub dev: LinuxDevice,
    pub _pad_after_device_prefix: [u8; LINUX_PCI_DEV_CFG_SIZE_OFFSET
        - (LINUX_PCI_DEV_DEVICE_OFFSET + size_of::<LinuxDevice>())],
    pub cfg_size: i32,
    pub irq: u32,
    pub resource: [LinuxResource; PCI_DEVICE_RESOURCE_COUNT],
    pub name: [u8; 32],
}

impl PciBar {
    /// Decode an assigned BAR value from PCI config space.
    ///
    /// This mirrors the non-sizing half of Linux's `pci_read_bases()`: it
    /// records the firmware-assigned address while leaving size as zero until
    /// the destructive all-ones sizing probe is wired.
    pub fn decode_config(raw: u32, next_raw: u32) -> Option<(Self, bool)> {
        if raw == 0 || raw == u32::MAX {
            return None;
        }

        if raw & 0x1 != 0 {
            return Some((
                Self {
                    base: (raw & !0x3) as u64,
                    size: 0,
                    is_mmio: false,
                    is_64bit: false,
                    prefetchable: false,
                },
                false,
            ));
        }

        let bar_type = (raw >> 1) & 0x3;
        if bar_type == 0x1 {
            return None;
        }

        let is_64bit = bar_type == 0x2;
        let base_lo = (raw & !0xf) as u64;
        let base = if is_64bit {
            base_lo | ((next_raw as u64) << 32)
        } else {
            base_lo
        };

        Some((
            Self {
                base,
                size: 0,
                is_mmio: true,
                is_64bit,
                prefetchable: raw & 0x8 != 0,
            },
            is_64bit,
        ))
    }
}

/// `struct pci_dev` — `include/linux/pci.h:351`.
///
/// Represents one PCI function. We carry the segment/bus/device/function
/// quadruple, the vendor/device IDs, class code, the 6 BARs, and a link
/// back to the containing bus.
pub struct PciDev {
    pub seg: u16,
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor: u16,
    pub device: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision: u8,
    pub subsystem_vendor: u16,
    pub subsystem_device: u16,
    pub bars: [Option<PciBar>; 6],
    pub config_space: [u8; PCI_CONFIG_SPACE_SIZE],
    pub irq: Mutex<Option<u32>>,
    pub name: String,
}

impl PciDev {
    pub fn new(
        seg: u16,
        bus: u8,
        dev: u8,
        func: u8,
        vendor: u16,
        device: u16,
        class: u8,
        subclass: u8,
        prog_if: u8,
        revision: u8,
    ) -> Arc<Self> {
        Self::new_with_subsystem(
            seg, bus, dev, func, vendor, device, class, subclass, prog_if, revision, 0, 0,
        )
    }

    pub fn new_with_subsystem(
        seg: u16,
        bus: u8,
        dev: u8,
        func: u8,
        vendor: u16,
        device: u16,
        class: u8,
        subclass: u8,
        prog_if: u8,
        revision: u8,
        subsystem_vendor: u16,
        subsystem_device: u16,
    ) -> Arc<Self> {
        Self::new_with_subsystem_and_bars(
            seg,
            bus,
            dev,
            func,
            vendor,
            device,
            class,
            subclass,
            prog_if,
            revision,
            subsystem_vendor,
            subsystem_device,
            [None; 6],
        )
    }

    pub fn new_with_subsystem_and_bars(
        seg: u16,
        bus: u8,
        dev: u8,
        func: u8,
        vendor: u16,
        device: u16,
        class: u8,
        subclass: u8,
        prog_if: u8,
        revision: u8,
        subsystem_vendor: u16,
        subsystem_device: u16,
        bars: [Option<PciBar>; 6],
    ) -> Arc<Self> {
        let config_space = default_config_space(
            vendor,
            device,
            class,
            subclass,
            prog_if,
            revision,
            subsystem_vendor,
            subsystem_device,
        );
        Self::new_with_subsystem_bars_and_config(
            seg,
            bus,
            dev,
            func,
            vendor,
            device,
            class,
            subclass,
            prog_if,
            revision,
            subsystem_vendor,
            subsystem_device,
            bars,
            config_space,
        )
    }

    pub fn new_with_subsystem_bars_and_config(
        seg: u16,
        bus: u8,
        dev: u8,
        func: u8,
        vendor: u16,
        device: u16,
        class: u8,
        subclass: u8,
        prog_if: u8,
        revision: u8,
        subsystem_vendor: u16,
        subsystem_device: u16,
        bars: [Option<PciBar>; 6],
        config_space: [u8; PCI_CONFIG_SPACE_SIZE],
    ) -> Arc<Self> {
        let name = alloc::format!("{:04x}:{:02x}:{:02x}.{}", seg, bus, dev, func);
        Arc::new(Self {
            seg,
            bus,
            dev,
            func,
            vendor,
            device,
            class,
            subclass,
            prog_if,
            revision,
            subsystem_vendor,
            subsystem_device,
            bars,
            config_space,
            irq: Mutex::new(None),
            name,
        })
    }
}

impl LinuxPciDeviceAbiState {
    pub fn from_pci_dev(dev: &PciDev) -> Self {
        let mut bars = [None; PCI_STD_NUM_BARS];
        for (index, bar) in dev.bars.iter().enumerate().take(PCI_STD_NUM_BARS) {
            if let Some(bar) = bar {
                let mut flags = if bar.is_mmio {
                    IORESOURCE_MEM
                } else {
                    IORESOURCE_IO
                };
                if bar.prefetchable {
                    flags |= IORESOURCE_PREFETCH;
                }
                if bar.is_64bit {
                    flags |= IORESOURCE_MEM_64;
                }
                bars[index] = Some(LinuxPciBarResource {
                    start: bar.base,
                    len: bar.size,
                    flags,
                });
            }
        }

        Self {
            config_space: dev.config_space,
            bars,
        }
    }
}

impl LinuxPciDev {
    pub fn from_pci_dev(dev: &PciDev, bus_type: *const LinuxBusType) -> Self {
        let state = LinuxPciDeviceAbiState::from_pci_dev(dev);
        let mut resource = [LinuxResource {
            start: 0,
            end: 0,
            name: core::ptr::null(),
            flags: 0,
            desc: 0,
            parent: core::ptr::null_mut(),
            sibling: core::ptr::null_mut(),
            child: core::ptr::null_mut(),
        }; PCI_DEVICE_RESOURCE_COUNT];
        for (idx, bar) in state.bars.iter().enumerate() {
            if let Some(bar) = bar {
                resource[idx].start = bar.start;
                resource[idx].end = bar.end();
                resource[idx].flags = bar.flags;
            }
        }

        Self {
            bus_list: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            bus: core::ptr::null_mut(),
            subordinate: core::ptr::null_mut(),
            sysdata: core::ptr::null_mut(),
            procent: core::ptr::null_mut(),
            slot: core::ptr::null_mut(),
            devfn: ((dev.dev as u32) << 3) | dev.func as u32,
            vendor: dev.vendor,
            device: dev.device,
            subsystem_vendor: dev.subsystem_vendor,
            subsystem_device: dev.subsystem_device,
            class: ((dev.class as u32) << 16) | ((dev.subclass as u32) << 8) | dev.prog_if as u32,
            revision: dev.revision,
            hdr_type: dev.config_space[0x0e] & 0x7f,
            _pad_to_driver: [0; 0x68 - 0x4a],
            driver: core::ptr::null_mut(),
            dma_mask: u32::MAX as u64,
            msi_addr_mask: u64::MAX,
            _pad_to_error_state: [0; LINUX_PCI_DEV_ERROR_STATE_OFFSET - 0x80],
            error_state: LINUX_PCI_CHANNEL_IO_NORMAL,
            dev: unsafe { core::mem::zeroed::<LinuxDevice>() },
            _pad_after_device_prefix: [0; LINUX_PCI_DEV_CFG_SIZE_OFFSET
                - (LINUX_PCI_DEV_DEVICE_OFFSET + size_of::<LinuxDevice>())],
            cfg_size: PCI_CONFIG_SPACE_SIZE as i32,
            irq: dev.config_space[0x3c] as u32,
            resource,
            name: [0; 32],
        }
        .with_device_fields(dev, bus_type)
    }

    fn with_device_fields(mut self, dev: &PciDev, bus_type: *const LinuxBusType) -> Self {
        let bytes = dev.name.as_bytes();
        let len = core::cmp::min(bytes.len(), self.name.len() - 1);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0;
        self.dev.init_name = self.name.as_ptr().cast::<c_char>();
        self.dev.bus = bus_type;
        self
    }
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciDevice {
    dev: usize,
    state: LinuxPciDeviceAbiState,
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciObject {
    seg: u16,
    bus: u8,
    dev: u8,
    func: u8,
    raw: usize,
}

lazy_static! {
    static ref LINUX_PCI_DEVICE_STATES: Mutex<Vec<RegisteredLinuxPciDevice>> =
        Mutex::new(Vec::new());
    static ref LINUX_PCI_OBJECTS: Mutex<Vec<RegisteredLinuxPciObject>> = Mutex::new(Vec::new());
}

#[cfg(not(test))]
static LINUX_PCI_CONFIG_IO_LOCK: Mutex<()> = Mutex::new(());

pub unsafe fn linux_pci_dev_from_device(dev: *const LinuxDevice) -> *mut LinuxPciDev {
    if dev.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        dev.cast::<u8>()
            .sub(LINUX_PCI_DEV_DEVICE_OFFSET)
            .cast_mut()
            .cast::<LinuxPciDev>()
    }
}

pub fn registered_linux_pci_device_count() -> usize {
    LINUX_PCI_OBJECTS.lock().len()
}

#[cfg(test)]
pub fn registered_linux_pci_bound_device_count() -> usize {
    LINUX_PCI_OBJECTS
        .lock()
        .iter()
        .filter(|entry| {
            let raw = entry.raw as *const LinuxPciDev;
            unsafe { !(*raw).driver.is_null() || !(*raw).dev.driver.is_null() }
        })
        .count()
}

pub fn linux_pci_raw_device_for_slot(seg: u16, bus: u8, dev: u8, func: u8) -> *mut LinuxPciDev {
    LINUX_PCI_OBJECTS
        .lock()
        .iter()
        .find(|entry| {
            entry.seg == seg && entry.bus == bus && entry.dev == dev && entry.func == func
        })
        .map(|entry| entry.raw as *mut LinuxPciDev)
        .unwrap_or(core::ptr::null_mut())
}

pub fn linux_pci_slot_for_raw(dev: *const c_void) -> Option<(u16, u8, u8, u8)> {
    if dev.is_null() {
        return None;
    }
    LINUX_PCI_OBJECTS
        .lock()
        .iter()
        .find(|entry| entry.raw == dev as usize)
        .map(|entry| (entry.seg, entry.bus, entry.dev, entry.func))
}

#[cfg(any(
    test,
    feature = "test-initramfs-rootfs",
    feature = "test-disk-root-remount"
))]
pub fn linux_pci_device_bound(seg: u16, bus: u8, dev: u8, func: u8) -> bool {
    let raw = linux_pci_raw_device_for_slot(seg, bus, dev, func);
    if raw.is_null() {
        return false;
    }
    unsafe { !(*raw).driver.is_null() || !(*raw).dev.driver.is_null() }
}

pub fn register_linux_pci_device(dev: &PciDev, bus_type: *const LinuxBusType) -> *mut LinuxPciDev {
    if bus_type.is_null() {
        return core::ptr::null_mut();
    }
    let existing = linux_pci_raw_device_for_slot(dev.seg, dev.bus, dev.dev, dev.func);
    if !existing.is_null() {
        return existing;
    }

    let mut raw = Box::new(LinuxPciDev::from_pci_dev(dev, bus_type));
    let raw_ptr = (&mut *raw) as *mut LinuxPciDev;
    unsafe {
        (*raw_ptr).dev.init_name = (*raw_ptr).name.as_ptr().cast::<c_char>();
    }

    let state = LinuxPciDeviceAbiState::from_pci_dev(dev);
    register_linux_pci_device_state(raw_ptr.cast_const().cast(), state);
    let ret = unsafe { linux_device_register(core::ptr::addr_of_mut!((*raw_ptr).dev)) };
    if ret != 0 {
        unregister_linux_pci_device_state(raw_ptr.cast_const().cast());
        return core::ptr::null_mut();
    }

    LINUX_PCI_OBJECTS.lock().push(RegisteredLinuxPciObject {
        seg: dev.seg,
        bus: dev.bus,
        dev: dev.dev,
        func: dev.func,
        raw: Box::into_raw(raw) as usize,
    });
    raw_ptr
}

pub fn register_linux_pci_device_state(dev: *const c_void, state: LinuxPciDeviceAbiState) {
    if dev.is_null() {
        return;
    }
    let mut states = LINUX_PCI_DEVICE_STATES.lock();
    if let Some(existing) = states.iter_mut().find(|entry| entry.dev == dev as usize) {
        existing.state = state;
    } else {
        states.push(RegisteredLinuxPciDevice {
            dev: dev as usize,
            state,
        });
    }
}

pub fn unregister_linux_pci_device_state(dev: *const c_void) {
    let dev = dev as usize;
    LINUX_PCI_DEVICE_STATES
        .lock()
        .retain(|entry| entry.dev != dev);
}

pub fn linux_pci_device_state(dev: *const c_void) -> Option<LinuxPciDeviceAbiState> {
    LINUX_PCI_DEVICE_STATES
        .lock()
        .iter()
        .find(|entry| entry.dev == dev as usize)
        .map(|entry| entry.state)
}

pub fn linux_pci_config_read(dev: *const c_void, offset: usize, width: usize) -> Option<u32> {
    let state = linux_pci_device_state(dev)?;
    if width == 0 || offset.checked_add(width)? > PCI_CONFIG_SPACE_SIZE {
        return None;
    }
    let bytes = &state.config_space[offset..offset + width];
    Some(match width {
        1 => bytes[0] as u32,
        2 => u16::from_le_bytes([bytes[0], bytes[1]]) as u32,
        4 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        _ => return None,
    })
}

pub fn linux_pci_config_write(dev: *const c_void, offset: usize, width: usize, value: u32) -> bool {
    if dev.is_null()
        || !matches!(width, 1 | 2 | 4)
        || offset
            .checked_add(width)
            .is_none_or(|end| end > PCI_CONFIG_SPACE_SIZE)
    {
        return false;
    }

    let mut states = LINUX_PCI_DEVICE_STATES.lock();
    let Some(entry) = states.iter_mut().find(|entry| entry.dev == dev as usize) else {
        return false;
    };
    let bytes = value.to_le_bytes();
    entry.state.config_space[offset..offset + width].copy_from_slice(&bytes[..width]);
    drop(states);

    write_pci_config_hardware(dev, offset, width, value);
    true
}

#[cfg(not(test))]
fn write_pci_config_hardware(dev: *const c_void, offset: usize, width: usize, value: u32) {
    if offset >= PCI_CONFIG_SPACE_SIZE {
        return;
    }
    let Some((seg, bus, slot, func)) = linux_pci_slot_for_raw(dev) else {
        return;
    };
    if seg != 0 {
        return;
    }

    let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
    let address = crate::arch::x86::pci::cf8_address(bus, slot, func, offset as u8);
    unsafe {
        outl(PCI_CONFIG_ADDRESS_PORT, address);
        match width {
            1 => outb(PCI_CONFIG_DATA_PORT + ((offset & 3) as u16), value as u8),
            2 => outw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16), value as u16),
            4 => outl(PCI_CONFIG_DATA_PORT, value),
            _ => {}
        }
    }
}

#[cfg(test)]
fn write_pci_config_hardware(_dev: *const c_void, _offset: usize, _width: usize, _value: u32) {}

pub fn linux_pci_bar_resource(dev: *const c_void, bar: usize) -> Option<LinuxPciBarResource> {
    if bar >= PCI_STD_NUM_BARS {
        return None;
    }
    linux_pci_device_state(dev)?.bars[bar]
}

fn default_config_space(
    vendor: u16,
    device: u16,
    class: u8,
    subclass: u8,
    prog_if: u8,
    revision: u8,
    subsystem_vendor: u16,
    subsystem_device: u16,
) -> [u8; PCI_CONFIG_SPACE_SIZE] {
    let mut config = [0u8; PCI_CONFIG_SPACE_SIZE];
    write_u16(&mut config, 0x00, vendor);
    write_u16(&mut config, 0x02, device);
    config[0x08] = revision;
    config[0x09] = prog_if;
    config[0x0a] = subclass;
    config[0x0b] = class;
    write_u16(&mut config, 0x2c, subsystem_vendor);
    write_u16(&mut config, 0x2e, subsystem_device);
    config
}

fn write_u16(config: &mut [u8; PCI_CONFIG_SPACE_SIZE], offset: usize, value: u16) {
    config[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

/// `struct pci_bus` — `include/linux/pci.h:700`.
pub struct PciBus {
    pub seg: u16,
    pub number: u8,
    pub devices: Mutex<Vec<Arc<PciDev>>>,
}

impl PciBus {
    pub fn new(seg: u16, number: u8) -> Arc<Self> {
        Arc::new(Self {
            seg,
            number,
            devices: Mutex::new(Vec::new()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::base::linux_device_registered;
    use core::mem::offset_of;

    #[test]
    fn decodes_32bit_mmio_bar_from_config_value() {
        let (bar, consumes_next) =
            PciBar::decode_config(0xfebd_0008, 0).expect("assigned mmio bar");

        assert_eq!(bar.base, 0xfebd_0000);
        assert!(bar.is_mmio);
        assert!(!bar.is_64bit);
        assert!(bar.prefetchable);
        assert_eq!(bar.size, 0);
        assert!(!consumes_next);
    }

    #[test]
    fn decodes_64bit_mmio_bar_from_config_pair() {
        let (bar, consumes_next) =
            PciBar::decode_config(0x0000_1004, 0x0000_0001).expect("assigned 64-bit bar");

        assert_eq!(bar.base, 0x1_0000_1000);
        assert!(bar.is_mmio);
        assert!(bar.is_64bit);
        assert!(consumes_next);
    }

    #[test]
    fn decodes_io_bar_from_config_value() {
        let (bar, consumes_next) = PciBar::decode_config(0x0000_c001, 0).expect("io bar");

        assert_eq!(bar.base, 0x0000_c000);
        assert!(!bar.is_mmio);
        assert!(!bar.is_64bit);
        assert!(!consumes_next);
    }

    #[test]
    fn pci_dev_default_config_space_mirrors_identity_fields() {
        let dev = PciDev::new_with_subsystem(
            0, 0, 1, 0, 0x1af4, 0x1042, 0x01, 0x00, 0x00, 0x05, 0x1af4, 0x0002,
        );

        assert_eq!(&dev.config_space[0x00..0x02], &0x1af4u16.to_le_bytes());
        assert_eq!(&dev.config_space[0x02..0x04], &0x1042u16.to_le_bytes());
        assert_eq!(dev.config_space[0x08], 0x05);
        assert_eq!(dev.config_space[0x0b], 0x01);
        assert_eq!(&dev.config_space[0x2e..0x30], &0x0002u16.to_le_bytes());
    }

    #[test]
    fn linux_pci_config_write_updates_registered_state() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        register_linux_pci_device_state(dev, state);

        assert!(linux_pci_config_write(dev, 0x04, 2, 0x0006));
        assert_eq!(linux_pci_config_read(dev, 0x04, 2), Some(0x0006));
        assert!(linux_pci_config_write(dev, 0x10, 4, 0x1234_5678));
        assert_eq!(linux_pci_config_read(dev, 0x10, 4), Some(0x1234_5678));
        assert!(!linux_pci_config_write(
            dev,
            PCI_CONFIG_SPACE_SIZE - 1,
            2,
            0
        ));

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn linux_pci_dev_layout_pins_embedded_device_offset() {
        assert_eq!(offset_of!(LinuxPciDev, bus_list), 0);
        assert_eq!(offset_of!(LinuxPciDev, bus), 16);
        assert_eq!(offset_of!(LinuxPciDev, devfn), 56);
        assert_eq!(offset_of!(LinuxPciDev, vendor), 60);
        assert_eq!(offset_of!(LinuxPciDev, device), 62);
        assert_eq!(offset_of!(LinuxPciDev, subsystem_vendor), 64);
        assert_eq!(offset_of!(LinuxPciDev, subsystem_device), 66);
        assert_eq!(offset_of!(LinuxPciDev, class), 68);
        assert_eq!(offset_of!(LinuxPciDev, revision), 72);
        assert_eq!(offset_of!(LinuxPciDev, hdr_type), 73);
        assert_eq!(offset_of!(LinuxPciDev, driver), 0x68);
        assert_eq!(
            offset_of!(LinuxPciDev, error_state),
            LINUX_PCI_DEV_ERROR_STATE_OFFSET
        );
        assert_eq!(offset_of!(LinuxPciDev, dev), LINUX_PCI_DEV_DEVICE_OFFSET);
        assert_eq!(
            offset_of!(LinuxPciDev, cfg_size),
            LINUX_PCI_DEV_CFG_SIZE_OFFSET
        );
        assert_eq!(offset_of!(LinuxPciDev, irq), LINUX_PCI_DEV_IRQ_OFFSET);
        assert_eq!(
            offset_of!(LinuxPciDev, resource),
            LINUX_PCI_DEV_RESOURCE_OFFSET
        );
    }

    #[test]
    fn linux_pci_raw_device_resources_use_vendor_offsets() {
        crate::linux_driver_abi::pci::driver::register_module_exports();
        let mut bars = [None; 6];
        bars[0] = Some(PciBar {
            base: 0x8000_0000,
            size: 0x1000,
            is_mmio: true,
            is_64bit: false,
            prefetchable: false,
        });
        let pdev = PciDev::new_with_subsystem_and_bars(
            0, 0, 31, 0, 0x1af4, 0x1042, 0x01, 0x00, 0x00, 1, 0x1af4, 0x1100, bars,
        );
        let raw = register_linux_pci_device(
            &pdev,
            crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr(),
        );

        assert!(!raw.is_null());
        unsafe {
            assert_eq!((*raw).resource[0].start, 0x8000_0000);
            assert_eq!((*raw).resource[0].end, 0x8000_0fff);
            assert_eq!((*raw).resource[0].flags & IORESOURCE_MEM, IORESOURCE_MEM);
        }
    }

    #[test]
    fn linux_pci_raw_device_registration_uses_vendor_shape() {
        crate::linux_driver_abi::pci::driver::register_module_exports();
        let pdev = PciDev::new_with_subsystem(
            0, 0, 30, 0, 0x1af4, 0x1042, 0x01, 0x00, 0x00, 1, 0x1af4, 0x1100,
        );
        let before = registered_linux_pci_device_count();
        let raw = register_linux_pci_device(
            &pdev,
            crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr(),
        );

        assert!(!raw.is_null());
        unsafe {
            assert_eq!((*raw).vendor, 0x1af4);
            assert_eq!((*raw).device, 0x1042);
            assert_eq!((*raw).class, 0x010000);
            assert_eq!((*raw).error_state, LINUX_PCI_CHANNEL_IO_NORMAL);
            assert_eq!((*raw).cfg_size, PCI_CONFIG_SPACE_SIZE as i32);
            assert_eq!(linux_pci_dev_from_device(&(*raw).dev), raw);
            assert!(linux_device_registered(&(*raw).dev));
            assert!(linux_pci_device_state(raw.cast_const().cast()).is_some());
        }
        assert_eq!(linux_pci_raw_device_for_slot(0, 0, 30, 0), raw);
        assert_eq!(registered_linux_pci_device_count(), before + 1);
    }
}
