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

use crate::arch::x86::include::asm::io::{inb, inl, inw, outb, outl, outw};
use crate::arch::x86::pci::early::{PCI_CONFIG_ADDRESS_PORT, PCI_CONFIG_DATA_PORT};
use crate::linux_driver_abi::base::{
    LinuxBusType, LinuxDevice, LinuxListHead, linux_device_register,
};
use crate::linux_driver_abi::pci::ecam::McfgEntry;

pub const PCI_CONFIG_SPACE_SIZE: usize = 256;
pub const PCI_CONFIG_SPACE_EXP_SIZE: usize = 4096;
pub const PCI_STD_NUM_BARS: usize = 6;
/// `DEVICE_COUNT_RESOURCE` with the staged x86_64 module configuration
/// (`CONFIG_PCI_IOV=n`): six BARs, one ROM, and four bridge windows.
pub const PCI_DEVICE_RESOURCE_COUNT: usize = 11;
pub const LINUX_PCI_DEV_DRIVER_OFFSET: usize = 0x78;
pub const LINUX_PCI_DEV_ROM_BASE_REG_OFFSET: usize = 0x6a;
pub const LINUX_PCI_DEV_PIN_OFFSET: usize = 0x6b;
pub const LINUX_PCI_DEV_DEVICE_OFFSET: usize = 0xc8;
pub const LINUX_PCI_DEV_ERROR_STATE_OFFSET: usize = LINUX_PCI_DEV_DEVICE_OFFSET - 4;
pub const LINUX_PCI_DEV_CFG_SIZE_OFFSET: usize = 0x3c0;
pub const LINUX_PCI_DEV_IRQ_OFFSET: usize = 0x3c4;
pub const LINUX_PCI_DEV_RESOURCE_OFFSET: usize = 0x3c8;
pub const LINUX_PCI_DEV_DRIVER_EXCLUSIVE_RESOURCE_OFFSET: usize = 0x688;
pub const LINUX_PCI_DEV_ABI_SIZE: usize = 0x798;
pub const LINUX_PCI_CHANNEL_IO_NORMAL: u32 = 1;
pub const LINUX_PCI_BUS_DEVICE_OFFSET: usize = 0x118;
pub const LINUX_PCI_BUS_FLAGS_OFFSET: usize = 0x410;
pub const LINUX_PCI_BUS_ABI_SIZE: usize = 0x418;

const LINUX_DEVICE_DMA_MASK_OFFSET: usize = 584;
const LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET: usize = 592;
const LINUX_DEVICE_DMA_PARMS_OFFSET: usize = 616;
static PCI_EXCLUSIVE_RESOURCE_NAME: [u8; 14] = *b"PCI Exclusive\0";
static PCI_BUSN_RESOURCE_NAME: [u8; 9] = *b"PCI busn\0";

/// PCI class code for host bridge (`vendor/linux/include/linux/pci_ids.h`).
pub const PCI_CLASS_BRIDGE_HOST: u8 = 0x06;
pub const PCI_CLASS_BRIDGE_PCI: u8 = 0x04;

pub const IORESOURCE_IO: usize = 0x0000_0100;
pub const IORESOURCE_MEM: usize = 0x0000_0200;
pub const IORESOURCE_BUS: usize = 0x0000_1000;
pub const IORESOURCE_TYPE_BITS: usize = 0x0000_1f00;
pub const IORESOURCE_PREFETCH: usize = 0x0000_2000;
pub const IORESOURCE_MEM_64: usize = 0x0010_0000;
pub const IORESOURCE_READONLY: usize = 0x0000_4000;
pub const IORESOURCE_SIZEALIGN: usize = 0x0004_0000;
pub const IORESOURCE_ROM_ENABLE: usize = 0x0000_0001;
pub const IORESOURCE_PCI_FIXED: usize = 0x0000_0010;
pub const IORESOURCE_DISABLED: usize = 0x1000_0000;
pub const IORESOURCE_UNSET: usize = 0x2000_0000;

/// Config-space route retained from PCI enumeration.
///
/// This is the Rust representation of the selected x86 `raw_pci_ops` route:
/// MCFG-backed functions keep their exact ECAM allocation, while the legacy
/// fallback is restricted to domain zero and conventional 256-byte config
/// space. `Snapshot` is only used by synthetic devices that were not found by
/// a hardware enumerator.
#[derive(Clone, Copy, Debug)]
pub enum PciConfigBackend {
    Snapshot,
    Ecam(McfgEntry),
    LegacyCf8,
}

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

/// Prefix of `struct pci_dev` through its complete target-config ABI storage.
///
/// Source: `vendor/linux/include/linux/pci.h:351`. Offsets are pinned to the
/// same generated `.config` as the staged modules.  `name` is private backing
/// storage placed after Linux's 1944-byte object and is reached only through
/// `dev.init_name`.
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
    pub _pad_to_rom_base_reg: [u8; LINUX_PCI_DEV_ROM_BASE_REG_OFFSET - 0x4a],
    pub rom_base_reg: u8,
    pub pin: u8,
    pub _pad_to_driver: [u8; LINUX_PCI_DEV_DRIVER_OFFSET - (LINUX_PCI_DEV_PIN_OFFSET + 1)],
    pub driver: *mut c_void,
    pub dma_mask: u64,
    pub msi_addr_mask: u64,
    pub dma_parms: LinuxDeviceDmaParameters,
    pub _pad_to_error_state: [u8; LINUX_PCI_DEV_ERROR_STATE_OFFSET - 0xa0],
    pub error_state: u32,
    pub dev: LinuxDevice,
    pub _pad_after_device_prefix: [u8; LINUX_PCI_DEV_CFG_SIZE_OFFSET
        - (LINUX_PCI_DEV_DEVICE_OFFSET + size_of::<LinuxDevice>())],
    pub cfg_size: i32,
    pub irq: u32,
    pub resource: [LinuxResource; PCI_DEVICE_RESOURCE_COUNT],
    pub driver_exclusive_resource: LinuxResource,
    pub _pad_to_abi_end: [u8; LINUX_PCI_DEV_ABI_SIZE
        - (LINUX_PCI_DEV_DRIVER_EXCLUSIVE_RESOURCE_OFFSET + size_of::<LinuxResource>())],
    pub name: [u8; 32],
}

/// `struct device_dma_parameters` from `include/linux/device.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxDeviceDmaParameters {
    pub max_segment_size: u32,
    pub min_align_mask: u32,
    pub segment_boundary_mask: usize,
}

/// Target-config layout of Linux `struct pci_bus` on x86_64.
///
/// `sysdata_storage` is private backing for the x86 `struct pci_sysdata` and
/// begins after Linux's 1048-byte object.
#[repr(C)]
pub struct LinuxPciBus {
    pub node: LinuxListHead,
    pub parent: *mut LinuxPciBus,
    pub children: LinuxListHead,
    pub devices: LinuxListHead,
    pub self_: *mut LinuxPciDev,
    pub slots: LinuxListHead,
    pub resource: [*mut LinuxResource; 4],
    pub resources: LinuxListHead,
    pub busn_res: LinuxResource,
    pub ops: *mut c_void,
    pub sysdata: *mut LinuxPciSysData,
    pub procdir: *mut c_void,
    pub number: u8,
    pub primary: u8,
    pub max_bus_speed: u8,
    pub cur_bus_speed: u8,
    pub name: [u8; 48],
    pub bridge_ctl: u16,
    pub bus_flags: u16,
    pub bridge: *mut LinuxDevice,
    pub dev: LinuxDevice,
    pub _pad_to_flags:
        [u8; LINUX_PCI_BUS_FLAGS_OFFSET - (LINUX_PCI_BUS_DEVICE_OFFSET + size_of::<LinuxDevice>())],
    pub state_flags: u32,
    pub _pad_to_abi_end: [u8; LINUX_PCI_BUS_ABI_SIZE - (LINUX_PCI_BUS_FLAGS_OFFSET + 4)],
    pub sysdata_storage: LinuxPciSysData,
}

/// x86 `struct pci_sysdata` prefix used by the inline `pci_domain_nr()` path.
#[repr(C)]
pub struct LinuxPciSysData {
    pub domain: i32,
    pub node: i32,
    pub companion: *mut c_void,
    pub iommu: *mut c_void,
    pub fwnode: *mut c_void,
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
    pub expansion_rom: Option<LinuxPciBarResource>,
    pub config_space: [u8; PCI_CONFIG_SPACE_SIZE],
    pub config_backend: PciConfigBackend,
    pub cfg_size: usize,
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
        Self::new_with_subsystem_bars_config_and_backend(
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
            None,
            config_space,
            PciConfigBackend::Snapshot,
            PCI_CONFIG_SPACE_SIZE,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_subsystem_bars_config_and_backend(
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
        expansion_rom: Option<LinuxPciBarResource>,
        config_space: [u8; PCI_CONFIG_SPACE_SIZE],
        config_backend: PciConfigBackend,
        cfg_size: usize,
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
            expansion_rom,
            config_space,
            config_backend,
            cfg_size,
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
        if let Some(rom) = dev.expansion_rom {
            resource[6].start = rom.start;
            resource[6].end = rom.end();
            resource[6].flags = rom.flags;
        }
        let header_type = dev.config_space[0x0e] & 0x7f;
        let rom_base_reg = match header_type {
            0 => 0x30,
            1 => 0x38,
            _ => 0,
        };
        let pin = dev.config_space[0x3d];
        if dev.class == PCI_CLASS_BRIDGE_HOST
            && dev.subclass == PCI_CLASS_BRIDGE_PCI
            && header_type == 1
        {
            decode_pci_bridge_windows(&dev.config_space, &mut resource);
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
            hdr_type: header_type,
            _pad_to_rom_base_reg: [0; LINUX_PCI_DEV_ROM_BASE_REG_OFFSET - 0x4a],
            rom_base_reg,
            pin,
            _pad_to_driver: [0; LINUX_PCI_DEV_DRIVER_OFFSET - (LINUX_PCI_DEV_PIN_OFFSET + 1)],
            driver: core::ptr::null_mut(),
            dma_mask: u32::MAX as u64,
            msi_addr_mask: u64::MAX,
            dma_parms: LinuxDeviceDmaParameters {
                max_segment_size: 65_536,
                min_align_mask: 0,
                segment_boundary_mask: u32::MAX as usize,
            },
            _pad_to_error_state: [0; LINUX_PCI_DEV_ERROR_STATE_OFFSET - 0xa0],
            error_state: LINUX_PCI_CHANNEL_IO_NORMAL,
            dev: unsafe { core::mem::zeroed::<LinuxDevice>() },
            _pad_after_device_prefix: [0; LINUX_PCI_DEV_CFG_SIZE_OFFSET
                - (LINUX_PCI_DEV_DEVICE_OFFSET + size_of::<LinuxDevice>())],
            cfg_size: dev.cfg_size as i32,
            irq: if pin == 0 {
                0
            } else {
                dev.config_space[0x3c] as u32
            },
            resource,
            driver_exclusive_resource: LinuxResource {
                start: 0,
                end: u64::MAX,
                name: PCI_EXCLUSIVE_RESOURCE_NAME.as_ptr().cast::<c_char>(),
                flags: 0,
                desc: 0,
                parent: core::ptr::null_mut(),
                sibling: core::ptr::null_mut(),
                child: core::ptr::null_mut(),
            },
            _pad_to_abi_end: [0; LINUX_PCI_DEV_ABI_SIZE
                - (LINUX_PCI_DEV_DRIVER_EXCLUSIVE_RESOURCE_OFFSET + size_of::<LinuxResource>())],
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

fn decode_pci_bridge_windows(
    config: &[u8; PCI_CONFIG_SPACE_SIZE],
    resource: &mut [LinuxResource; PCI_DEVICE_RESOURCE_COUNT],
) {
    let io_base_lo = config[0x1c];
    let io_limit_lo = config[0x1d];
    let mut io_base = ((io_base_lo & 0xf0) as u64) << 8;
    let mut io_limit = ((io_limit_lo & 0xf0) as u64) << 8;
    if io_base_lo & 0x0f == 1 {
        io_base |= (u16::from_le_bytes([config[0x30], config[0x31]]) as u64) << 16;
        io_limit |= (u16::from_le_bytes([config[0x32], config[0x33]]) as u64) << 16;
    }
    set_bridge_window(
        &mut resource[7],
        io_base,
        io_limit,
        0x0fff,
        IORESOURCE_IO | (io_base_lo & 0x0f) as usize,
        io_base_lo != 0 || io_limit_lo != 0,
    );

    let mem_base_lo = u16::from_le_bytes([config[0x20], config[0x21]]);
    let mem_limit_lo = u16::from_le_bytes([config[0x22], config[0x23]]);
    let mem_base = ((mem_base_lo & 0xfff0) as u64) << 16;
    let mem_limit = ((mem_limit_lo & 0xfff0) as u64) << 16;
    set_bridge_window(
        &mut resource[8],
        mem_base,
        mem_limit,
        0x000f_ffff,
        IORESOURCE_MEM | (mem_base_lo & 0x000f) as usize,
        mem_base_lo != 0 || mem_limit_lo != 0,
    );

    let pref_base_lo = u16::from_le_bytes([config[0x24], config[0x25]]);
    let pref_limit_lo = u16::from_le_bytes([config[0x26], config[0x27]]);
    let mut pref_base = ((pref_base_lo & 0xfff0) as u64) << 16;
    let mut pref_limit = ((pref_limit_lo & 0xfff0) as u64) << 16;
    let pref_type = pref_base_lo & 0x000f;
    if pref_type == 1 {
        pref_base |= (u32::from_le_bytes(config[0x28..0x2c].try_into().unwrap()) as u64) << 32;
        pref_limit |= (u32::from_le_bytes(config[0x2c..0x30].try_into().unwrap()) as u64) << 32;
    }
    let mut pref_flags = IORESOURCE_MEM | IORESOURCE_PREFETCH | pref_type as usize;
    if pref_type == 1 {
        pref_flags |= IORESOURCE_MEM_64;
    }
    set_bridge_window(
        &mut resource[9],
        pref_base,
        pref_limit,
        0x000f_ffff,
        pref_flags,
        pref_base_lo != 0 || pref_limit_lo != 0,
    );
}

fn set_bridge_window(
    resource: &mut LinuxResource,
    base: u64,
    limit: u64,
    granularity_mask: u64,
    flags: usize,
    implemented: bool,
) {
    resource.flags = flags;
    if implemented && base <= limit {
        resource.start = base;
        resource.end = limit | granularity_mask;
    } else {
        resource.start = 0;
        resource.end = 0;
        resource.flags |= IORESOURCE_DISABLED | IORESOURCE_UNSET;
    }
}

impl LinuxPciBus {
    fn allocate(seg: u16, number: u8, bus_type: *const LinuxBusType) -> Box<Self> {
        let mut raw = Box::new(Self {
            node: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            parent: core::ptr::null_mut(),
            children: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            devices: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            self_: core::ptr::null_mut(),
            slots: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            resource: [core::ptr::null_mut(); 4],
            resources: LinuxListHead {
                next: core::ptr::null_mut(),
                prev: core::ptr::null_mut(),
            },
            busn_res: LinuxResource {
                start: number as u64,
                end: number as u64,
                name: core::ptr::null(),
                flags: 0x0000_1000,
                desc: 0,
                parent: core::ptr::null_mut(),
                sibling: core::ptr::null_mut(),
                child: core::ptr::null_mut(),
            },
            ops: core::ptr::null_mut(),
            sysdata: core::ptr::null_mut(),
            procdir: core::ptr::null_mut(),
            number,
            primary: 0,
            max_bus_speed: u8::MAX,
            cur_bus_speed: u8::MAX,
            name: [0; 48],
            bridge_ctl: 0,
            bus_flags: 0,
            bridge: core::ptr::null_mut(),
            dev: unsafe { core::mem::zeroed::<LinuxDevice>() },
            _pad_to_flags: [0; LINUX_PCI_BUS_FLAGS_OFFSET
                - (LINUX_PCI_BUS_DEVICE_OFFSET + size_of::<LinuxDevice>())],
            state_flags: 0,
            _pad_to_abi_end: [0; LINUX_PCI_BUS_ABI_SIZE - (LINUX_PCI_BUS_FLAGS_OFFSET + 4)],
            sysdata_storage: LinuxPciSysData {
                domain: seg as i32,
                node: -1,
                companion: core::ptr::null_mut(),
                iommu: core::ptr::null_mut(),
                fwnode: core::ptr::null_mut(),
            },
        });

        let label = alloc::format!("PCI Bus {:04x}:{:02x}", seg, number);
        let len = core::cmp::min(label.len(), raw.name.len() - 1);
        raw.name[..len].copy_from_slice(&label.as_bytes()[..len]);
        raw.name[len] = 0;

        let ptr = (&mut *raw) as *mut LinuxPciBus;
        unsafe {
            linux_list_head_init(core::ptr::addr_of_mut!((*ptr).node));
            linux_list_head_init(core::ptr::addr_of_mut!((*ptr).children));
            linux_list_head_init(core::ptr::addr_of_mut!((*ptr).devices));
            linux_list_head_init(core::ptr::addr_of_mut!((*ptr).slots));
            linux_list_head_init(core::ptr::addr_of_mut!((*ptr).resources));
            (*ptr).sysdata = core::ptr::addr_of_mut!((*ptr).sysdata_storage);
            (*ptr).busn_res.name = (*ptr).name.as_ptr().cast::<c_char>();
            (*ptr).dev.init_name = (*ptr).name.as_ptr().cast::<c_char>();
            (*ptr).dev.bus = bus_type;
        }
        raw
    }
}

unsafe fn linux_list_head_init(head: *mut LinuxListHead) {
    unsafe {
        (*head).next = head.cast();
        (*head).prev = head.cast();
    }
}

unsafe fn linux_list_add_tail(node: *mut LinuxListHead, head: *mut LinuxListHead) {
    unsafe {
        let tail = (*head).prev.cast::<LinuxListHead>();
        (*node).next = head.cast();
        (*node).prev = tail.cast();
        (*tail).next = node.cast();
        (*head).prev = node.cast();
    }
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciDevice {
    dev: usize,
    state: LinuxPciDeviceAbiState,
    backend: PciConfigBackend,
    segment: u16,
    bus: u8,
    slot: u8,
    function: u8,
    cfg_size: usize,
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciObject {
    seg: u16,
    bus: u8,
    dev: u8,
    func: u8,
    raw: usize,
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciBus {
    seg: u16,
    number: u8,
    raw: usize,
}

#[derive(Clone, Copy)]
struct RegisteredLinuxPciDomainBusnResource {
    seg: u16,
    raw: usize,
}

lazy_static! {
    /// Serializes publication and topology reconstruction of the raw Linux PCI
    /// objects. Enumeration is normally single-threaded, but driver-triggered
    /// discovery and parallel host tests can otherwise rebuild the same
    /// intrusive lists concurrently.
    static ref LINUX_PCI_REGISTRATION_LOCK: Mutex<()> = Mutex::new(());
    static ref LINUX_PCI_DEVICE_STATES: Mutex<Vec<RegisteredLinuxPciDevice>> =
        Mutex::new(Vec::new());
    static ref LINUX_PCI_OBJECTS: Mutex<Vec<RegisteredLinuxPciObject>> = Mutex::new(Vec::new());
    static ref LINUX_PCI_BUSES: Mutex<Vec<RegisteredLinuxPciBus>> = Mutex::new(Vec::new());
    static ref LINUX_PCI_DOMAIN_BUSN_RESOURCES: Mutex<Vec<RegisteredLinuxPciDomainBusnResource>> =
        Mutex::new(Vec::new());
}

pub(crate) static LINUX_PCI_CONFIG_IO_LOCK: Mutex<()> = Mutex::new(());

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

/// Snapshot the raw Linux `struct pci_dev` objects registered by enumeration.
/// The objects remain owned by `LINUX_PCI_OBJECTS` for the life of the kernel.
pub fn registered_linux_pci_raw_devices() -> Vec<*mut LinuxPciDev> {
    LINUX_PCI_OBJECTS
        .lock()
        .iter()
        .map(|entry| entry.raw as *mut LinuxPciDev)
        .collect()
}

fn linux_pci_bus_raw(seg: u16, number: u8, bus_type: *const LinuxBusType) -> *mut LinuxPciBus {
    let mut buses = LINUX_PCI_BUSES.lock();
    if let Some(bus) = buses
        .iter()
        .find(|bus| bus.seg == seg && bus.number == number)
    {
        return bus.raw as *mut LinuxPciBus;
    }

    let mut raw = LinuxPciBus::allocate(seg, number, bus_type);
    let raw_ptr = (&mut *raw) as *mut LinuxPciBus;
    buses.push(RegisteredLinuxPciBus {
        seg,
        number,
        raw: Box::into_raw(raw) as usize,
    });
    raw_ptr
}

fn linux_pci_domain_busn_resource(seg: u16) -> *mut LinuxResource {
    let mut domains = LINUX_PCI_DOMAIN_BUSN_RESOURCES.lock();
    if let Some(domain) = domains.iter().find(|domain| domain.seg == seg) {
        return domain.raw as *mut LinuxResource;
    }

    let resource = Box::new(LinuxResource {
        start: 0,
        end: u8::MAX as u64,
        name: PCI_BUSN_RESOURCE_NAME.as_ptr().cast::<c_char>(),
        flags: IORESOURCE_BUS | IORESOURCE_PCI_FIXED,
        desc: 0,
        parent: core::ptr::null_mut(),
        sibling: core::ptr::null_mut(),
        child: core::ptr::null_mut(),
    });
    let raw = Box::into_raw(resource);
    domains.push(RegisteredLinuxPciDomainBusnResource {
        seg,
        raw: raw as usize,
    });
    raw
}

#[cfg(test)]
fn registered_linux_pci_bus(seg: u16, number: u8) -> *mut LinuxPciBus {
    LINUX_PCI_BUSES
        .lock()
        .iter()
        .find(|bus| bus.seg == seg && bus.number == number)
        .map(|bus| bus.raw as *mut LinuxPciBus)
        .unwrap_or(core::ptr::null_mut())
}

unsafe fn linux_resource_request(parent: *mut LinuxResource, resource: *mut LinuxResource) -> bool {
    if parent.is_null() || resource.is_null() {
        return false;
    }
    unsafe {
        if ((*parent).flags & IORESOURCE_TYPE_BITS) != ((*resource).flags & IORESOURCE_TYPE_BITS)
            || (*parent).flags & IORESOURCE_UNSET != 0
            || (*resource).flags & IORESOURCE_UNSET != 0
            || (*parent).start > (*resource).start
            || (*parent).end < (*resource).end
        {
            return false;
        }

        let mut link = core::ptr::addr_of_mut!((*parent).child);
        while !(*link).is_null() {
            let sibling = *link;
            if (*sibling).end < (*resource).start {
                link = core::ptr::addr_of_mut!((*sibling).sibling);
                continue;
            }
            if (*sibling).start <= (*resource).end {
                return false;
            }
            break;
        }

        (*resource).parent = parent;
        (*resource).sibling = *link;
        *link = resource;
    }
    true
}

unsafe fn linux_pci_find_parent_resource(
    bus: *mut LinuxPciBus,
    resource: *mut LinuxResource,
) -> *mut LinuxResource {
    if bus.is_null() || resource.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        for candidate in (*bus).resource {
            if candidate.is_null()
                || ((*candidate).flags & IORESOURCE_TYPE_BITS)
                    != ((*resource).flags & IORESOURCE_TYPE_BITS)
                || (*candidate).flags & IORESOURCE_UNSET != 0
                || (*resource).flags & IORESOURCE_UNSET != 0
                || (*candidate).start > (*resource).start
                || (*candidate).end < (*resource).end
            {
                continue;
            }
            if (*candidate).flags & IORESOURCE_PREFETCH != 0
                && (*resource).flags & IORESOURCE_PREFETCH == 0
            {
                return core::ptr::null_mut();
            }
            return candidate;
        }
    }
    core::ptr::null_mut()
}

unsafe fn linux_pci_bus_depth(mut bus: *mut LinuxPciBus) -> usize {
    let mut depth = 0usize;
    while !bus.is_null() && depth < 256 {
        bus = unsafe { (*bus).parent };
        depth += 1;
    }
    depth
}

fn sync_linux_pci_resource_ancestry(
    buses: &[RegisteredLinuxPciBus],
    devices: &[RegisteredLinuxPciObject],
) {
    let mut ordered_buses = buses.to_vec();
    ordered_buses.sort_by_key(|bus| (bus.seg, bus.number));

    unsafe {
        for domain in LINUX_PCI_DOMAIN_BUSN_RESOURCES.lock().iter() {
            let resource = domain.raw as *mut LinuxResource;
            (*resource).parent = core::ptr::null_mut();
            (*resource).sibling = core::ptr::null_mut();
            (*resource).child = core::ptr::null_mut();
        }

        for bus in &ordered_buses {
            let raw = bus.raw as *mut LinuxPciBus;
            (*raw).busn_res.start = (*raw).number as u64;
            (*raw).busn_res.parent = core::ptr::null_mut();
            (*raw).busn_res.sibling = core::ptr::null_mut();
            (*raw).busn_res.child = core::ptr::null_mut();
            (*raw).busn_res.flags = if (*raw).parent.is_null() {
                IORESOURCE_BUS | IORESOURCE_PCI_FIXED
            } else {
                IORESOURCE_BUS
            };
        }

        for root in ordered_buses
            .iter()
            .filter(|bus| (*(bus.raw as *mut LinuxPciBus)).parent.is_null())
        {
            let root_raw = root.raw as *mut LinuxPciBus;
            let mut end = (*root_raw).number as u64;
            for child in &ordered_buses {
                let child_raw = child.raw as *mut LinuxPciBus;
                if (*child_raw).parent == root_raw {
                    end = end.max((*child_raw).busn_res.end);
                }
            }
            (*root_raw).busn_res.end = end;
            let domain = linux_pci_domain_busn_resource(root.seg);
            let _ = linux_resource_request(domain, core::ptr::addr_of_mut!((*root_raw).busn_res));
        }

        ordered_buses.sort_by_key(|bus| linux_pci_bus_depth(bus.raw as *mut LinuxPciBus));
        for child in ordered_buses
            .iter()
            .filter(|bus| !(*(bus.raw as *mut LinuxPciBus)).parent.is_null())
        {
            let child_raw = child.raw as *mut LinuxPciBus;
            let parent = (*child_raw).parent;
            let _ = linux_resource_request(
                core::ptr::addr_of_mut!((*parent).busn_res),
                core::ptr::addr_of_mut!((*child_raw).busn_res),
            );
        }

        for device in devices {
            let raw = device.raw as *mut LinuxPciDev;
            for (index, resource) in (*raw).resource.iter_mut().enumerate() {
                resource.parent = core::ptr::null_mut();
                resource.sibling = core::ptr::null_mut();
                resource.child = core::ptr::null_mut();
                if resource.flags != 0 {
                    resource.name = if index >= 7 && !(*raw).subordinate.is_null() {
                        let child = (*raw).subordinate.cast::<LinuxPciBus>();
                        (*child).name.as_ptr().cast::<c_char>()
                    } else {
                        (*raw).name.as_ptr().cast::<c_char>()
                    };
                }
            }
        }

        let mut ordered_devices = devices.to_vec();
        ordered_devices.sort_by_key(|device| {
            let raw = device.raw as *mut LinuxPciDev;
            linux_pci_bus_depth((*raw).bus.cast::<LinuxPciBus>())
        });
        for device in &ordered_devices {
            let raw = device.raw as *mut LinuxPciDev;
            let bus = (*raw).bus.cast::<LinuxPciBus>();
            for resource in &mut (*raw).resource {
                if resource.flags & (IORESOURCE_IO | IORESOURCE_MEM) == 0
                    || resource.flags & (IORESOURCE_UNSET | IORESOURCE_DISABLED) != 0
                {
                    continue;
                }
                let parent = linux_pci_find_parent_resource(bus, resource);
                if !parent.is_null() {
                    let _ = linux_resource_request(parent, resource);
                }
            }
        }
    }
}

fn sync_linux_pci_bus_topology() {
    let buses = LINUX_PCI_BUSES.lock().clone();
    let devices = LINUX_PCI_OBJECTS.lock().clone();

    unsafe {
        for bus in &buses {
            let raw = bus.raw as *mut LinuxPciBus;
            (*raw).parent = core::ptr::null_mut();
            (*raw).self_ = core::ptr::null_mut();
            (*raw).bridge = core::ptr::null_mut();
            (*raw).primary = 0;
            (*raw).bridge_ctl = 0;
            (*raw).resource.fill(core::ptr::null_mut());
            (*raw).busn_res.start = (*raw).number as u64;
            (*raw).busn_res.end = (*raw).number as u64;
            (*raw).busn_res.parent = core::ptr::null_mut();
            (*raw).busn_res.sibling = core::ptr::null_mut();
            (*raw).busn_res.child = core::ptr::null_mut();
            (*raw).dev.parent = core::ptr::null_mut();
            linux_list_head_init(core::ptr::addr_of_mut!((*raw).node));
            linux_list_head_init(core::ptr::addr_of_mut!((*raw).children));
            linux_list_head_init(core::ptr::addr_of_mut!((*raw).devices));
        }

        for device in &devices {
            let raw = device.raw as *mut LinuxPciDev;
            linux_list_head_init(core::ptr::addr_of_mut!((*raw).bus_list));
            (*raw).subordinate = core::ptr::null_mut();
            if let Some(bus) = buses
                .iter()
                .find(|bus| bus.seg == device.seg && bus.number == device.bus)
            {
                let bus_raw = bus.raw as *mut LinuxPciBus;
                (*raw).bus = bus_raw.cast();
                (*raw).sysdata = (*bus_raw).sysdata.cast();
                (*raw).dev.parent = core::ptr::addr_of_mut!((*bus_raw).dev);
                linux_list_add_tail(
                    core::ptr::addr_of_mut!((*raw).bus_list),
                    core::ptr::addr_of_mut!((*bus_raw).devices),
                );
            }
        }

        for child in &buses {
            let candidates = devices
                .iter()
                .filter(|device| {
                    if device.seg != child.seg || device.bus >= child.number {
                        return false;
                    }
                    let Some(state) = linux_pci_device_state(device.raw as *const c_void) else {
                        return false;
                    };
                    state.config_space[0x0b] == PCI_CLASS_BRIDGE_HOST
                        && state.config_space[0x0a] == PCI_CLASS_BRIDGE_PCI
                        && state.config_space[0x0e] & 0x7f == 1
                        && state.config_space[0x18] == device.bus
                        && state.config_space[0x19] == child.number
                        && state.config_space[0x1a] >= child.number
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                continue;
            }
            let bridge = candidates[0];
            let Some(parent) = buses
                .iter()
                .find(|bus| bus.seg == bridge.seg && bus.number == bridge.bus)
            else {
                continue;
            };

            let child_raw = child.raw as *mut LinuxPciBus;
            let parent_raw = parent.raw as *mut LinuxPciBus;
            let bridge_raw = bridge.raw as *mut LinuxPciDev;
            let state = linux_pci_device_state(bridge.raw as *const c_void)
                .expect("registered PCI bridge state");
            (*child_raw).parent = parent_raw;
            (*child_raw).self_ = bridge_raw;
            (*child_raw).bridge = core::ptr::addr_of_mut!((*bridge_raw).dev);
            (*child_raw).dev.parent = core::ptr::addr_of_mut!((*bridge_raw).dev);
            (*child_raw).ops = (*parent_raw).ops;
            (*child_raw).sysdata = (*parent_raw).sysdata;
            (*child_raw).bus_flags = (*parent_raw).bus_flags;
            (*child_raw).primary = state.config_space[0x18];
            (*child_raw).busn_res.end = state.config_space[0x1a] as u64;
            (*child_raw).bridge_ctl =
                u16::from_le_bytes([state.config_space[0x3e], state.config_space[0x3f]]);
            for index in 0..4 {
                (*child_raw).resource[index] =
                    core::ptr::addr_of_mut!((*bridge_raw).resource[7 + index]);
            }
            (*bridge_raw).subordinate = child_raw.cast();
            for endpoint in devices
                .iter()
                .filter(|device| device.seg == child.seg && device.bus == child.number)
            {
                (*(endpoint.raw as *mut LinuxPciDev)).sysdata = (*child_raw).sysdata.cast();
            }
            linux_list_add_tail(
                core::ptr::addr_of_mut!((*child_raw).node),
                core::ptr::addr_of_mut!((*parent_raw).children),
            );
        }
    }
    sync_linux_pci_resource_ancestry(&buses, &devices);
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
    let _registration_guard = LINUX_PCI_REGISTRATION_LOCK.lock();
    let existing = linux_pci_raw_device_for_slot(dev.seg, dev.bus, dev.dev, dev.func);
    if !existing.is_null() {
        return existing;
    }

    let bus_raw = linux_pci_bus_raw(dev.seg, dev.bus, bus_type);
    if dev.class == PCI_CLASS_BRIDGE_HOST
        && dev.subclass == PCI_CLASS_BRIDGE_PCI
        && dev.config_space[0x0e] & 0x7f == 1
    {
        let secondary = dev.config_space[0x19];
        let subordinate = dev.config_space[0x1a];
        if secondary > dev.bus && subordinate >= secondary {
            linux_pci_bus_raw(dev.seg, secondary, bus_type);
        }
    }

    let mut raw = Box::new(LinuxPciDev::from_pci_dev(dev, bus_type));
    let raw_ptr = (&mut *raw) as *mut LinuxPciDev;
    unsafe {
        (*raw_ptr).bus = bus_raw.cast();
        (*raw_ptr).sysdata = (*bus_raw).sysdata.cast();
        (*raw_ptr).dev.parent = core::ptr::addr_of_mut!((*bus_raw).dev);
        (*raw_ptr).dev.init_name = (*raw_ptr).name.as_ptr().cast::<c_char>();
        let dev_bytes = core::ptr::addr_of_mut!((*raw_ptr).dev).cast::<u8>();
        dev_bytes
            .add(LINUX_DEVICE_DMA_MASK_OFFSET)
            .cast::<*mut u64>()
            .write(core::ptr::addr_of_mut!((*raw_ptr).dma_mask));
        dev_bytes
            .add(LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET)
            .cast::<u64>()
            .write(u32::MAX as u64);
        dev_bytes
            .add(LINUX_DEVICE_DMA_PARMS_OFFSET)
            .cast::<*mut LinuxDeviceDmaParameters>()
            .write(core::ptr::addr_of_mut!((*raw_ptr).dma_parms));
    }

    let state = LinuxPciDeviceAbiState::from_pci_dev(dev);
    register_linux_pci_device_state_with_backend(
        raw_ptr.cast_const().cast(),
        state,
        dev.config_backend,
        dev.seg,
        dev.bus,
        dev.dev,
        dev.func,
        dev.cfg_size,
    );
    let leaked = Box::into_raw(raw);
    LINUX_PCI_OBJECTS.lock().push(RegisteredLinuxPciObject {
        seg: dev.seg,
        bus: dev.bus,
        dev: dev.dev,
        func: dev.func,
        raw: leaked as usize,
    });
    sync_linux_pci_bus_topology();

    let ret = unsafe { linux_device_register(core::ptr::addr_of_mut!((*raw_ptr).dev)) };
    if ret != 0 {
        LINUX_PCI_OBJECTS
            .lock()
            .retain(|entry| entry.raw != leaked as usize);
        unregister_linux_pci_device_state(raw_ptr.cast_const().cast());
        sync_linux_pci_bus_topology();
        unsafe {
            drop(Box::from_raw(leaked));
        }
        return core::ptr::null_mut();
    }
    raw_ptr
}

pub fn register_linux_pci_device_state(dev: *const c_void, state: LinuxPciDeviceAbiState) {
    register_linux_pci_device_state_with_backend(
        dev,
        state,
        PciConfigBackend::Snapshot,
        0,
        0,
        0,
        0,
        PCI_CONFIG_SPACE_SIZE,
    );
}

#[allow(clippy::too_many_arguments)]
fn register_linux_pci_device_state_with_backend(
    dev: *const c_void,
    state: LinuxPciDeviceAbiState,
    backend: PciConfigBackend,
    segment: u16,
    bus: u8,
    slot: u8,
    function: u8,
    cfg_size: usize,
) {
    if dev.is_null() {
        return;
    }
    let mut states = LINUX_PCI_DEVICE_STATES.lock();
    if let Some(existing) = states.iter_mut().find(|entry| entry.dev == dev as usize) {
        existing.state = state;
        existing.backend = backend;
        existing.segment = segment;
        existing.bus = bus;
        existing.slot = slot;
        existing.function = function;
        existing.cfg_size = cfg_size;
    } else {
        states.push(RegisteredLinuxPciDevice {
            dev: dev as usize,
            state,
            backend,
            segment,
            bus,
            slot,
            function,
            cfg_size,
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
    let entry = LINUX_PCI_DEVICE_STATES
        .lock()
        .iter()
        .find(|entry| entry.dev == dev as usize)
        .copied()?;
    if !pci_config_access_is_aligned(offset, width) || offset.checked_add(width)? > entry.cfg_size {
        return None;
    }

    match entry.backend {
        PciConfigBackend::Snapshot => {
            if offset + width > PCI_CONFIG_SPACE_SIZE {
                return None;
            }
            let bytes = &entry.state.config_space[offset..offset + width];
            Some(match width {
                1 => bytes[0] as u32,
                2 => u16::from_le_bytes([bytes[0], bytes[1]]) as u32,
                4 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                _ => unreachable!(),
            })
        }
        PciConfigBackend::Ecam(ecam) => {
            if entry.segment != ecam.segment
                || entry.bus < ecam.bus_start
                || entry.bus > ecam.bus_end
                || offset + width > PCI_CONFIG_SPACE_EXP_SIZE
            {
                return None;
            }
            let offset = offset as u16;
            Some(unsafe {
                match width {
                    1 => ecam.read8(entry.bus, entry.slot, entry.function, offset) as u32,
                    2 => ecam.read16(entry.bus, entry.slot, entry.function, offset) as u32,
                    4 => ecam.read32(entry.bus, entry.slot, entry.function, offset),
                    _ => unreachable!(),
                }
            })
        }
        PciConfigBackend::LegacyCf8 => {
            if entry.segment != 0 || offset + width > PCI_CONFIG_SPACE_SIZE {
                return None;
            }
            let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
            let address = crate::arch::x86::pci::cf8_address(
                entry.bus,
                entry.slot,
                entry.function,
                offset as u8,
            );
            unsafe {
                outl(PCI_CONFIG_ADDRESS_PORT, address);
                Some(match width {
                    1 => inb(PCI_CONFIG_DATA_PORT + ((offset & 3) as u16)) as u32,
                    2 => inw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16)) as u32,
                    4 => inl(PCI_CONFIG_DATA_PORT),
                    _ => unreachable!(),
                })
            }
        }
    }
}

pub fn linux_pci_config_write(dev: *const c_void, offset: usize, width: usize, value: u32) -> bool {
    if dev.is_null() || !pci_config_access_is_aligned(offset, width) {
        return false;
    }

    let Some(entry) = LINUX_PCI_DEVICE_STATES
        .lock()
        .iter()
        .find(|entry| entry.dev == dev as usize)
        .copied()
    else {
        return false;
    };
    if offset
        .checked_add(width)
        .is_none_or(|end| end > entry.cfg_size)
    {
        return false;
    }

    let written = match entry.backend {
        PciConfigBackend::Snapshot => offset + width <= PCI_CONFIG_SPACE_SIZE,
        PciConfigBackend::Ecam(ecam) => {
            if entry.segment != ecam.segment
                || entry.bus < ecam.bus_start
                || entry.bus > ecam.bus_end
                || offset + width > PCI_CONFIG_SPACE_EXP_SIZE
            {
                false
            } else {
                let offset = offset as u16;
                unsafe {
                    match width {
                        1 => {
                            ecam.write8(entry.bus, entry.slot, entry.function, offset, value as u8)
                        }
                        2 => ecam.write16(
                            entry.bus,
                            entry.slot,
                            entry.function,
                            offset,
                            value as u16,
                        ),
                        4 => ecam.write32(entry.bus, entry.slot, entry.function, offset, value),
                        _ => unreachable!(),
                    }
                }
                true
            }
        }
        PciConfigBackend::LegacyCf8 => {
            if entry.segment != 0 || offset + width > PCI_CONFIG_SPACE_SIZE {
                false
            } else {
                let _guard = LINUX_PCI_CONFIG_IO_LOCK.lock();
                let address = crate::arch::x86::pci::cf8_address(
                    entry.bus,
                    entry.slot,
                    entry.function,
                    offset as u8,
                );
                unsafe {
                    outl(PCI_CONFIG_ADDRESS_PORT, address);
                    match width {
                        1 => outb(PCI_CONFIG_DATA_PORT + ((offset & 3) as u16), value as u8),
                        2 => outw(PCI_CONFIG_DATA_PORT + ((offset & 2) as u16), value as u16),
                        4 => outl(PCI_CONFIG_DATA_PORT, value),
                        _ => unreachable!(),
                    }
                }
                true
            }
        }
    };
    if !written {
        return false;
    }

    if offset + width <= PCI_CONFIG_SPACE_SIZE {
        let mut states = LINUX_PCI_DEVICE_STATES.lock();
        if let Some(current) = states.iter_mut().find(|entry| entry.dev == dev as usize) {
            let bytes = value.to_le_bytes();
            current.state.config_space[offset..offset + width].copy_from_slice(&bytes[..width]);
        }
    }
    true
}

const fn pci_config_access_is_aligned(offset: usize, width: usize) -> bool {
    match width {
        1 => true,
        2 => offset & 1 == 0,
        4 => offset & 3 == 0,
        _ => false,
    }
}

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
        assert_eq!(offset_of!(LinuxPciDev, driver), LINUX_PCI_DEV_DRIVER_OFFSET);
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
        assert_eq!(PCI_DEVICE_RESOURCE_COUNT, 11);
        assert_eq!(
            offset_of!(LinuxPciDev, driver_exclusive_resource),
            LINUX_PCI_DEV_DRIVER_EXCLUSIVE_RESOURCE_OFFSET
        );
        assert_eq!(offset_of!(LinuxPciDev, name), LINUX_PCI_DEV_ABI_SIZE);
        assert_eq!(size_of::<LinuxDeviceDmaParameters>(), 16);
    }

    #[test]
    fn linux_pci_bus_layout_matches_staged_x86_64_config() {
        assert_eq!(offset_of!(LinuxPciBus, node), 0);
        assert_eq!(offset_of!(LinuxPciBus, parent), 16);
        assert_eq!(offset_of!(LinuxPciBus, children), 24);
        assert_eq!(offset_of!(LinuxPciBus, devices), 40);
        assert_eq!(offset_of!(LinuxPciBus, self_), 56);
        assert_eq!(offset_of!(LinuxPciBus, slots), 64);
        assert_eq!(offset_of!(LinuxPciBus, resource), 80);
        assert_eq!(offset_of!(LinuxPciBus, resources), 112);
        assert_eq!(offset_of!(LinuxPciBus, busn_res), 128);
        assert_eq!(offset_of!(LinuxPciBus, ops), 192);
        assert_eq!(offset_of!(LinuxPciBus, sysdata), 200);
        assert_eq!(offset_of!(LinuxPciBus, procdir), 208);
        assert_eq!(offset_of!(LinuxPciBus, number), 216);
        assert_eq!(offset_of!(LinuxPciBus, primary), 217);
        assert_eq!(offset_of!(LinuxPciBus, name), 220);
        assert_eq!(offset_of!(LinuxPciBus, bridge_ctl), 268);
        assert_eq!(offset_of!(LinuxPciBus, bus_flags), 270);
        assert_eq!(offset_of!(LinuxPciBus, bridge), 272);
        assert_eq!(offset_of!(LinuxPciBus, dev), LINUX_PCI_BUS_DEVICE_OFFSET);
        assert_eq!(
            offset_of!(LinuxPciBus, state_flags),
            LINUX_PCI_BUS_FLAGS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxPciBus, sysdata_storage),
            LINUX_PCI_BUS_ABI_SIZE
        );
        assert_eq!(size_of::<LinuxPciSysData>(), 32);
    }

    #[test]
    fn linux_pci_bus_topology_links_bridge_and_child_device_lists() {
        let bus_type = crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr();
        crate::linux_driver_abi::pci::driver::register_module_exports();

        let mut bridge_config = default_config_space(0x8086, 0x1234, 0x06, 0x04, 0, 1, 0, 0);
        bridge_config[0x0e] = 1;
        bridge_config[0x18] = 2;
        bridge_config[0x19] = 3;
        bridge_config[0x1a] = 3;
        bridge_config[0x3e..0x40].copy_from_slice(&0x1234u16.to_le_bytes());
        let bridge = PciDev::new_with_subsystem_bars_and_config(
            0x7f7f,
            2,
            1,
            0,
            0x8086,
            0x1234,
            0x06,
            0x04,
            0,
            1,
            0,
            0,
            [None; 6],
            bridge_config,
        );
        let endpoint =
            PciDev::new_with_subsystem(0x7f7f, 3, 2, 0, 0x8086, 0x5678, 0x03, 0x00, 0, 1, 0, 0);

        let bridge_raw = register_linux_pci_device(&bridge, bus_type);
        let endpoint_raw = register_linux_pci_device(&endpoint, bus_type);
        let parent_bus = registered_linux_pci_bus(0x7f7f, 2);
        let child_bus = registered_linux_pci_bus(0x7f7f, 3);
        let _topology_guard = LINUX_PCI_REGISTRATION_LOCK.lock();
        assert!(!bridge_raw.is_null());
        assert!(!endpoint_raw.is_null());
        assert!(!parent_bus.is_null());
        assert!(!child_bus.is_null());

        unsafe {
            assert_eq!((*bridge_raw).bus, parent_bus.cast());
            assert_eq!((*bridge_raw).subordinate, child_bus.cast());
            assert_eq!((*endpoint_raw).bus, child_bus.cast());
            assert_eq!((*child_bus).parent, parent_bus);
            assert_eq!((*child_bus).self_, bridge_raw);
            assert_eq!((*child_bus).primary, 2);
            assert_eq!((*child_bus).busn_res.end, 3);
            assert_eq!((*child_bus).bridge_ctl, 0x1234);
            assert_eq!(
                (*child_bus).resource[0],
                core::ptr::addr_of_mut!((*bridge_raw).resource[7])
            );
            assert_eq!(
                (*child_bus).resource[3],
                core::ptr::addr_of_mut!((*bridge_raw).resource[10])
            );
            assert_eq!((*parent_bus).children.next, child_bus.cast());
            assert_eq!((*child_bus).devices.next, endpoint_raw.cast());
            assert_eq!((*(*child_bus).sysdata).domain, 0x7f7f);
        }
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

            let dev_bytes = core::ptr::addr_of!((*raw).dev).cast::<u8>();
            assert_eq!(
                dev_bytes
                    .add(LINUX_DEVICE_DMA_MASK_OFFSET)
                    .cast::<*mut u64>()
                    .read(),
                core::ptr::addr_of_mut!((*raw).dma_mask)
            );
            assert_eq!(
                dev_bytes
                    .add(LINUX_DEVICE_COHERENT_DMA_MASK_OFFSET)
                    .cast::<u64>()
                    .read(),
                u32::MAX as u64
            );
            assert_eq!(
                dev_bytes
                    .add(LINUX_DEVICE_DMA_PARMS_OFFSET)
                    .cast::<*mut LinuxDeviceDmaParameters>()
                    .read(),
                core::ptr::addr_of_mut!((*raw).dma_parms)
            );
        }
        assert_eq!(linux_pci_raw_device_for_slot(0, 0, 30, 0), raw);
    }
}
