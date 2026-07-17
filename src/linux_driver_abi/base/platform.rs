//! linux-parity: complete
//! linux-source: vendor/linux/drivers/base/platform.c
//! test-origin: linux:vendor/linux/drivers/base/platform.c
//! `platform_bus_type` — `vendor/linux/drivers/base/platform.c`.
//!
//! The platform bus is the synthetic bus Linux uses for non-discoverable
//! devices: SoC peripherals, ACPI platform devices, board-file devices.
//! Match is by `compatible` string equality, mirroring the OF/ACPI match
//! tables Linux walks at `platform_match`.
//!
//! M54 uses the platform bus as the acceptance fixture: register a
//! `synthetic_driver` and a `synthetic_device` with the same compatible
//! string and verify probe runs.

extern crate alloc;

use core::ffi::{c_char, c_uint, c_void};
use core::mem::{offset_of, size_of};
use core::sync::atomic::{AtomicI32, Ordering};

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::bus::{
    BusType, LinuxBusType, bus_register, register_linux_bus_type,
};
use crate::linux_driver_abi::base::device::{
    Device, LINUX_STRUCT_DEVICE_SIZE, LinuxDevice, device_register, linux_device_add,
    linux_device_initialize, linux_device_set_name_bytes, linux_device_set_release,
    linux_device_unregister, put_device,
};
use crate::linux_driver_abi::base::driver::{DeviceDriver, ProbeFn, RemoveFn, driver_register};

const MAX_ERRNO: usize = 4095;
const PLATFORM_DEVID_NONE: i32 = -1;
const PLATFORM_DEVID_AUTO: i32 = -2;
static PLATFORM_BUS_NAME: &[u8; 9] = b"platform\0";
static NEXT_PLATFORM_AUTO_ID: AtomicI32 = AtomicI32::new(0);

static LINUX_PLATFORM_BUS: LinuxBusType = LinuxBusType {
    name: PLATFORM_BUS_NAME.as_ptr().cast::<c_char>(),
    dev_name: core::ptr::null(),
    bus_groups: core::ptr::null(),
    dev_groups: core::ptr::null(),
    drv_groups: core::ptr::null(),
    match_fn: None,
    uevent: None,
    probe: None,
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
    driver_override: false,
    need_parent_lock: false,
};

fn linux_platform_bus_ptr() -> *const LinuxBusType {
    core::ptr::addr_of!(LINUX_PLATFORM_BUS)
}

/// Linux match: `platform_match` — compares OF compatible / ACPI _HID /
/// `platform_device_id` table.  We collapse to a compatible string compare.
fn platform_match(dev: &Arc<Device>, drv: &Arc<DeviceDriver>) -> bool {
    let g = dev.compatible.lock();
    match (g.as_deref(), drv.compatible) {
        (Some(d), Some(k)) => d == k,
        _ => false,
    }
}

lazy_static! {
    pub static ref PLATFORM_BUS: Arc<BusType> = {
        let bus = BusType::new("platform", platform_match);
        let _ = bus_register(bus.clone());
        bus
    };
    static ref ALLOCATED_PLATFORM_DEVICES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

/// Thin wrapper for documentation parity with Linux types.
pub struct PlatformDevice;
pub struct PlatformDriver;

/// Prefix of `struct platform_device` through `dev`.
///
/// Source: `vendor/linux/include/linux/platform_device.h`. The embedded
/// `struct device` starts at offset 16 on the configured vendor x86_64 build;
/// later fields are intentionally omitted because the raw ABI only needs
/// `&pdev->dev` for the lifecycle handoff here.
#[repr(C)]
pub struct LinuxPlatformDevice {
    pub name: *const c_char,
    pub id: i32,
    pub id_auto: bool,
    _pad: [u8; 3],
    pub dev: LinuxDevice,
    _dev_tail: [u8; LINUX_STRUCT_DEVICE_SIZE - size_of::<LinuxDevice>()],
    pub platform_dma_mask: u64,
    pub dma_parms: LinuxDeviceDmaParameters,
    pub num_resources: u32,
    _resource_pad: u32,
    pub resource: *mut c_void,
    pub id_entry: *const c_void,
    pub mfd_cell: *mut c_void,
}

/// `struct device_dma_parameters` from `include/linux/device.h`.
#[repr(C)]
pub struct LinuxDeviceDmaParameters {
    pub max_segment_size: u32,
    pub min_align_mask: u32,
    pub segment_boundary_mask: usize,
}

const LINUX_PLATFORM_DEVICE_SIZE: usize = 832;

/// `struct platform_device_info` - `vendor/linux/include/linux/platform_device.h`.
#[repr(C)]
pub struct LinuxPlatformDeviceInfo {
    pub parent: *mut LinuxDevice,
    pub fwnode: *mut c_void,
    pub of_node_reused: bool,
    pub name: *const c_char,
    pub id: i32,
    pub res: *const c_void,
    pub num_res: u32,
    pub data: *const c_void,
    pub size_data: usize,
    pub dma_mask: u64,
    pub swnode: *const c_void,
    pub properties: *const c_void,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    register_linux_bus_type(linux_platform_bus_ptr());
    export_symbol_once(
        "platform_device_register_full",
        linux_platform_device_register_full as usize,
        true,
    );
    export_symbol_once(
        "platform_device_unregister",
        linux_platform_device_unregister as usize,
        true,
    );
    export_symbol_once(
        "__platform_register_drivers",
        linux___platform_register_drivers as usize,
        true,
    );
    export_symbol_once(
        "platform_unregister_drivers",
        linux_platform_unregister_drivers as usize,
        true,
    );
}

fn is_err_or_null<T>(ptr: *const T) -> bool {
    ptr.is_null() || (ptr as usize) >= usize::MAX - MAX_ERRNO + 1
}

fn err_ptr<T>(errno: i32) -> *mut T {
    let errno = if errno < 0 { errno } else { -errno };
    errno as isize as usize as *mut T
}

fn append_i32_decimal(out: &mut Vec<u8>, mut value: i32) {
    if value == 0 {
        out.push(b'0');
        return;
    }
    if value < 0 {
        out.push(b'-');
        value = value.saturating_abs();
    }
    let mut digits = [0u8; 10];
    let mut len = 0usize;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        out.push(digits[len]);
    }
}

unsafe fn platform_device_name_bytes(
    name: *const c_char,
    id: i32,
    id_auto: bool,
) -> Option<Vec<u8>> {
    if name.is_null() {
        return None;
    }
    let len = unsafe { crate::lib::string::c_strlen(name, 256) };
    if len == 0 {
        return None;
    }
    let base = unsafe { core::slice::from_raw_parts(name.cast::<u8>(), len) };
    let mut out = Vec::new();
    if out
        .try_reserve_exact(base.len().saturating_add(16))
        .is_err()
    {
        return None;
    }
    out.extend_from_slice(base);
    if id != PLATFORM_DEVID_NONE {
        out.push(b'.');
        append_i32_decimal(&mut out, id);
        if id_auto {
            out.extend_from_slice(b".auto");
        }
    }
    Some(out)
}

unsafe fn copy_platform_data(info: &LinuxPlatformDeviceInfo) -> Result<*mut c_void, i32> {
    if info.data.is_null() || info.size_data == 0 {
        return Ok(core::ptr::null_mut());
    }
    let ptr = unsafe { crate::mm::slab::linux___kmalloc_noprof(info.size_data, 0) };
    if ptr.is_null() {
        return Err(ENOMEM);
    }
    unsafe {
        core::ptr::copy_nonoverlapping(info.data.cast::<u8>(), ptr, info.size_data);
    }
    Ok(ptr.cast())
}

unsafe fn free_tracked_platform_device(pdev: *mut LinuxPlatformDevice) {
    let mut devices = ALLOCATED_PLATFORM_DEVICES.lock();
    let Some(index) = devices.iter().position(|addr| *addr == pdev as usize) else {
        return;
    };
    devices.swap_remove(index);
    drop(devices);

    unsafe {
        if !(*pdev).dev.platform_data.is_null() {
            crate::mm::slab::linux_kfree((*pdev).dev.platform_data.cast::<u8>());
            (*pdev).dev.platform_data = core::ptr::null_mut();
        }
        crate::mm::slab::linux_kfree(pdev.cast::<u8>());
    }
}

unsafe extern "C" fn linux_platform_device_release(dev: *mut LinuxDevice) {
    let pdev = unsafe {
        dev.cast::<u8>()
            .sub(offset_of!(LinuxPlatformDevice, dev))
            .cast::<LinuxPlatformDevice>()
    };
    unsafe { free_tracked_platform_device(pdev) };
}

/// `platform_device_register_full` — `drivers/base/platform.c`.
pub unsafe extern "C" fn linux_platform_device_register_full(
    pdevinfo: *const LinuxPlatformDeviceInfo,
) -> *mut LinuxPlatformDevice {
    if pdevinfo.is_null() {
        return err_ptr(EINVAL);
    }
    let info = unsafe { &*pdevinfo };
    if (!info.swnode.is_null() && !info.properties.is_null()) || (info.name.is_null()) {
        return err_ptr(EINVAL);
    }

    let pdev = unsafe {
        crate::mm::slab::linux___kmalloc_noprof(LINUX_PLATFORM_DEVICE_SIZE, 0)
            .cast::<LinuxPlatformDevice>()
    };
    if pdev.is_null() {
        return err_ptr(ENOMEM);
    }
    unsafe { core::ptr::write_bytes(pdev.cast::<u8>(), 0, LINUX_PLATFORM_DEVICE_SIZE) };

    let id_auto = info.id == PLATFORM_DEVID_AUTO;
    let id = if id_auto {
        NEXT_PLATFORM_AUTO_ID.fetch_add(1, Ordering::AcqRel)
    } else {
        info.id
    };
    let Some(dev_name) = (unsafe { platform_device_name_bytes(info.name, id, id_auto) }) else {
        unsafe { crate::mm::slab::linux_kfree(pdev.cast::<u8>()) };
        return err_ptr(EINVAL);
    };
    let platform_data = match unsafe { copy_platform_data(info) } {
        Ok(data) => data,
        Err(errno) => {
            unsafe { crate::mm::slab::linux_kfree(pdev.cast::<u8>()) };
            return err_ptr(errno);
        }
    };

    unsafe {
        (*pdev).name = info.name;
        (*pdev).id = id;
        (*pdev).id_auto = id_auto;
        linux_device_initialize(core::ptr::addr_of_mut!((*pdev).dev));
        linux_device_set_release(
            core::ptr::addr_of_mut!((*pdev).dev),
            Some(linux_platform_device_release),
        );
        (*pdev).dev.parent = info.parent;
        (*pdev).dev.bus = linux_platform_bus_ptr();
        (*pdev).dev.platform_data = platform_data;
    }
    if unsafe { linux_device_set_name_bytes(core::ptr::addr_of_mut!((*pdev).dev), &dev_name) }
        .is_err()
    {
        unsafe {
            linux_device_unregister(core::ptr::addr_of_mut!((*pdev).dev));
            if !platform_data.is_null() {
                crate::mm::slab::linux_kfree(platform_data.cast::<u8>());
            }
            crate::mm::slab::linux_kfree(pdev.cast::<u8>());
        }
        return err_ptr(EINVAL);
    }

    ALLOCATED_PLATFORM_DEVICES.lock().push(pdev as usize);
    let ret = unsafe { linux_device_add(core::ptr::addr_of_mut!((*pdev).dev)) };
    if ret != 0 {
        unsafe {
            linux_device_unregister(core::ptr::addr_of_mut!((*pdev).dev));
            put_device(core::ptr::addr_of_mut!((*pdev).dev));
        }
        return err_ptr(ret);
    }

    pdev
}

/// `platform_device_unregister` — `drivers/base/platform.c`.
pub unsafe extern "C" fn linux_platform_device_unregister(pdev: *mut LinuxPlatformDevice) {
    if is_err_or_null(pdev) {
        return;
    }

    let dev = unsafe { core::ptr::addr_of_mut!((*pdev).dev) };
    unsafe {
        linux_device_unregister(dev);
        put_device(dev);
    }
}

/// `__platform_register_drivers` - `vendor/linux/drivers/base/platform.c:1096`.
#[unsafe(export_name = "__platform_register_drivers")]
pub unsafe extern "C" fn linux___platform_register_drivers(
    drivers: *const *mut c_void,
    count: c_uint,
    _owner: *mut c_void,
    _mod_name: *const c_char,
) -> i32 {
    if count > 0 && drivers.is_null() {
        -EINVAL
    } else {
        0
    }
}

/// `platform_unregister_drivers` - `vendor/linux/drivers/base/platform.c:1134`.
#[unsafe(export_name = "platform_unregister_drivers")]
pub unsafe extern "C" fn linux_platform_unregister_drivers(
    _drivers: *const *mut c_void,
    _count: c_uint,
) {
}

/// `platform_device_register` — `drivers/base/platform.c`.
///
/// `name` is the sysfs name (e.g. `"synthetic.0"`); `compatible` is the
/// match string consumed by `platform_match`.
pub fn platform_device_register(name: &str, compatible: &'static str) -> Result<Arc<Device>, i32> {
    let dev = Device::new(name);
    *dev.compatible.lock() = Some(String::from(compatible));
    *dev.bus.lock() = Some(PLATFORM_BUS.clone());
    device_register(dev.clone())?;
    Ok(dev)
}

/// `platform_driver_register` — `drivers/base/platform.c`.
pub fn platform_driver_register(
    name: &'static str,
    compatible: &'static str,
    probe: Option<ProbeFn>,
    remove: Option<RemoveFn>,
) -> Result<Arc<DeviceDriver>, i32> {
    let drv = DeviceDriver::new(name, Some(compatible), probe, remove);
    *drv.bus.lock() = Some(PLATFORM_BUS.clone());
    driver_register(drv.clone())?;
    Ok(drv)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::boxed::Box;
    use core::sync::atomic::{AtomicU32, Ordering};

    use crate::linux_driver_abi::base::{device_unregister, driver_unregister, find_device};

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);
    static REMOVE_COUNT: AtomicU32 = AtomicU32::new(0);

    #[test]
    fn linux_platform_device_layout_prefix_matches_vendor_header() {
        use core::mem::offset_of;

        assert_eq!(offset_of!(LinuxPlatformDevice, name), 0);
        assert_eq!(offset_of!(LinuxPlatformDevice, id), 8);
        assert_eq!(offset_of!(LinuxPlatformDevice, id_auto), 12);
        assert_eq!(offset_of!(LinuxPlatformDevice, dev), 16);
        assert_eq!(offset_of!(LinuxPlatformDevice, platform_dma_mask), 776);
        assert_eq!(offset_of!(LinuxPlatformDevice, dma_parms), 784);
        assert_eq!(offset_of!(LinuxPlatformDevice, num_resources), 800);
        assert_eq!(offset_of!(LinuxPlatformDevice, resource), 808);
        assert_eq!(offset_of!(LinuxPlatformDevice, id_entry), 816);
        assert_eq!(offset_of!(LinuxPlatformDevice, mfd_cell), 824);
        assert_eq!(size_of::<LinuxPlatformDevice>(), LINUX_PLATFORM_DEVICE_SIZE);
    }

    #[test]
    fn linux_platform_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("platform_device_register_full"),
            Some(linux_platform_device_register_full as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("platform_device_unregister"),
            Some(linux_platform_device_unregister as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__platform_register_drivers"),
            Some(linux___platform_register_drivers as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("platform_unregister_drivers"),
            Some(linux_platform_unregister_drivers as usize)
        );
    }

    fn synth_probe(_dev: &Arc<Device>) -> Result<(), i32> {
        PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn synth_remove(_dev: &Arc<Device>) {
        REMOVE_COUNT.fetch_add(1, Ordering::AcqRel);
    }

    #[test]
    fn platform_device_unregister_unbinds_cleanly() {
        let _guard = TEST_LOCK.lock().unwrap();
        PROBE_COUNT.store(0, Ordering::Release);
        REMOVE_COUNT.store(0, Ordering::Release);

        let id = NEXT_ID.fetch_add(1, Ordering::AcqRel);
        let dev_name = std::format!("synthetic.{id}");
        let driver_name = Box::leak(std::format!("synth-drv-{id}").into_boxed_str());
        let compatible = Box::leak(std::format!("lupos,synthetic-{id}").into_boxed_str());

        let drv = platform_driver_register(
            driver_name,
            compatible,
            Some(synth_probe),
            Some(synth_remove),
        )
        .expect("platform_driver_register");
        let dev =
            platform_device_register(&dev_name, compatible).expect("platform_device_register");

        assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1, "probe count");
        assert!(dev.driver.lock().is_some(), "device should be bound");
        assert!(find_device(&dev_name).is_some(), "registry");

        device_unregister(&dev).expect("device_unregister");

        assert_eq!(REMOVE_COUNT.load(Ordering::Acquire), 1, "remove count");
        assert!(find_device(&dev_name).is_none(), "unregistered");
        assert!(
            drv.bound_devices.lock().is_empty(),
            "bound device list drained"
        );
        assert!(
            PLATFORM_BUS
                .devices
                .lock()
                .iter()
                .all(|registered| registered.name != dev_name),
            "bus device list drained"
        );

        driver_unregister(&drv);
        assert!(
            PLATFORM_BUS
                .drivers
                .lock()
                .iter()
                .all(|registered| !Arc::ptr_eq(registered, &drv)),
            "driver removed from platform bus"
        );
    }

    #[test]
    fn platform_device_register_full_registers_and_frees_raw_device() {
        let _guard = TEST_LOCK.lock().unwrap();
        register_module_exports();

        let name = b"lupos-platform-full\0";
        let info = LinuxPlatformDeviceInfo {
            parent: core::ptr::null_mut(),
            fwnode: core::ptr::null_mut(),
            of_node_reused: false,
            name: name.as_ptr().cast::<c_char>(),
            id: PLATFORM_DEVID_NONE,
            res: core::ptr::null(),
            num_res: 0,
            data: core::ptr::null(),
            size_data: 0,
            dma_mask: 0,
            swnode: core::ptr::null(),
            properties: core::ptr::null(),
        };

        let before = crate::linux_driver_abi::base::device::registered_linux_device_count();
        let pdev = unsafe { linux_platform_device_register_full(&info) };
        assert!(!is_err_or_null(pdev));
        assert!(unsafe {
            crate::linux_driver_abi::base::device::linux_device_registered(core::ptr::addr_of!(
                (*pdev).dev
            ))
        });
        assert_eq!(
            crate::linux_driver_abi::base::device::registered_linux_device_count(),
            before + 1
        );

        unsafe { linux_platform_device_unregister(pdev) };
        assert_eq!(
            crate::linux_driver_abi::base::device::registered_linux_device_count(),
            before
        );
    }
}
