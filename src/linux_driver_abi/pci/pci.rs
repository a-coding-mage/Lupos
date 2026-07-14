//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/{pci.c,rebar.c}
//! test-origin: linux:vendor/linux/drivers/pci/{pci.c,rebar.c}
//! Generic PCI helper exports used by Linux-built PCI drivers.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EILSEQ, EINVAL, EIO, ENODEV, ENOENT, ENOTTY};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::LinuxListHead;
use crate::linux_driver_abi::pci::access::{
    PCIBIOS_SUCCESSFUL, pcie_capability_clear_and_set_word_locked,
};
use crate::linux_driver_abi::pci::device::{
    IORESOURCE_DISABLED, IORESOURCE_IO, IORESOURCE_MEM, IORESOURCE_PCI_FIXED, IORESOURCE_UNSET,
    LinuxPciBus, LinuxPciDev, LinuxResource, PCI_DEVICE_RESOURCE_COUNT, PCI_STD_NUM_BARS,
    linux_pci_bar_resource, linux_pci_config_read, linux_pci_config_write, linux_pci_device_state,
    linux_pci_slot_for_raw, linux_resource_request,
};

const PCI_COMMAND: usize = 0x04;
const PCI_COMMAND_IO: u16 = 0x1;
const PCI_COMMAND_MEMORY: u16 = 0x2;
const PCI_COMMAND_MASTER: u16 = 0x4;
const PCI_STATUS: usize = 0x06;
const PCI_STATUS_CAP_LIST: u16 = 0x10;
const PCI_STATUS_PARITY: u16 = 0x0100;
const PCI_STATUS_SIG_TARGET_ABORT: u16 = 0x0800;
const PCI_STATUS_REC_TARGET_ABORT: u16 = 0x1000;
const PCI_STATUS_REC_MASTER_ABORT: u16 = 0x2000;
const PCI_STATUS_SIG_SYSTEM_ERROR: u16 = 0x4000;
const PCI_STATUS_DETECTED_PARITY: u16 = 0x8000;
const PCI_STATUS_ERROR_BITS: u16 = PCI_STATUS_DETECTED_PARITY
    | PCI_STATUS_SIG_SYSTEM_ERROR
    | PCI_STATUS_REC_MASTER_ABORT
    | PCI_STATUS_REC_TARGET_ABORT
    | PCI_STATUS_SIG_TARGET_ABORT
    | PCI_STATUS_PARITY;
const PCI_CAPABILITY_LIST: usize = 0x34;
const PCI_CAP_LIST_NEXT: usize = 1;
const PCI_CAP_MIN: u8 = 0x40;
const PCI_FIND_CAP_TTL: usize = 48;
const PCI_CFG_SPACE_SIZE: u16 = 256;
const PCI_VPD_LRDT: u8 = 0x80;
const PCI_VPD_LRDT_RO_DATA: u8 = PCI_VPD_LRDT | 0x10;
const PCI_VPD_LRDT_TAG_SIZE: usize = 3;
const PCI_VPD_INFO_FLD_HDR_SIZE: usize = 3;
static PCI_VPD_RO_KEYWORD_CHKSUM: [u8; 3] = *b"RV\0";
const PCI_EXT_CAP_ID_REBAR: i32 = 0x15;
const PCI_REBAR_MIN_SIZE_LOG2: i32 = 20;
const PCI_REBAR_MAX_SIZE_LOG2: i32 = 47;
const PCI_REBAR_MAX_ENCODED_SIZE: i32 = PCI_REBAR_MAX_SIZE_LOG2 - PCI_REBAR_MIN_SIZE_LOG2;
const PCI_EXP_DEVCTL: i32 = 0x08;
const PCI_EXP_DEVCTL_READRQ: u16 = 0x7000;
const ENOTSUPP: i32 = 524;
const BIOS_END: u64 = 0x0010_0000;
const IORESOURCE_STARTALIGN: usize = 0x0008_0000;
const IORESOURCE_BUSY: usize = 0x8000_0000;
const PCI_BRIDGE_RESOURCES: usize = 7;
static PCI_IO_RESOURCE_NAME: [u8; 7] = *b"PCI IO\0";
static PCI_MEM_RESOURCE_NAME: [u8; 8] = *b"PCI mem\0";
static PCI_POWER_ERROR: [u8; 6] = *b"error\0";
static PCI_POWER_D0: [u8; 3] = *b"D0\0";
static PCI_POWER_D1: [u8; 3] = *b"D1\0";
static PCI_POWER_D2: [u8; 3] = *b"D2\0";
static PCI_POWER_D3HOT: [u8; 6] = *b"D3hot\0";
static PCI_POWER_D3COLD: [u8; 7] = *b"D3cold\0";
static PCI_POWER_UNKNOWN: [u8; 8] = *b"unknown\0";

#[repr(transparent)]
#[derive(Clone, Copy)]
struct StaticCStringPtr(*const c_char);

unsafe impl Sync for StaticCStringPtr {}

static PCI_POWER_NAMES: [StaticCStringPtr; 7] = [
    StaticCStringPtr(PCI_POWER_ERROR.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_D0.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_D1.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_D2.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_D3HOT.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_D3COLD.as_ptr().cast::<c_char>()),
    StaticCStringPtr(PCI_POWER_UNKNOWN.as_ptr().cast::<c_char>()),
];

type PciResourceAlignFn =
    unsafe extern "C" fn(*mut c_void, *const LinuxResource, *const LinuxResource, u64, u64) -> u64;

static mut LINUX_IOPORT_RESOURCE: LinuxResource = LinuxResource {
    start: 0,
    end: 0xffff,
    name: PCI_IO_RESOURCE_NAME.as_ptr().cast::<c_char>(),
    flags: IORESOURCE_IO,
    desc: 0,
    parent: core::ptr::null_mut(),
    sibling: core::ptr::null_mut(),
    child: core::ptr::null_mut(),
};

static mut LINUX_IOMEM_RESOURCE: LinuxResource = LinuxResource {
    start: 0,
    end: u64::MAX,
    name: PCI_MEM_RESOURCE_NAME.as_ptr().cast::<c_char>(),
    flags: IORESOURCE_MEM,
    desc: 0,
    parent: core::ptr::null_mut(),
    sibling: core::ptr::null_mut(),
    child: core::ptr::null_mut(),
};

static mut LINUX_INTEL_GRAPHICS_STOLEN_RES: LinuxResource = LinuxResource {
    start: 0,
    end: u64::MAX,
    name: core::ptr::null(),
    flags: IORESOURCE_MEM,
    desc: 0,
    parent: core::ptr::null_mut(),
    sibling: core::ptr::null_mut(),
    child: core::ptr::null_mut(),
};

#[repr(C)]
struct LinuxPciBusResource {
    list: LinuxListHead,
    res: *mut LinuxResource,
}

#[derive(Clone, Copy)]
struct PciResourceSpec {
    start: u64,
    end: u64,
    flags: usize,
    raw: *mut LinuxResource,
}

struct PciRegionReservation {
    start: u64,
    end: u64,
    resource_type: usize,
    raw: usize,
    parent: usize,
}

lazy_static! {
    /// Linux serializes `request_region()` / `release_region()` with
    /// `resource_lock`.  This table is the corresponding busy-resource state
    /// for driver reservations; PCI BAR descriptors themselves remain the
    /// non-busy parents published by enumeration.
    static ref PCI_REGION_RESERVATIONS: Mutex<Vec<PciRegionReservation>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    unsafe {
        export_symbol_once(
            "ioport_resource",
            core::ptr::addr_of_mut!(LINUX_IOPORT_RESOURCE) as usize,
            false,
        );
        export_symbol_once(
            "iomem_resource",
            core::ptr::addr_of_mut!(LINUX_IOMEM_RESOURCE) as usize,
            false,
        );
        export_symbol_once(
            "intel_graphics_stolen_res",
            core::ptr::addr_of_mut!(LINUX_INTEL_GRAPHICS_STOLEN_RES) as usize,
            false,
        );
    }
    export_symbol_once("pci_power_names", PCI_POWER_NAMES.as_ptr() as usize, true);
    export_symbol_once("pci_find_capability", pci_find_capability as usize, false);
    export_symbol_once(
        "pci_find_next_capability",
        pci_find_next_capability as usize,
        true,
    );
    export_symbol_once(
        "pci_find_next_ext_capability",
        pci_find_next_ext_capability as usize,
        true,
    );
    export_symbol_once(
        "pci_find_ext_capability",
        pci_find_ext_capability as usize,
        true,
    );
    export_symbol_once(
        "pci_rebar_bytes_to_size",
        pci_rebar_bytes_to_size as usize,
        true,
    );
    export_symbol_once(
        "pci_rebar_size_supported",
        pci_rebar_size_supported as usize,
        true,
    );
    export_symbol_once("pci_resize_resource", pci_resize_resource as usize, false);
    export_symbol_once("pci_enable_device", pci_enable_device as usize, false);
    export_symbol_once("pci_reenable_device", pci_reenable_device as usize, false);
    export_symbol_once(
        "pci_enable_device_mem",
        pci_enable_device_mem as usize,
        false,
    );
    export_symbol_once("pci_disable_device", pci_disable_device as usize, false);
    export_symbol_once("pci_set_master", pci_set_master as usize, false);
    export_symbol_once("pci_clear_master", pci_clear_master as usize, false);
    export_symbol_once("pci_set_mwi", pci_set_mwi as usize, false);
    export_symbol_once("pcim_set_mwi", pcim_set_mwi as usize, false);
    export_symbol_once("pci_clear_mwi", pci_clear_mwi as usize, false);
    export_symbol_once("pcix_get_mmrbc", pcix_get_mmrbc as usize, false);
    export_symbol_once("pcix_set_mmrbc", pcix_set_mmrbc as usize, false);
    export_symbol_once("pci_select_bars", pci_select_bars as usize, false);
    export_symbol_once("pci_bus_resource_n", pci_bus_resource_n as usize, true);
    export_symbol_once("pci_save_state", pci_save_state as usize, false);
    export_symbol_once("pci_restore_state", pci_restore_state as usize, false);
    export_symbol_once("pci_set_power_state", pci_set_power_state as usize, false);
    export_symbol_once("pci_enable_wake", pci_enable_wake as usize, false);
    export_symbol_once("pci_wake_from_d3", pci_wake_from_d3 as usize, false);
    export_symbol_once("pci_pme_capable", pci_pme_capable as usize, false);
    export_symbol_once("pci_pme_active", pci_pme_active as usize, false);
    export_symbol_once("pci_prepare_to_sleep", pci_prepare_to_sleep as usize, false);
    export_symbol_once("pcie_set_readrq", pcie_set_readrq as usize, false);
    export_symbol_once("pci_dev_run_wake", pci_dev_run_wake as usize, true);
    export_symbol_once(
        "pci_disable_link_state",
        pci_disable_link_state as usize,
        false,
    );
    export_symbol_once(
        "pci_disable_link_state_locked",
        pci_disable_link_state_locked as usize,
        false,
    );
    export_symbol_once("pci_read_vpd_any", pci_read_vpd_any as usize, false);
    export_symbol_once("pci_write_vpd_any", pci_write_vpd_any as usize, false);
    export_symbol_once("pci_vpd_alloc", pci_vpd_alloc as usize, true);
    export_symbol_once(
        "pci_vpd_find_ro_info_keyword",
        pci_vpd_find_ro_info_keyword as usize,
        true,
    );
    export_symbol_once("pci_vpd_check_csum", pci_vpd_check_csum as usize, true);
    export_symbol_once(
        "pci_status_get_and_clear_errors",
        pci_status_get_and_clear_errors as usize,
        true,
    );
    export_symbol_once("pci_d3cold_enable", pci_d3cold_enable as usize, true);
    export_symbol_once("pci_d3cold_disable", pci_d3cold_disable as usize, true);
    export_symbol_once("pci_reset_bus", pci_reset_bus as usize, true);
    export_symbol_once("pci_map_rom", pci_map_rom as usize, false);
    export_symbol_once("pci_unmap_rom", pci_unmap_rom as usize, false);
    export_symbol_once(
        "pci_device_is_present",
        pci_device_is_present as usize,
        true,
    );
    export_symbol_once(
        "pci_request_selected_regions",
        pci_request_selected_regions as usize,
        false,
    );
    export_symbol_once(
        "pci_request_selected_regions_exclusive",
        pci_request_selected_regions_exclusive as usize,
        false,
    );
    export_symbol_once(
        "pci_release_selected_regions",
        pci_release_selected_regions as usize,
        false,
    );
    export_symbol_once("pci_request_region", pci_request_region as usize, false);
    export_symbol_once("pci_release_region", pci_release_region as usize, false);
    export_symbol_once("pci_request_regions", pci_request_regions as usize, false);
    export_symbol_once("pci_release_regions", pci_release_regions as usize, false);
    export_symbol_once(
        "pci_request_regions_exclusive",
        pci_request_regions_exclusive as usize,
        false,
    );
    export_symbol_once(
        "pcibios_resource_to_bus",
        pcibios_resource_to_bus as usize,
        false,
    );
    export_symbol_once(
        "pcibios_align_resource",
        pcibios_align_resource as usize,
        false,
    );
    export_symbol_once("pci_assign_resource", pci_assign_resource as usize, false);
    export_symbol_once(
        "pci_assign_unassigned_bus_resources",
        pci_assign_unassigned_bus_resources as usize,
        true,
    );
    export_symbol_once(
        "pci_bus_alloc_resource",
        pci_bus_alloc_resource as usize,
        false,
    );
}

#[repr(C)]
pub struct PciBusRegion {
    start: u64,
    end: u64,
}

#[unsafe(export_name = "pcibios_resource_to_bus")]
pub unsafe extern "C" fn pcibios_resource_to_bus(
    _bus: *mut LinuxPciBus,
    region: *mut PciBusRegion,
    res: *const LinuxResource,
) {
    if region.is_null() || res.is_null() {
        return;
    }
    unsafe {
        (*region).start = (*res).start;
        (*region).end = (*res).end;
    }
}

/// `pcibios_align_resource` - `vendor/linux/arch/x86/pci/i386.c`.
#[unsafe(export_name = "pcibios_align_resource")]
pub unsafe extern "C" fn pcibios_align_resource(
    _data: *mut c_void,
    res: *const LinuxResource,
    _empty_res: *const LinuxResource,
    _size: u64,
    _align: u64,
) -> u64 {
    if res.is_null() {
        return 0;
    }
    let flags = unsafe { (*res).flags };
    let mut start = unsafe { (*res).start };
    if flags & IORESOURCE_IO != 0 {
        if start & 0x300 != 0 {
            start = start.saturating_add(0x3ff) & !0x3ff;
        }
    } else if flags & IORESOURCE_MEM != 0 && start < BIOS_END {
        start = BIOS_END;
    }
    start
}

/// `pci_assign_resource` - `vendor/linux/drivers/pci/setup-res.c`.
#[unsafe(export_name = "pci_assign_resource")]
pub unsafe extern "C" fn pci_assign_resource(dev: *mut LinuxPciDev, resno: i32) -> i32 {
    let Ok(resno) = usize::try_from(resno) else {
        return -EINVAL;
    };
    if dev.is_null() || resno >= PCI_DEVICE_RESOURCE_COUNT {
        return -EINVAL;
    }
    let res = unsafe { core::ptr::addr_of_mut!((*dev).resource[resno]) };
    unsafe {
        if (*res).flags & IORESOURCE_PCI_FIXED != 0 {
            return 0;
        }
        if (*res).flags & (IORESOURCE_IO | IORESOURCE_MEM) == 0
            || (*res).end == 0
            || (*res).end < (*res).start
        {
            return -EINVAL;
        }

        (*res).flags &= !IORESOURCE_UNSET;
        (*res).flags &= !IORESOURCE_STARTALIGN;
        if resno >= PCI_BRIDGE_RESOURCES {
            (*res).flags &= !IORESOURCE_DISABLED;
        }
    }
    0
}

/// `pci_assign_unassigned_bus_resources` - `vendor/linux/drivers/pci/setup-bus.c:2470`.
#[unsafe(export_name = "pci_assign_unassigned_bus_resources")]
pub unsafe extern "C" fn pci_assign_unassigned_bus_resources(bus: *mut LinuxPciBus) {
    unsafe { pci_assign_unassigned_bus_resources_one(bus, 0) };
}

unsafe fn pci_assign_unassigned_bus_resources_one(bus: *mut LinuxPciBus, depth: usize) {
    if bus.is_null() || depth > 32 {
        return;
    }

    let head = unsafe { core::ptr::addr_of_mut!((*bus).devices) };
    let mut node = unsafe { (*head).next.cast::<LinuxListHead>() };
    let mut remaining = 4096usize;
    while !node.is_null() && node != head && remaining != 0 {
        let dev = node.cast::<LinuxPciDev>();
        unsafe {
            for resno in 0..PCI_DEVICE_RESOURCE_COUNT {
                let res = core::ptr::addr_of_mut!((*dev).resource[resno]);
                if (*res).parent.is_null()
                    && (*res).flags & (IORESOURCE_IO | IORESOURCE_MEM) != 0
                    && (*res).end >= (*res).start
                    && (*res).end != 0
                {
                    let _ = pci_assign_resource(dev, resno as i32);
                }
            }

            let subordinate = (*dev).subordinate.cast::<LinuxPciBus>();
            if !subordinate.is_null() {
                pci_assign_unassigned_bus_resources_one(subordinate, depth + 1);
            }

            node = (*node).next.cast::<LinuxListHead>();
        }
        remaining -= 1;
    }
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align <= 1 {
        Some(value)
    } else {
        let mask = align.checked_sub(1)?;
        value.checked_add(mask).map(|v| v & !mask)
    }
}

unsafe fn candidate_bus_resource(
    bus: *mut LinuxPciBus,
    index: i32,
    flags: usize,
) -> *mut LinuxResource {
    let resource = unsafe { pci_bus_resource_n(bus, index) };
    if !resource.is_null()
        && unsafe {
            (*resource).flags & flags != 0
                && (*resource).flags & (IORESOURCE_UNSET | IORESOURCE_DISABLED) == 0
        }
    {
        return resource;
    }
    core::ptr::null_mut()
}

/// `pci_bus_alloc_resource` - `vendor/linux/drivers/pci/bus.c:264`.
#[unsafe(export_name = "pci_bus_alloc_resource")]
pub unsafe extern "C" fn pci_bus_alloc_resource(
    bus: *mut LinuxPciBus,
    res: *mut LinuxResource,
    size: u64,
    align: u64,
    min: u64,
    type_mask: usize,
    alignf: Option<PciResourceAlignFn>,
    alignf_data: *mut c_void,
) -> i32 {
    if bus.is_null() || res.is_null() || size == 0 {
        return -EINVAL;
    }

    let mut flags = unsafe { (*res).flags } & (IORESOURCE_IO | IORESOURCE_MEM);
    if flags == 0 {
        flags = type_mask & (IORESOURCE_IO | IORESOURCE_MEM);
    }
    if flags == 0 {
        return -EINVAL;
    }

    let fallback = unsafe {
        if flags & IORESOURCE_IO != 0 {
            core::ptr::addr_of_mut!(LINUX_IOPORT_RESOURCE)
        } else {
            core::ptr::addr_of_mut!(LINUX_IOMEM_RESOURCE)
        }
    };

    for idx in 0..8 {
        let mut parent = unsafe { candidate_bus_resource(bus, idx, flags) };
        if parent.is_null() && idx == 0 {
            parent = fallback;
        }
        if parent.is_null() {
            continue;
        }

        let parent_start = unsafe { (*parent).start };
        let parent_end = unsafe { (*parent).end };
        let Some(mut start) = align_up(parent_start.max(min), align.max(1)) else {
            continue;
        };

        unsafe {
            (*res).start = start;
            (*res).end = start.saturating_add(size.saturating_sub(1));
            (*res).flags = ((*res).flags | flags) & !IORESOURCE_UNSET & !IORESOURCE_DISABLED;
        }

        if let Some(alignf) = alignf {
            start = unsafe {
                alignf(
                    alignf_data,
                    res.cast_const(),
                    core::ptr::null(),
                    size,
                    align,
                )
            };
            let Some(aligned) = align_up(start.max(parent_start).max(min), align.max(1)) else {
                continue;
            };
            start = aligned;
            unsafe {
                (*res).start = start;
                (*res).end = start.saturating_add(size.saturating_sub(1));
            }
        }

        if unsafe { (*res).end >= (*res).start && (*res).end <= parent_end }
            && unsafe { linux_resource_request(parent, res) }
        {
            return 0;
        }
    }

    -EBUSY
}

/// `pci_bus_resource_n` - `vendor/linux/drivers/pci/bus.c:78`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_bus_resource_n(bus: *const LinuxPciBus, n: i32) -> *mut LinuxResource {
    if bus.is_null() || n < 0 {
        return core::ptr::null_mut();
    }
    let mut n = n as usize;
    if n < 4 {
        return unsafe { (*bus).resource[n] };
    }
    n -= 4;

    unsafe {
        let head = core::ptr::addr_of!((*bus).resources).cast_mut();
        let mut node = (*head).next.cast::<LinuxListHead>();
        let mut remaining = 4096usize;
        while node != head && !node.is_null() && remaining != 0 {
            let bus_resource = node.cast::<LinuxPciBusResource>();
            if n == 0 {
                return (*bus_resource).res;
            }
            n -= 1;
            node = (*node).next.cast::<LinuxListHead>();
            remaining -= 1;
        }
    }
    core::ptr::null_mut()
}

fn find_next_capability_in_config(config: &[u8], mut pos: u8, cap: i32) -> u8 {
    for _ in 0..PCI_FIND_CAP_TTL {
        pos &= !0x3;
        let offset = pos as usize;
        if pos < PCI_CAP_MIN || offset + PCI_CAP_LIST_NEXT >= config.len() {
            break;
        }
        if config[offset] == cap as u8 {
            return pos;
        }
        pos = config[offset + PCI_CAP_LIST_NEXT];
    }
    0
}

/// `pci_find_next_capability` - `vendor/linux/drivers/pci/pci.c:429`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_next_capability(dev: *mut c_void, pos: u8, cap: i32) -> u8 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    let next = pos
        .checked_add(PCI_CAP_LIST_NEXT as u8)
        .and_then(|offset| state.config_space.get(offset as usize).copied())
        .unwrap_or(0);
    find_next_capability_in_config(&state.config_space, next, cap)
}

/// `pci_find_capability` - `vendor/linux/drivers/pci/pci.c:475`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_capability(dev: *mut c_void, cap: i32) -> u8 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    let status = u16::from_le_bytes([
        state.config_space[PCI_STATUS],
        state.config_space[PCI_STATUS + 1],
    ]);
    if status & PCI_STATUS_CAP_LIST == 0 {
        return 0;
    }
    let pos = state.config_space[PCI_CAPABILITY_LIST];
    find_next_capability_in_config(&state.config_space, pos, cap)
}

/// `pci_find_next_ext_capability` - `vendor/linux/drivers/pci/pci.c:525`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_next_ext_capability(
    dev: *mut c_void,
    start: u16,
    _cap: i32,
) -> u16 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return 0;
    };
    if state.config_space.len() <= PCI_CFG_SPACE_SIZE as usize {
        return 0;
    }
    if start < PCI_CFG_SPACE_SIZE {
        PCI_CFG_SPACE_SIZE
    } else {
        0
    }
}

/// `pci_find_ext_capability` - `vendor/linux/drivers/pci/pci.c:549`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_find_ext_capability(dev: *mut c_void, cap: i32) -> u16 {
    unsafe { pci_find_next_ext_capability(dev, 0, cap) }
}

fn ilog2_u64(value: u64) -> i32 {
    (u64::BITS - 1 - value.leading_zeros()) as i32
}

/// `pci_rebar_bytes_to_size` - `vendor/linux/drivers/pci/rebar.c:18`.
#[unsafe(export_name = "pci_rebar_bytes_to_size")]
pub unsafe extern "C" fn pci_rebar_bytes_to_size(bytes: u64) -> i32 {
    let rounded = bytes
        .checked_next_power_of_two()
        .unwrap_or(1u64 << 63)
        .max(1);
    ilog2_u64(rounded).max(PCI_REBAR_MIN_SIZE_LOG2) - PCI_REBAR_MIN_SIZE_LOG2
}

/// `pci_rebar_size_supported` - `vendor/linux/drivers/pci/rebar.c:122`.
#[unsafe(export_name = "pci_rebar_size_supported")]
pub unsafe extern "C" fn pci_rebar_size_supported(dev: *mut c_void, bar: i32, size: i32) -> bool {
    if dev.is_null()
        || !(0..PCI_STD_NUM_BARS as i32).contains(&bar)
        || !(0..=PCI_REBAR_MAX_ENCODED_SIZE).contains(&size)
    {
        return false;
    }

    // Lupos does not yet model the PCIe Resizable BAR extended capability in
    // module-visible config state. Match Linux's no-capability path.
    let _ = unsafe { pci_find_ext_capability(dev, PCI_EXT_CAP_ID_REBAR) };
    false
}

/// `pci_resize_resource` - `vendor/linux/drivers/pci/rebar.c:298`.
#[unsafe(export_name = "pci_resize_resource")]
pub unsafe extern "C" fn pci_resize_resource(
    dev: *mut LinuxPciDev,
    resno: i32,
    size: i32,
    _exclude_bars: i32,
) -> i32 {
    if dev.is_null()
        || !(0..PCI_STD_NUM_BARS as i32).contains(&resno)
        || !(0..=PCI_REBAR_MAX_ENCODED_SIZE).contains(&size)
    {
        return -EINVAL;
    }

    if unsafe { linux_pci_config_read(dev.cast_const().cast(), PCI_COMMAND, 2) }
        .is_some_and(|cmd| cmd as u16 & PCI_COMMAND_MEMORY != 0)
    {
        return -EBUSY;
    }

    if !unsafe { pci_rebar_size_supported(dev.cast(), resno, size) } {
        return -EINVAL;
    }

    // Lupos currently exposes ReBAR as unsupported to modules. If a future
    // device model advertises ReBAR sizes, fail closed until resource release,
    // resize, and bus reassignment are implemented together.
    -ENOTSUPP
}

/// `pci_enable_device` - `vendor/linux/drivers/pci/pci.c:2122`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_device(dev: *mut c_void) -> i32 {
    let Some(state) = linux_pci_device_state(dev.cast_const()) else {
        return -EINVAL;
    };

    let mut enable = 0u16;
    for resource in state.bars.iter().flatten() {
        if resource.flags & IORESOURCE_IO != 0 {
            enable |= PCI_COMMAND_IO;
        }
        if resource.flags & IORESOURCE_MEM != 0 {
            enable |= PCI_COMMAND_MEMORY;
        }
    }
    if enable == 0 {
        return 0;
    }

    update_command_bits(dev.cast_const(), enable)
}

/// `pci_reenable_device` - `vendor/linux/drivers/pci/pci.c:2030`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_reenable_device(dev: *mut c_void) -> i32 {
    if linux_pci_device_state(dev.cast_const()).is_none() {
        return -EINVAL;
    }
    unsafe { pci_enable_device(dev) }
}

/// `pci_enable_device_mem` - `vendor/linux/drivers/pci/pci.c:2105`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_device_mem(dev: *mut c_void) -> i32 {
    unsafe { pci_enable_device(dev) }
}

/// `pci_disable_device` - `vendor/linux/drivers/pci/pci.c:2165`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_disable_device(_dev: *mut c_void) {}

/// `pci_set_master` - `vendor/linux/drivers/pci/pci.c:4140`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_master(dev: *mut c_void) {
    let _ = update_command_bits(dev.cast_const(), PCI_COMMAND_MASTER);
}

/// `pci_clear_master` - `vendor/linux/drivers/pci/pci.c:4187`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_clear_master(dev: *mut c_void) {
    let _ = clear_command_bits(dev.cast_const(), PCI_COMMAND_MASTER);
}

/// `pci_set_mwi` - `vendor/linux/drivers/pci/pci.c:4203`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_mwi(_dev: *mut c_void) -> i32 {
    0
}

/// `pcim_set_mwi` - `vendor/linux/drivers/pci/devres.c:278`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcim_set_mwi(dev: *mut c_void) -> i32 {
    unsafe { pci_set_mwi(dev) }
}

/// `pci_clear_mwi` - `vendor/linux/drivers/pci/pci.c:4251`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_clear_mwi(_dev: *mut c_void) {}

/// `pcix_get_mmrbc` - `vendor/linux/drivers/pci/pci.c:5719`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcix_get_mmrbc(_dev: *mut c_void) -> i32 {
    512
}

/// `pcix_set_mmrbc` - `vendor/linux/drivers/pci/pci.c:5744`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcix_set_mmrbc(_dev: *mut c_void, _mmrbc: i32) -> i32 {
    0
}

/// `pci_select_bars` - `vendor/linux/drivers/pci/pci.c:6130`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_select_bars(dev: *mut c_void, flags: usize) -> i32 {
    let mut bars = 0i32;
    for bar in 0..PCI_STD_NUM_BARS {
        if let Some(resource) = linux_pci_bar_resource(dev.cast_const(), bar) {
            if resource.flags & flags != 0 {
                bars |= 1 << bar;
            }
        }
    }
    bars
}

/// `pci_save_state` - `vendor/linux/drivers/pci/pci.c:1739`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_save_state(_dev: *mut c_void) -> i32 {
    0
}

/// `pci_restore_state` - `vendor/linux/drivers/pci/pci.c:1826`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_restore_state(_dev: *mut c_void) {}

/// `pci_set_power_state` - `vendor/linux/drivers/pci/pci.c:1596`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_power_state(_dev: *mut c_void, _state: i32) -> i32 {
    0
}

/// `pci_enable_wake` - `vendor/linux/drivers/pci/pci.c:2574`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_enable_wake(_dev: *mut c_void, _state: i32, _enable: bool) -> i32 {
    0
}

/// `pci_wake_from_d3` - `vendor/linux/drivers/pci/pci.c:2597`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_wake_from_d3(_dev: *mut c_void, _enable: bool) -> i32 {
    0
}

/// `pci_pme_capable` - `vendor/linux/drivers/pci/pci.c:2328`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_pme_capable(_dev: *mut c_void, _state: i32) -> bool {
    false
}

/// `pci_pme_active` - `vendor/linux/drivers/pci/pci.c:2477`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_pme_active(_dev: *mut c_void, _enable: bool) {}

/// `pci_prepare_to_sleep` - `vendor/linux/drivers/pci/pci.c:2672`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_prepare_to_sleep(_dev: *mut c_void) -> i32 {
    0
}

/// `pcie_set_readrq` - `vendor/linux/drivers/pci/pci.c:5837`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcie_set_readrq(dev: *mut c_void, rq: i32) -> i32 {
    if !(128..=4096).contains(&rq) || rq & (rq - 1) != 0 {
        return -EINVAL;
    }
    let firstbit = rq.trailing_zeros() + 1;
    if firstbit < 8 {
        return -EINVAL;
    }
    let value = ((firstbit - 8) as u16) << 12;
    let ret = unsafe {
        pcie_capability_clear_and_set_word_locked(
            dev.cast_const(),
            PCI_EXP_DEVCTL,
            PCI_EXP_DEVCTL_READRQ,
            value,
        )
    };
    if ret == PCIBIOS_SUCCESSFUL {
        0
    } else {
        -EINVAL
    }
}

/// `pci_dev_run_wake` - `vendor/linux/drivers/pci/pci.c:2761`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_dev_run_wake(_dev: *mut c_void) -> bool {
    false
}

/// `pci_disable_link_state_locked` - `vendor/linux/drivers/pci/pcie/aspm.c:1477`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_disable_link_state_locked(_dev: *mut c_void, _state: i32) {}

/// `pci_disable_link_state` - `vendor/linux/drivers/pci/pcie/aspm.c:1492`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_disable_link_state(_dev: *mut c_void, _state: i32) {}

/// `pci_read_vpd_any` - `vendor/linux/drivers/pci/vpd.c:449`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_read_vpd_any(
    _dev: *mut c_void,
    _pos: i64,
    _count: usize,
    _buf: *mut c_void,
) -> isize {
    0
}

/// `pci_write_vpd_any` - `vendor/linux/drivers/pci/vpd.c:487`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_write_vpd_any(
    _dev: *mut c_void,
    _pos: i64,
    _count: usize,
    _buf: *const c_void,
) -> isize {
    0
}

fn pci_vpd_find_lrdt_tag(buf: &[u8], rdt: u8) -> Result<(usize, usize), i32> {
    let mut offset = 0usize;
    while offset + PCI_VPD_LRDT_TAG_SIZE <= buf.len() && buf[offset] & PCI_VPD_LRDT != 0 {
        let tag = buf[offset];
        let len = u16::from_le_bytes([buf[offset + 1], buf[offset + 2]]) as usize;
        offset += PCI_VPD_LRDT_TAG_SIZE;
        if tag == rdt {
            return Ok((offset, len.min(buf.len() - offset)));
        }
        let Some(next) = offset.checked_add(len) else {
            return Err(-EINVAL);
        };
        offset = next;
    }
    Err(-ENOENT)
}

fn pci_vpd_find_info_keyword(
    buf: &[u8],
    offset: usize,
    len: usize,
    kw: [u8; 2],
) -> Result<usize, i32> {
    let end = offset.checked_add(len).ok_or(-EINVAL)?.min(buf.len());
    let mut cursor = offset;
    while cursor + PCI_VPD_INFO_FLD_HDR_SIZE <= end {
        if buf[cursor] == kw[0] && buf[cursor + 1] == kw[1] {
            return Ok(cursor);
        }
        let field_len = buf[cursor + 2] as usize;
        cursor = cursor
            .checked_add(PCI_VPD_INFO_FLD_HDR_SIZE + field_len)
            .ok_or(-EINVAL)?;
    }
    Err(-ENOENT)
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

/// `pci_vpd_alloc` - `vendor/linux/drivers/pci/vpd.c:343`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_vpd_alloc(_dev: *mut c_void, size: *mut u32) -> *mut c_void {
    if !size.is_null() {
        unsafe { *size = 0 };
    }
    err_ptr(ENODEV)
}

/// `pci_vpd_find_ro_info_keyword` - `vendor/linux/drivers/pci/vpd.c:493`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_vpd_find_ro_info_keyword(
    buf: *const c_void,
    len: u32,
    kw: *const c_char,
    size: *mut u32,
) -> i32 {
    if buf.is_null() || kw.is_null() {
        return -EINVAL;
    }
    let data = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), len as usize) };
    let keyword = unsafe { [*kw.cast::<u8>(), *kw.cast::<u8>().add(1)] };
    let (ro_start, ro_len) = match pci_vpd_find_lrdt_tag(data, PCI_VPD_LRDT_RO_DATA) {
        Ok(result) => result,
        Err(errno) => return errno,
    };
    let info_start = match pci_vpd_find_info_keyword(data, ro_start, ro_len, keyword) {
        Ok(start) => start,
        Err(errno) => return errno,
    };
    let info_size = data[info_start + 2] as usize;
    let value_start = info_start + PCI_VPD_INFO_FLD_HDR_SIZE;
    if value_start
        .checked_add(info_size)
        .is_none_or(|end| end > data.len())
    {
        return -EINVAL;
    }
    if !size.is_null() {
        unsafe { *size = info_size as u32 };
    }
    if value_start > i32::MAX as usize {
        -EINVAL
    } else {
        value_start as i32
    }
}

/// `pci_vpd_check_csum` - `vendor/linux/drivers/pci/vpd.c:520`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_vpd_check_csum(buf: *const c_void, len: u32) -> i32 {
    if buf.is_null() {
        return -EINVAL;
    }

    let mut size = 0u32;
    let rv_start = unsafe {
        pci_vpd_find_ro_info_keyword(
            buf,
            len,
            PCI_VPD_RO_KEYWORD_CHKSUM.as_ptr().cast(),
            &mut size,
        )
    };
    if rv_start == -ENOENT {
        return 1;
    }
    if rv_start < 0 {
        return rv_start;
    }
    if size == 0 {
        return -EINVAL;
    }

    let data = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), len as usize) };
    let mut csum = 0u8;
    for byte in data[..=rv_start as usize].iter().rev() {
        csum = csum.wrapping_add(*byte);
    }
    if csum == 0 { 0 } else { -EILSEQ }
}

/// `pci_d3cold_enable` - `vendor/linux/drivers/pci/pci.c:3125`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_d3cold_enable(_dev: *mut c_void) {}

/// `pci_status_get_and_clear_errors` - `vendor/linux/drivers/pci/pci.c:199`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_status_get_and_clear_errors(pdev: *mut c_void) -> i32 {
    let Some(status) = linux_pci_config_read(pdev.cast_const(), PCI_STATUS, 2) else {
        return -EIO;
    };
    let status = (status as u16) & PCI_STATUS_ERROR_BITS;
    if status != 0 {
        let _ = linux_pci_config_write(pdev.cast_const(), PCI_STATUS, 2, status as u32);
    }
    status as i32
}

/// `pci_d3cold_disable` - `vendor/linux/drivers/pci/pci.c:3142`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_d3cold_disable(_dev: *mut c_void) {}

/// `pci_reset_bus` - `vendor/linux/drivers/pci/pci.c:5714`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_reset_bus(_pdev: *mut c_void) -> i32 {
    -ENOTTY
}

/// `pci_map_rom` - `vendor/linux/drivers/pci/rom.c:239`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_map_rom(_pdev: *mut LinuxPciDev, size: *mut usize) -> *mut c_void {
    if !size.is_null() {
        unsafe {
            *size = 0;
        }
    }
    core::ptr::null_mut()
}

/// `pci_unmap_rom` - `vendor/linux/drivers/pci/rom.c:290`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_unmap_rom(_pdev: *mut LinuxPciDev, _rom: *mut c_void) {}

fn update_command_bits(dev: *const c_void, bits: u16) -> i32 {
    let Some(command) = linux_pci_config_read(dev, PCI_COMMAND, 2) else {
        return -EINVAL;
    };
    let command = (command as u16) | bits;
    if linux_pci_config_write(dev, PCI_COMMAND, 2, command as u32) {
        0
    } else {
        -EINVAL
    }
}

fn clear_command_bits(dev: *const c_void, bits: u16) -> i32 {
    let Some(command) = linux_pci_config_read(dev, PCI_COMMAND, 2) else {
        return -EINVAL;
    };
    let command = (command as u16) & !bits;
    if linux_pci_config_write(dev, PCI_COMMAND, 2, command as u32) {
        0
    } else {
        -EINVAL
    }
}

/// `pci_device_is_present` - `vendor/linux/drivers/pci/pci.c:6296`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_device_is_present(dev: *mut c_void) -> bool {
    linux_pci_device_state(dev.cast_const()).is_some()
}

fn pci_resource_spec(dev: *mut c_void, bar: i32) -> Result<PciResourceSpec, i32> {
    let Ok(bar) = usize::try_from(bar) else {
        return Err(EINVAL);
    };
    if bar >= PCI_DEVICE_RESOURCE_COUNT || linux_pci_device_state(dev.cast_const()).is_none() {
        return Err(EINVAL);
    }

    if linux_pci_slot_for_raw(dev.cast_const()).is_some() {
        let raw = unsafe { core::ptr::addr_of_mut!((*dev.cast::<LinuxPciDev>()).resource[bar]) };
        return Ok(PciResourceSpec {
            start: unsafe { (*raw).start },
            end: unsafe { (*raw).end },
            flags: unsafe { (*raw).flags },
            raw,
        });
    }

    // Synthetic PCI state used by the existing vendor-derived host tests has
    // no complete `struct pci_dev` allocation.  Its standard BAR snapshot is
    // nevertheless sufficient for the same reservation semantics.
    let resource = (bar < PCI_STD_NUM_BARS)
        .then(|| linux_pci_bar_resource(dev.cast_const(), bar))
        .flatten();
    Ok(match resource {
        Some(resource) => PciResourceSpec {
            start: resource.start,
            end: resource.end(),
            flags: resource.flags,
            raw: core::ptr::null_mut(),
        },
        None => PciResourceSpec {
            start: 0,
            end: 0,
            flags: 0,
            raw: core::ptr::null_mut(),
        },
    })
}

fn resource_len(resource: PciResourceSpec) -> u64 {
    if resource.end == 0 || resource.end < resource.start {
        0
    } else {
        resource.end - resource.start + 1
    }
}

unsafe fn link_busy_resource(parent: *mut LinuxResource, child: *mut LinuxResource) {
    if parent.is_null() {
        return;
    }
    unsafe {
        (*child).parent = parent;
        (*child).sibling = (*parent).child;
        (*parent).child = child;
    }
}

unsafe fn unlink_busy_resource(parent: *mut LinuxResource, child: *mut LinuxResource) {
    if parent.is_null() || child.is_null() {
        return;
    }
    unsafe {
        let mut link = core::ptr::addr_of_mut!((*parent).child);
        while !(*link).is_null() {
            if *link == child {
                *link = (*child).sibling;
                (*child).parent = core::ptr::null_mut();
                (*child).sibling = core::ptr::null_mut();
                return;
            }
            link = core::ptr::addr_of_mut!((**link).sibling);
        }
    }
}

unsafe fn request_pci_resource(dev: *mut c_void, bar: i32, name: *const c_char) -> Result<(), i32> {
    let resource = pci_resource_spec(dev, bar)?;
    if resource_len(resource) == 0 {
        return Ok(());
    }
    let resource_type = resource.flags & (IORESOURCE_IO | IORESOURCE_MEM);
    if resource_type == 0 {
        return Ok(());
    }

    let mut reservations = PCI_REGION_RESERVATIONS.lock();
    if reservations.iter().any(|busy| {
        busy.resource_type == resource_type
            && busy.start <= resource.end
            && resource.start <= busy.end
    }) {
        return Err(EBUSY);
    }

    let busy = Box::new(LinuxResource {
        start: resource.start,
        end: resource.end,
        name,
        flags: resource_type | IORESOURCE_BUSY,
        desc: if resource.raw.is_null() {
            0
        } else {
            unsafe { (*resource.raw).desc }
        },
        parent: core::ptr::null_mut(),
        sibling: core::ptr::null_mut(),
        child: core::ptr::null_mut(),
    });
    let busy = Box::into_raw(busy);
    unsafe { link_busy_resource(resource.raw, busy) };
    reservations.push(PciRegionReservation {
        start: resource.start,
        end: resource.end,
        resource_type,
        raw: busy as usize,
        parent: resource.raw as usize,
    });
    Ok(())
}

unsafe fn release_pci_resource(dev: *mut c_void, bar: i32) {
    let Ok(resource) = pci_resource_spec(dev, bar) else {
        return;
    };
    if resource_len(resource) == 0 {
        return;
    }
    let resource_type = resource.flags & (IORESOURCE_IO | IORESOURCE_MEM);
    if resource_type == 0 {
        return;
    }

    let mut reservations = PCI_REGION_RESERVATIONS.lock();
    let Some(index) = reservations.iter().position(|busy| {
        busy.resource_type == resource_type
            && busy.start == resource.start
            && busy.end == resource.end
    }) else {
        return;
    };
    let busy = reservations.swap_remove(index);
    let raw = busy.raw as *mut LinuxResource;
    unsafe { unlink_busy_resource(busy.parent as *mut LinuxResource, raw) };
    unsafe {
        drop(Box::from_raw(raw));
    }
}

/// `pci_request_region` - `vendor/linux/drivers/pci/pci.c:3864`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_region(
    dev: *mut c_void,
    bar: i32,
    name: *const c_char,
) -> i32 {
    match unsafe { request_pci_resource(dev, bar, name) } {
        Ok(()) => 0,
        Err(errno) => -errno,
    }
}

/// `pci_release_region` - `vendor/linux/drivers/pci/pci.c:3785`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_release_region(dev: *mut c_void, bar: i32) {
    unsafe { release_pci_resource(dev, bar) };
}

/// `pci_request_selected_regions` - `vendor/linux/drivers/pci/pci.c:3884`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_selected_regions(
    dev: *mut c_void,
    bars: i32,
    name: *const c_char,
) -> i32 {
    for bar in 0..PCI_STD_NUM_BARS as i32 {
        if bars & (1 << bar) == 0 {
            continue;
        }
        if unsafe { pci_request_region(dev, bar, name) } != 0 {
            for rollback in (0..bar).rev() {
                if bars & (1 << rollback) != 0 {
                    unsafe { pci_release_region(dev, rollback) };
                }
            }
            return -EBUSY;
        }
    }
    0
}

/// `pci_request_selected_regions_exclusive` - `vendor/linux/drivers/pci/pci.c:3931`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_selected_regions_exclusive(
    dev: *mut c_void,
    bars: i32,
    name: *const c_char,
) -> i32 {
    unsafe { pci_request_selected_regions(dev, bars, name) }
}

/// `pci_release_selected_regions` - `vendor/linux/drivers/pci/pci.c:3846`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_release_selected_regions(dev: *mut c_void, bars: i32) {
    for bar in 0..PCI_STD_NUM_BARS as i32 {
        if bars & (1 << bar) != 0 {
            unsafe { pci_release_region(dev, bar) };
        }
    }
}

/// `pci_release_regions` - `vendor/linux/drivers/pci/pci.c:3948`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_release_regions(dev: *mut c_void) {
    let bars = unsafe { pci_select_bars(dev, IORESOURCE_IO | IORESOURCE_MEM) };
    unsafe { pci_release_selected_regions(dev, bars) };
}

/// `pci_request_regions` - `vendor/linux/drivers/pci/pci.c:3966`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_regions(dev: *mut c_void, name: *const c_char) -> i32 {
    let bars = unsafe { pci_select_bars(dev, IORESOURCE_IO | IORESOURCE_MEM) };
    unsafe { pci_request_selected_regions(dev, bars, name) }
}

/// `pci_request_regions_exclusive` - `vendor/linux/drivers/pci/pci.c:3990`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_request_regions_exclusive(
    dev: *mut c_void,
    name: *const c_char,
) -> i32 {
    let bars = unsafe { pci_select_bars(dev, IORESOURCE_IO | IORESOURCE_MEM) };
    unsafe { pci_request_selected_regions_exclusive(dev, bars, name) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::pci::device::{
        IORESOURCE_MEM, LinuxPciBarResource, LinuxPciDeviceAbiState, PCI_CONFIG_SPACE_SIZE,
        PCI_STD_NUM_BARS, PciBar, PciDev, register_linux_pci_device,
        register_linux_pci_device_state, registered_linux_pci_bus,
        unregister_linux_pci_device_state,
    };

    #[test]
    fn pci_helper_exports_module_symbols() {
        let pci_source = include_str!("../../../vendor/linux/drivers/pci/pci.c");
        let vpd_source = include_str!("../../../vendor/linux/drivers/pci/vpd.c");
        let devres_source = include_str!("../../../vendor/linux/drivers/pci/devres.c");
        assert!(pci_source.contains("EXPORT_SYMBOL(pcie_set_readrq);"));
        assert!(pci_source.contains("EXPORT_SYMBOL_GPL(pci_status_get_and_clear_errors);"));
        assert!(pci_source.contains("EXPORT_SYMBOL_GPL(pci_reset_bus);"));
        assert!(vpd_source.contains("EXPORT_SYMBOL_GPL(pci_vpd_alloc);"));
        assert!(vpd_source.contains("EXPORT_SYMBOL_GPL(pci_vpd_find_ro_info_keyword);"));
        assert!(vpd_source.contains("EXPORT_SYMBOL_GPL(pci_vpd_check_csum);"));
        assert!(devres_source.contains("EXPORT_SYMBOL(pcim_set_mwi);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("pci_find_capability"),
            Some(pci_find_capability as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_request_selected_regions"),
            Some(pci_request_selected_regions as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_request_selected_regions_exclusive"),
            Some(pci_request_selected_regions_exclusive as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_request_regions"),
            Some(pci_request_regions as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_release_regions"),
            Some(pci_release_regions as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_find_ext_capability"),
            Some(pci_find_ext_capability as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_rebar_bytes_to_size"),
            Some(pci_rebar_bytes_to_size as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_rebar_size_supported"),
            Some(pci_rebar_size_supported as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_resize_resource"),
            Some(pci_resize_resource as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_enable_device"),
            Some(pci_enable_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_clear_master"),
            Some(pci_clear_master as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_reenable_device"),
            Some(pci_reenable_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_device_is_present"),
            Some(pci_device_is_present as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_d3cold_enable"),
            Some(pci_d3cold_enable as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_disable_link_state"),
            Some(pci_disable_link_state as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_disable_link_state_locked"),
            Some(pci_disable_link_state_locked as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_pme_capable"),
            Some(pci_pme_capable as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcim_set_mwi"),
            Some(pcim_set_mwi as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_dev_run_wake"),
            Some(pci_dev_run_wake as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcie_set_readrq"),
            Some(pcie_set_readrq as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("pcie_set_readrq"),
            Some(false)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_read_vpd_any"),
            Some(pci_read_vpd_any as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_vpd_alloc"),
            Some(pci_vpd_alloc as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("pci_vpd_alloc"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_vpd_find_ro_info_keyword"),
            Some(pci_vpd_find_ro_info_keyword as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_vpd_check_csum"),
            Some(pci_vpd_check_csum as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("pci_vpd_check_csum"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_status_get_and_clear_errors"),
            Some(pci_status_get_and_clear_errors as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("pci_status_get_and_clear_errors"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_reset_bus"),
            Some(pci_reset_bus as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("pci_reset_bus"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_assign_unassigned_bus_resources"),
            Some(pci_assign_unassigned_bus_resources as usize)
        );
    }

    #[test]
    fn pci_status_get_and_clear_errors_returns_and_clears_error_bits() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        let status =
            PCI_STATUS_CAP_LIST | PCI_STATUS_SIG_TARGET_ABORT | PCI_STATUS_REC_MASTER_ABORT;
        state.config_space[PCI_STATUS..PCI_STATUS + 2].copy_from_slice(&status.to_le_bytes());
        register_linux_pci_device_state(dev, state);

        assert_eq!(
            unsafe { pci_status_get_and_clear_errors(dev) },
            (PCI_STATUS_SIG_TARGET_ABORT | PCI_STATUS_REC_MASTER_ABORT) as i32
        );
        assert_eq!(
            linux_pci_config_read(dev, PCI_STATUS, 2),
            Some(PCI_STATUS_CAP_LIST as u32)
        );
        assert_eq!(
            unsafe { pci_status_get_and_clear_errors(core::ptr::null_mut()) },
            -EIO
        );

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_reset_bus_reports_unsupported_until_reset_paths_are_modeled() {
        assert_eq!(unsafe { pci_reset_bus(core::ptr::null_mut()) }, -ENOTTY);
    }

    #[test]
    fn pcie_set_readrq_rejects_invalid_request_sizes() {
        assert_eq!(
            unsafe { pcie_set_readrq(core::ptr::null_mut(), 64) },
            -EINVAL
        );
        assert_eq!(
            unsafe { pcie_set_readrq(core::ptr::null_mut(), 192) },
            -EINVAL
        );
        assert_eq!(
            unsafe { pcie_set_readrq(core::ptr::null_mut(), 8192) },
            -EINVAL
        );
    }

    #[test]
    fn pci_vpd_alloc_reports_vpd_unavailable() {
        let mut size = u32::MAX;
        let ptr = unsafe { pci_vpd_alloc(core::ptr::null_mut(), &mut size) };

        assert_eq!(ptr as isize, -(ENODEV as isize));
        assert_eq!(size, 0);
    }

    #[test]
    fn pci_vpd_check_csum_matches_vendor_vpd_checksum_rules() {
        let mut vpd = [PCI_VPD_LRDT_RO_DATA, 4, 0, b'R', b'V', 1, 0xc3];

        assert_eq!(
            unsafe { pci_vpd_check_csum(vpd.as_ptr().cast(), vpd.len() as u32) },
            0
        );
        vpd[6] = 0;
        assert_eq!(
            unsafe { pci_vpd_check_csum(vpd.as_ptr().cast(), vpd.len() as u32) },
            -EILSEQ
        );

        let no_checksum = [PCI_VPD_LRDT_RO_DATA, 3, 0, b'P', b'N', 0];
        assert_eq!(
            unsafe { pci_vpd_check_csum(no_checksum.as_ptr().cast(), no_checksum.len() as u32) },
            1
        );
    }

    #[test]
    fn pci_vpd_ro_info_keyword_returns_value_offset_and_size() {
        let vpd = [
            PCI_VPD_LRDT_RO_DATA,
            8,
            0,
            b'P',
            b'N',
            4,
            b't',
            b'e',
            b's',
            b't',
        ];
        let mut size = 0u32;

        let offset = unsafe {
            pci_vpd_find_ro_info_keyword(
                vpd.as_ptr().cast(),
                vpd.len() as u32,
                b"PN\0".as_ptr().cast(),
                &mut size,
            )
        };

        assert_eq!(offset, 6);
        assert_eq!(size, 4);
    }

    #[test]
    fn pci_capability_walk_matches_standard_list_layout() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        state.config_space[PCI_STATUS..PCI_STATUS + 2]
            .copy_from_slice(&PCI_STATUS_CAP_LIST.to_le_bytes());
        state.config_space[PCI_CAPABILITY_LIST] = 0x40;
        state.config_space[0x40] = 0x09;
        state.config_space[0x41] = 0x50;
        state.config_space[0x50] = 0x05;
        state.config_space[0x51] = 0;
        register_linux_pci_device_state(dev, state);

        unsafe {
            assert_eq!(pci_find_capability(dev, 0x09), 0x40);
            assert_eq!(pci_find_next_capability(dev, 0x40, 0x05), 0x50);
            assert_eq!(pci_find_ext_capability(dev, 0x10), 0);
            assert_eq!(pci_enable_device(dev), 0);
            assert!(pci_device_is_present(dev));
            pci_set_master(dev);
            pci_disable_device(dev);
            assert_eq!(pci_find_capability(dev, 0xff), 0);
            assert_eq!(
                pci_request_selected_regions(dev, 0x3f, core::ptr::null()),
                0
            );
        }

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_assign_unassigned_bus_resources_visits_registered_bus_devices() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/pci/setup-bus.c"
        ));
        assert!(source.contains("void pci_assign_unassigned_bus_resources(struct pci_bus *bus)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(pci_assign_unassigned_bus_resources);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("pci_assign_unassigned_bus_resources"),
            Some(pci_assign_unassigned_bus_resources as usize)
        );

        crate::linux_driver_abi::pci::driver::register_module_exports();
        let mut bars = [None; 6];
        bars[0] = Some(PciBar {
            base: 0x9000_0000,
            size: 0x1000,
            is_mmio: true,
            is_64bit: false,
            prefetchable: false,
        });
        let pdev = PciDev::new_with_subsystem_and_bars(
            0x7f7d, 7, 1, 0, 0x8086, 0x1234, 0x03, 0x00, 0, 1, 0, 0, bars,
        );
        let raw = register_linux_pci_device(
            &pdev,
            crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr(),
        );
        let bus = registered_linux_pci_bus(0x7f7d, 7);
        assert!(!raw.is_null());
        assert!(!bus.is_null());

        unsafe {
            (*raw).resource[0].flags |= IORESOURCE_UNSET | IORESOURCE_STARTALIGN;
            (*raw).resource[0].parent = core::ptr::null_mut();
            pci_assign_unassigned_bus_resources(bus);
            assert_eq!((*raw).resource[0].flags & IORESOURCE_UNSET, 0);
            assert_eq!((*raw).resource[0].flags & IORESOURCE_STARTALIGN, 0);
        }
    }

    #[test]
    fn pci_rebar_size_supported_matches_no_capability_path() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/pci/rebar.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL_GPL(pci_rebar_bytes_to_size);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(pci_rebar_size_supported);"));
        assert!(source.contains("EXPORT_SYMBOL(pci_resize_resource);"));

        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        register_linux_pci_device_state(dev, state);

        unsafe {
            assert!(!pci_rebar_size_supported(dev, 0, 0));
            assert!(!pci_rebar_size_supported(dev, -1, 0));
            assert!(!pci_rebar_size_supported(
                dev,
                0,
                PCI_REBAR_MAX_ENCODED_SIZE + 1
            ));
            assert!(!pci_rebar_size_supported(core::ptr::null_mut(), 0, 0));
        }

        unregister_linux_pci_device_state(dev);
    }

    #[test]
    fn pci_rebar_bytes_to_size_rounds_like_linux() {
        unsafe {
            assert_eq!(pci_rebar_bytes_to_size(0), 0);
            assert_eq!(pci_rebar_bytes_to_size(1 << PCI_REBAR_MIN_SIZE_LOG2), 0);
            assert_eq!(
                pci_rebar_bytes_to_size((1 << PCI_REBAR_MIN_SIZE_LOG2) + 1),
                1
            );
            assert_eq!(
                pci_rebar_bytes_to_size(1 << (PCI_REBAR_MIN_SIZE_LOG2 + 3)),
                3
            );
        }
    }

    #[test]
    fn pci_resize_resource_fails_closed_without_rebar_capability() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<LinuxPciDev>();
        let state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        register_linux_pci_device_state(dev.cast(), state);

        unsafe {
            assert_eq!(pci_resize_resource(dev, 0, 0, 0), -EINVAL);
            assert_eq!(pci_resize_resource(dev, -1, 0, 0), -EINVAL);
            assert_eq!(
                pci_resize_resource(dev, 0, PCI_REBAR_MAX_ENCODED_SIZE + 1, 0),
                -EINVAL
            );
            assert_eq!(pci_resize_resource(core::ptr::null_mut(), 0, 0, 0), -EINVAL);
        }

        unregister_linux_pci_device_state(dev.cast());
    }

    #[test]
    fn pci_resize_resource_rejects_enabled_memory_decode() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<LinuxPciDev>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        state.config_space[PCI_COMMAND..PCI_COMMAND + 2]
            .copy_from_slice(&PCI_COMMAND_MEMORY.to_le_bytes());
        register_linux_pci_device_state(dev.cast(), state);

        unsafe {
            assert_eq!(pci_resize_resource(dev, 0, 0, 0), -EBUSY);
        }

        unregister_linux_pci_device_state(dev.cast());
    }

    #[test]
    fn pci_enable_device_and_set_master_update_command_bits() {
        let mut token = 0u8;
        let dev = (&mut token as *mut u8).cast::<c_void>();
        let mut state = LinuxPciDeviceAbiState {
            config_space: [0; PCI_CONFIG_SPACE_SIZE],
            bars: [None; PCI_STD_NUM_BARS],
        };
        state.bars[5] = Some(LinuxPciBarResource {
            start: 0xfebd_0000,
            len: 0x1000,
            flags: IORESOURCE_MEM,
        });
        register_linux_pci_device_state(dev, state);

        unsafe {
            assert_eq!(pci_enable_device(dev), 0);
            assert_eq!(
                linux_pci_config_read(dev, PCI_COMMAND, 2),
                Some(PCI_COMMAND_MEMORY as u32)
            );
            pci_set_master(dev);
            assert_eq!(
                linux_pci_config_read(dev, PCI_COMMAND, 2),
                Some((PCI_COMMAND_MEMORY | PCI_COMMAND_MASTER) as u32)
            );
        }

        unregister_linux_pci_device_state(dev);
    }
}
