//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci
//! test-origin: linux:vendor/linux/drivers/pci
//! `struct pci_driver` — `include/linux/pci.h:1021`.
//!
//! PCI driver registration mirrors `drivers/pci/pci-driver.c`.
//! Probe dispatch: `pci_device_probe` (line 466) / `__pci_register_driver` (line 1464).

extern crate alloc;

use core::ffi::{c_char, c_void};

use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::{
    LinuxBusType, LinuxDeviceDriver, linux_driver_register, linux_driver_unregister,
    register_linux_bus_type,
};
use crate::linux_driver_abi::pci::device::{
    LinuxPciDev, linux_pci_dev_from_device, linux_pci_device_state, linux_pci_slot_for_raw,
    registered_linux_pci_raw_devices,
};

const PCI_ANY_ID: u32 = u32::MAX;
static PCI_BUS_NAME: &[u8; 4] = b"pci\0";

/// `struct pci_device_id` - `vendor/linux/include/linux/mod_devicetable.h:44`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxPciDeviceId {
    pub vendor: u32,
    pub device: u32,
    pub subvendor: u32,
    pub subdevice: u32,
    pub class: u32,
    pub class_mask: u32,
    pub driver_data: usize,
    pub override_only: u32,
}

/// Prefix of `struct pci_driver` through the embedded `struct device_driver`.
///
/// Source: `vendor/linux/include/linux/pci.h:1021`. Linux's
/// `__pci_register_driver()` initializes the embedded driver and delegates to
/// driver core; dynamic IDs are intentionally left to the vendor module.
#[repr(C)]
pub struct LinuxPciDriver {
    pub name: *const c_char,
    pub id_table: *const LinuxPciDeviceId,
    pub probe: Option<unsafe extern "C" fn(dev: *mut c_void, id: *const LinuxPciDeviceId) -> i32>,
    pub remove: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub suspend: Option<unsafe extern "C" fn(dev: *mut c_void, state: usize) -> i32>,
    pub resume: Option<unsafe extern "C" fn(dev: *mut c_void) -> i32>,
    pub shutdown: Option<unsafe extern "C" fn(dev: *mut c_void)>,
    pub sriov_configure: Option<unsafe extern "C" fn(dev: *mut c_void, num_vfs: i32) -> i32>,
    pub sriov_set_msix_vec_count:
        Option<unsafe extern "C" fn(dev: *mut c_void, msix_vec_count: i32) -> i32>,
    pub sriov_get_vf_total_msix: Option<unsafe extern "C" fn(dev: *mut c_void) -> u32>,
    pub err_handler: *const c_void,
    pub groups: *const *const c_void,
    pub dev_groups: *const *const c_void,
    pub driver: LinuxDeviceDriver,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    register_linux_bus_type(linux_pci_bus_ptr());
    export_symbol_once(
        "__pci_register_driver",
        linux___pci_register_driver as usize,
        false,
    );
    export_symbol_once(
        "pci_unregister_driver",
        linux_pci_unregister_driver as usize,
        false,
    );
    export_symbol_once("pci_add_dynid", linux_pci_add_dynid as usize, true);
    export_symbol_once("pci_match_id", linux_pci_match_id as usize, false);
    export_symbol_once("pci_dev_present", linux_pci_dev_present as usize, false);
}

static LINUX_PCI_BUS: LinuxBusType = LinuxBusType {
    name: PCI_BUS_NAME.as_ptr().cast::<c_char>(),
    dev_name: core::ptr::null(),
    bus_groups: core::ptr::null(),
    dev_groups: core::ptr::null(),
    drv_groups: core::ptr::null(),
    match_fn: Some(linux_pci_bus_match),
    uevent: None,
    probe: Some(linux_pci_device_probe),
    sync_state: None,
    remove: None,
    shutdown: None,
    irq_get_affinity: None,
    online: None,
    offline: None,
    suspend: None,
    resume: None,
    num_vf: None,
    dma_configure: None,
    dma_cleanup: None,
    pm: core::ptr::null(),
    driver_override: true,
    need_parent_lock: false,
};

pub fn linux_pci_bus_type_ptr() -> *const LinuxBusType {
    core::ptr::addr_of!(LINUX_PCI_BUS)
}

fn linux_pci_bus_ptr() -> *const LinuxBusType {
    linux_pci_bus_type_ptr()
}

unsafe fn linux_pci_driver_from_driver(driver: *const LinuxDeviceDriver) -> *mut LinuxPciDriver {
    if driver.is_null() {
        return core::ptr::null_mut();
    }
    let offset = core::mem::offset_of!(LinuxPciDriver, driver);
    unsafe {
        driver
            .cast::<u8>()
            .sub(offset)
            .cast_mut()
            .cast::<LinuxPciDriver>()
    }
}

/// `pci_add_dynid` - `vendor/linux/drivers/pci/pci-driver.c`.
#[unsafe(export_name = "pci_add_dynid")]
unsafe extern "C" fn linux_pci_add_dynid(
    drv: *mut LinuxPciDriver,
    _vendor: u32,
    _device: u32,
    _subvendor: u32,
    _subdevice: u32,
    _class: u32,
    _class_mask: u32,
    _driver_data: usize,
) -> i32 {
    if drv.is_null() { -EINVAL } else { 0 }
}

fn linux_pci_id_is_terminator(id: &LinuxPciDeviceId) -> bool {
    id.vendor == 0 && id.subvendor == 0 && id.class_mask == 0
}

/// `pci_match_id` - `vendor/linux/drivers/pci/pci-driver.c`.
#[unsafe(export_name = "pci_match_id")]
pub unsafe extern "C" fn linux_pci_match_id(
    ids: *const LinuxPciDeviceId,
    dev: *const c_void,
) -> *const LinuxPciDeviceId {
    let Some(state) = linux_pci_device_state(dev) else {
        return core::ptr::null();
    };
    if ids.is_null() {
        return core::ptr::null();
    }

    let vendor = u16::from_le_bytes([state.config_space[0x00], state.config_space[0x01]]) as u32;
    let device = u16::from_le_bytes([state.config_space[0x02], state.config_space[0x03]]) as u32;
    let class = ((state.config_space[0x0b] as u32) << 16)
        | ((state.config_space[0x0a] as u32) << 8)
        | state.config_space[0x09] as u32;
    let subvendor = u16::from_le_bytes([state.config_space[0x2c], state.config_space[0x2d]]) as u32;
    let subdevice = u16::from_le_bytes([state.config_space[0x2e], state.config_space[0x2f]]) as u32;

    let mut idx = 0usize;
    loop {
        let id = unsafe { &*ids.add(idx) };
        if linux_pci_id_is_terminator(id) {
            return core::ptr::null();
        }
        if (id.vendor == PCI_ANY_ID || id.vendor == vendor)
            && (id.device == PCI_ANY_ID || id.device == device)
            && (id.subvendor == PCI_ANY_ID || id.subvendor == subvendor)
            && (id.subdevice == PCI_ANY_ID || id.subdevice == subdevice)
            && ((id.class ^ class) & id.class_mask) == 0
        {
            return unsafe { ids.add(idx) };
        }
        idx += 1;
    }
}

/// `pci_dev_present` - `vendor/linux/drivers/pci/search.c:456`.
///
/// This is a hint-style presence check over the current PCI device registry.
/// It deliberately does not retain a device reference, matching Linux's
/// documented stale-result semantics for this helper.
#[unsafe(export_name = "pci_dev_present")]
pub unsafe extern "C" fn linux_pci_dev_present(ids: *const LinuxPciDeviceId) -> i32 {
    if ids.is_null() {
        return 0;
    }

    let mut idx = 0usize;
    loop {
        let id = unsafe { &*ids.add(idx) };
        if linux_pci_id_is_terminator(id) {
            return 0;
        }

        for dev in registered_linux_pci_raw_devices() {
            if !unsafe { linux_pci_match_id(ids.add(idx), dev.cast_const().cast()) }.is_null() {
                return 1;
            }
        }

        idx += 1;
    }
}

unsafe extern "C" fn linux_pci_bus_match(dev: *mut c_void, drv: *const c_void) -> i32 {
    let pci_drv = unsafe { linux_pci_driver_from_driver(drv.cast::<LinuxDeviceDriver>()) };
    if pci_drv.is_null() {
        return 0;
    }
    let pci_dev = unsafe { linux_pci_dev_from_device(dev.cast_const().cast()) };
    if pci_dev.is_null() {
        return 0;
    }
    let id = unsafe { linux_pci_match_id((*pci_drv).id_table, pci_dev.cast_const().cast()) };
    (!id.is_null()) as i32
}

unsafe extern "C" fn linux_pci_device_probe(dev: *mut c_void) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let pci_dev = unsafe { linux_pci_dev_from_device(dev.cast_const().cast()) };
    if pci_dev.is_null() {
        return -EINVAL;
    }
    let driver = unsafe { (*dev.cast::<crate::linux_driver_abi::base::LinuxDevice>()).driver };
    let pci_drv = unsafe { linux_pci_driver_from_driver(driver.cast_const()) };
    if pci_drv.is_null() {
        return -EINVAL;
    }
    let id = unsafe { linux_pci_match_id((*pci_drv).id_table, pci_dev.cast_const().cast()) };
    if id.is_null() {
        return -EINVAL;
    }
    let Some(probe) = (unsafe { (*pci_drv).probe }) else {
        return -EINVAL;
    };

    unsafe {
        let intpin = linux_pci_device_state(pci_dev.cast_const().cast())
            .map(|state| state.config_space[0x3d])
            .unwrap_or(0);
        let (seg, bus, slot, func) = linux_pci_slot_for_raw(pci_dev.cast_const().cast())
            .unwrap_or((0xffff, 0xff, 0xff, 0xff));
        crate::log_warn!(
            "pci",
            "Linux PCI probe: dev={:p} bdf={:04x}:{:02x}:{:02x}.{} devfn=0x{:02x} vendor=0x{:04x} device=0x{:04x} class=0x{:06x} irq={} intpin={} id_vendor=0x{:08x} id_device=0x{:08x} id_class=0x{:06x} id_mask=0x{:06x} driver_data=0x{:x} probe=0x{:x}",
            pci_dev,
            seg,
            bus,
            slot,
            func,
            (*pci_dev).devfn,
            (*pci_dev).vendor,
            (*pci_dev).device,
            (*pci_dev).class,
            (*pci_dev).irq,
            intpin,
            (*id).vendor,
            (*id).device,
            (*id).class,
            (*id).class_mask,
            (*id).driver_data,
            probe as usize
        );
    }

    unsafe {
        (*pci_dev).driver = pci_drv.cast::<c_void>();
    }
    let ret = unsafe { probe(pci_dev.cast::<c_void>(), id) };
    if ret != 0 {
        crate::log_warn!(
            "pci",
            "Linux PCI driver probe failed for {:p}: errno {}",
            pci_dev,
            ret
        );
        unsafe {
            (*pci_dev).driver = core::ptr::null_mut();
        }
    }
    ret
}

/// `__pci_register_driver` - `vendor/linux/drivers/pci/pci-driver.c:1471`.
#[unsafe(export_name = "__pci_register_driver")]
pub unsafe extern "C" fn linux___pci_register_driver(
    drv: *mut LinuxPciDriver,
    owner: *mut c_void,
    mod_name: *const c_char,
) -> i32 {
    if drv.is_null() {
        return -EINVAL;
    }
    unsafe {
        (*drv).driver.name = (*drv).name;
        (*drv).driver.bus = linux_pci_bus_ptr();
        (*drv).driver.owner = owner;
        (*drv).driver.mod_name = mod_name;
        (*drv).driver.groups = (*drv).groups;
        (*drv).driver.dev_groups = (*drv).dev_groups;
        linux_driver_register(core::ptr::addr_of_mut!((*drv).driver))
    }
}

/// `pci_unregister_driver` - `vendor/linux/drivers/pci/pci-driver.c:1500`.
#[unsafe(export_name = "pci_unregister_driver")]
pub unsafe extern "C" fn linux_pci_unregister_driver(drv: *mut LinuxPciDriver) {
    if drv.is_null() {
        return;
    }
    unsafe {
        linux_driver_unregister(core::ptr::addr_of_mut!((*drv).driver));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::base::{
        linux_bus_type_registered, linux_device_driver_registered,
    };
    use crate::linux_driver_abi::pci::device::{
        LinuxPciDeviceAbiState, PciDev, register_linux_pci_device_state,
        unregister_linux_pci_device_state,
    };

    #[test]
    fn linux_pci_driver_layout_matches_vendor_prefix() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxPciDeviceId, vendor), 0);
        assert_eq!(offset_of!(LinuxPciDeviceId, device), 4);
        assert_eq!(offset_of!(LinuxPciDeviceId, subvendor), 8);
        assert_eq!(offset_of!(LinuxPciDeviceId, subdevice), 12);
        assert_eq!(offset_of!(LinuxPciDeviceId, class), 16);
        assert_eq!(offset_of!(LinuxPciDeviceId, class_mask), 20);
        assert_eq!(offset_of!(LinuxPciDeviceId, driver_data), 24);
        assert_eq!(offset_of!(LinuxPciDeviceId, override_only), 32);
        assert_eq!(size_of::<LinuxPciDeviceId>(), 40);

        assert_eq!(offset_of!(LinuxPciDriver, name), 0);
        assert_eq!(offset_of!(LinuxPciDriver, id_table), 8);
        assert_eq!(offset_of!(LinuxPciDriver, probe), 16);
        assert_eq!(offset_of!(LinuxPciDriver, remove), 24);
        assert_eq!(offset_of!(LinuxPciDriver, groups), 88);
        assert_eq!(offset_of!(LinuxPciDriver, dev_groups), 96);
        assert_eq!(offset_of!(LinuxPciDriver, driver), 104);
    }

    #[test]
    fn linux_pci_module_exports_register_for_modules() {
        register_module_exports();

        assert!(linux_bus_type_registered(linux_pci_bus_ptr()));
        assert_eq!(
            crate::kernel::module::find_symbol("__pci_register_driver"),
            Some(linux___pci_register_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_unregister_driver"),
            Some(linux_pci_unregister_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pci_dev_present"),
            Some(linux_pci_dev_present as usize)
        );
    }

    #[test]
    fn linux_pci_register_driver_initializes_embedded_driver_core() {
        unsafe {
            let mut drv = core::mem::zeroed::<LinuxPciDriver>();
            let name = b"pci-raw-test\0";
            let mod_name = b"pci_raw_test\0";
            let groups = 0x1234usize as *const *const c_void;
            let dev_groups = 0x5678usize as *const *const c_void;
            drv.name = name.as_ptr().cast::<c_char>();
            drv.groups = groups;
            drv.dev_groups = dev_groups;

            assert_eq!(
                linux___pci_register_driver(
                    &mut drv,
                    0xfeedusize as *mut c_void,
                    mod_name.as_ptr().cast::<c_char>(),
                ),
                0
            );
            assert_eq!(drv.driver.name, drv.name);
            assert_eq!(drv.driver.bus, linux_pci_bus_ptr());
            assert_eq!(drv.driver.owner, 0xfeedusize as *mut c_void);
            assert_eq!(drv.driver.mod_name, mod_name.as_ptr().cast::<c_char>());
            assert_eq!(drv.driver.groups, groups);
            assert_eq!(drv.driver.dev_groups, dev_groups);
            assert!(linux_device_driver_registered(&drv.driver));

            linux_pci_unregister_driver(&mut drv);
            assert!(!linux_device_driver_registered(&drv.driver));
        }
    }

    #[test]
    fn linux_pci_match_id_uses_registered_raw_config_state() {
        unsafe {
            let dev = PciDev::new_with_subsystem(
                0, 0, 1, 0, 0x1af4, 0x1041, 0x01, 0x00, 0x00, 1, 0x1af4, 0x1100,
            );
            let raw = 0x1234usize as *const c_void;
            register_linux_pci_device_state(raw, LinuxPciDeviceAbiState::from_pci_dev(&dev));
            let ids = [
                LinuxPciDeviceId {
                    vendor: 0x1af4,
                    device: 0x1041,
                    subvendor: PCI_ANY_ID,
                    subdevice: PCI_ANY_ID,
                    class: 0,
                    class_mask: 0,
                    driver_data: 7,
                    override_only: 0,
                },
                LinuxPciDeviceId {
                    vendor: 0,
                    device: 0,
                    subvendor: 0,
                    subdevice: 0,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0,
                    override_only: 0,
                },
            ];

            let matched = linux_pci_match_id(ids.as_ptr(), raw);
            assert_eq!(matched, ids.as_ptr());
            unregister_linux_pci_device_state(raw);
        }
    }

    #[test]
    fn linux_pci_dev_present_matches_registered_raw_device() {
        unsafe {
            let pdev = PciDev::new_with_subsystem(
                0, 0, 30, 0, 0xfefe, 0xcafe, 0x03, 0x00, 0x00, 1, 0x1111, 0x2222,
            );
            let raw = crate::linux_driver_abi::pci::device::register_linux_pci_device(
                &pdev,
                linux_pci_bus_type_ptr(),
            );
            assert!(!raw.is_null());

            let ids = [
                LinuxPciDeviceId {
                    vendor: 0x1234,
                    device: 0x5678,
                    subvendor: PCI_ANY_ID,
                    subdevice: PCI_ANY_ID,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0,
                    override_only: 0,
                },
                LinuxPciDeviceId {
                    vendor: 0xfefe,
                    device: 0xcafe,
                    subvendor: 0x1111,
                    subdevice: 0x2222,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0,
                    override_only: 0,
                },
                LinuxPciDeviceId {
                    vendor: 0,
                    device: 0,
                    subvendor: 0,
                    subdevice: 0,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0,
                    override_only: 0,
                },
            ];

            assert_eq!(linux_pci_dev_present(ids.as_ptr()), 1);
        }
    }

    #[test]
    fn linux_pci_driver_core_probes_with_raw_pci_dev_pointer() {
        static PROBED: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

        unsafe extern "C" fn probe(dev: *mut c_void, id: *const LinuxPciDeviceId) -> i32 {
            unsafe {
                assert_eq!((*id).driver_data, 0x55aa);
            }
            PROBED.store(dev as usize, core::sync::atomic::Ordering::Release);
            0
        }

        unsafe {
            register_module_exports();
            let pdev = PciDev::new_with_subsystem(
                0, 0, 29, 0, 0x1af4, 0x1043, 0x01, 0x00, 0x00, 1, 0x1af4, 0x1100,
            );
            let raw = crate::linux_driver_abi::pci::device::register_linux_pci_device(
                &pdev,
                linux_pci_bus_type_ptr(),
            );
            assert!(!raw.is_null());

            let ids = [
                LinuxPciDeviceId {
                    vendor: 0x1af4,
                    device: 0x1043,
                    subvendor: PCI_ANY_ID,
                    subdevice: PCI_ANY_ID,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0x55aa,
                    override_only: 0,
                },
                LinuxPciDeviceId {
                    vendor: 0,
                    device: 0,
                    subvendor: 0,
                    subdevice: 0,
                    class: 0,
                    class_mask: 0,
                    driver_data: 0,
                    override_only: 0,
                },
            ];
            let mut driver = core::mem::zeroed::<LinuxPciDriver>();
            let name = b"pci-probe-raw-test\0";
            driver.name = name.as_ptr().cast::<c_char>();
            driver.id_table = ids.as_ptr();
            driver.probe = Some(probe);

            assert_eq!(
                linux___pci_register_driver(&mut driver, core::ptr::null_mut(), core::ptr::null()),
                0
            );
            assert_eq!(
                PROBED.load(core::sync::atomic::Ordering::Acquire),
                raw as usize
            );
            assert_eq!((*raw).driver, (&mut driver as *mut LinuxPciDriver).cast());
            assert!(
                crate::linux_driver_abi::pci::device::linux_pci_device_bound(0, 0, 29, 0),
                "raw Linux PCI device should be marked bound after successful probe"
            );
            assert!(
                crate::linux_driver_abi::pci::device::registered_linux_pci_bound_device_count() > 0,
                "bound raw Linux PCI devices should be counted"
            );

            linux_pci_unregister_driver(&mut driver);
        }
    }
}
