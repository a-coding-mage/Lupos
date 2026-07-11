//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/pci.c
//! test-origin: linux:vendor/linux/drivers/pci/pci.c
//! Generic PCI helper exports used by Linux-built PCI drivers.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::LinuxListHead;
use crate::linux_driver_abi::pci::device::{
    IORESOURCE_IO, IORESOURCE_MEM, LinuxPciBus, LinuxPciDev, LinuxResource,
    PCI_DEVICE_RESOURCE_COUNT, PCI_STD_NUM_BARS, linux_pci_bar_resource, linux_pci_config_read,
    linux_pci_config_write, linux_pci_device_state, linux_pci_slot_for_raw,
};

const PCI_COMMAND: usize = 0x04;
const PCI_COMMAND_IO: u16 = 0x1;
const PCI_COMMAND_MEMORY: u16 = 0x2;
const PCI_COMMAND_MASTER: u16 = 0x4;
const PCI_STATUS: usize = 0x06;
const PCI_STATUS_CAP_LIST: u16 = 0x10;
const PCI_CAPABILITY_LIST: usize = 0x34;
const PCI_CAP_LIST_NEXT: usize = 1;
const PCI_CAP_MIN: u8 = 0x40;
const PCI_FIND_CAP_TTL: usize = 48;
const PCI_CFG_SPACE_SIZE: u16 = 256;
const IORESOURCE_BUSY: usize = 0x8000_0000;
static PCI_IO_RESOURCE_NAME: [u8; 7] = *b"PCI IO\0";
static PCI_MEM_RESOURCE_NAME: [u8; 8] = *b"PCI mem\0";

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
    }
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
    export_symbol_once("pci_enable_device", pci_enable_device as usize, false);
    export_symbol_once(
        "pci_enable_device_mem",
        pci_enable_device_mem as usize,
        false,
    );
    export_symbol_once("pci_disable_device", pci_disable_device as usize, false);
    export_symbol_once("pci_set_master", pci_set_master as usize, false);
    export_symbol_once("pci_set_mwi", pci_set_mwi as usize, false);
    export_symbol_once("pci_clear_mwi", pci_clear_mwi as usize, false);
    export_symbol_once("pcix_get_mmrbc", pcix_get_mmrbc as usize, false);
    export_symbol_once("pcix_set_mmrbc", pcix_set_mmrbc as usize, false);
    export_symbol_once("pci_select_bars", pci_select_bars as usize, false);
    export_symbol_once("pci_bus_resource_n", pci_bus_resource_n as usize, true);
    export_symbol_once("pci_save_state", pci_save_state as usize, false);
    export_symbol_once("pci_set_power_state", pci_set_power_state as usize, false);
    export_symbol_once("pci_enable_wake", pci_enable_wake as usize, false);
    export_symbol_once("pci_wake_from_d3", pci_wake_from_d3 as usize, false);
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
        "pci_release_selected_regions",
        pci_release_selected_regions as usize,
        false,
    );
    export_symbol_once("pci_request_region", pci_request_region as usize, false);
    export_symbol_once("pci_release_region", pci_release_region as usize, false);
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

/// `pci_set_mwi` - `vendor/linux/drivers/pci/pci.c:4203`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_set_mwi(_dev: *mut c_void) -> i32 {
    0
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

/// `pci_release_selected_regions` - `vendor/linux/drivers/pci/pci.c:3846`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pci_release_selected_regions(dev: *mut c_void, bars: i32) {
    for bar in 0..PCI_STD_NUM_BARS as i32 {
        if bars & (1 << bar) != 0 {
            unsafe { pci_release_region(dev, bar) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::pci::device::{
        IORESOURCE_MEM, LinuxPciBarResource, LinuxPciDeviceAbiState, PCI_CONFIG_SPACE_SIZE,
        PCI_STD_NUM_BARS, register_linux_pci_device_state, unregister_linux_pci_device_state,
    };

    #[test]
    fn pci_helper_exports_module_symbols() {
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
            crate::kernel::module::find_symbol("pci_find_ext_capability"),
            Some(pci_find_ext_capability as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_enable_device"),
            Some(pci_enable_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_device_is_present"),
            Some(pci_device_is_present as usize)
        );
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
