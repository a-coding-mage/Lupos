//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/auxiliary.c
//! test-origin: linux:vendor/linux/drivers/base/auxiliary.c
//! Linux auxiliary bus ABI.

extern crate alloc;

use core::ffi::{c_char, c_void};
use core::mem::size_of;

use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::bus::{LinuxBusType, register_linux_bus_type};
use crate::linux_driver_abi::base::device::{
    LinuxDevice, get_device, linux_device_add, linux_device_initialize,
    linux_device_set_name_bytes, linux_device_unregister, put_device,
};
use crate::linux_driver_abi::base::driver::{
    LinuxDeviceDriver, linux_driver_register, linux_driver_unregister,
};
use crate::mm::page_flags::GFP_KERNEL;
use crate::mm::slab::{linux___kmalloc_noprof, linux_kfree};

const LINUX_STRUCT_DEVICE_SIZE: usize = 760;
const LINUX_AUXILIARY_DEVICE_NAME_OFFSET: usize = LINUX_STRUCT_DEVICE_SIZE;
const LINUX_AUXILIARY_DEVICE_ID_OFFSET: usize =
    LINUX_AUXILIARY_DEVICE_NAME_OFFSET + size_of::<usize>();
const LINUX_AUXILIARY_DRIVER_DRIVER_OFFSET: usize = 48;
const AUXILIARY_NAME_SIZE: usize = 40;

type LinuxAuxiliaryProbeFn =
    unsafe extern "C" fn(*mut LinuxAuxiliaryDevice, *const LinuxAuxiliaryDeviceId) -> i32;
type LinuxAuxiliaryRemoveFn = unsafe extern "C" fn(*mut LinuxAuxiliaryDevice);
type LinuxAuxiliaryShutdownFn = unsafe extern "C" fn(*mut LinuxAuxiliaryDevice);
type LinuxAuxiliaryPmFn = unsafe extern "C" fn(*mut LinuxAuxiliaryDevice, usize) -> i32;

#[repr(C)]
pub struct LinuxAuxiliaryDevice {
    pub dev: LinuxDevice,
}

#[repr(C)]
pub struct LinuxAuxiliaryDeviceId {
    pub name: [c_char; AUXILIARY_NAME_SIZE],
    pub driver_data: usize,
}

#[repr(C)]
pub struct LinuxAuxiliaryDriver {
    pub probe: Option<LinuxAuxiliaryProbeFn>,
    pub remove: Option<LinuxAuxiliaryRemoveFn>,
    pub shutdown: Option<LinuxAuxiliaryShutdownFn>,
    pub suspend: Option<LinuxAuxiliaryPmFn>,
    pub resume: Option<LinuxAuxiliaryPmFn>,
    pub name: *const c_char,
    pub driver: LinuxDeviceDriver,
    pub id_table: *const LinuxAuxiliaryDeviceId,
}

const AUXILIARY_BUS_NAME: &[u8] = b"auxiliary\0";

static LINUX_AUXILIARY_BUS_TYPE: LinuxBusType = LinuxBusType {
    name: AUXILIARY_BUS_NAME.as_ptr().cast::<c_char>(),
    dev_name: core::ptr::null(),
    bus_groups: core::ptr::null(),
    dev_groups: core::ptr::null(),
    drv_groups: core::ptr::null(),
    match_fn: Some(linux_auxiliary_match),
    uevent: None,
    probe: Some(linux_auxiliary_bus_probe),
    sync_state: None,
    remove: Some(linux_auxiliary_bus_remove),
    shutdown: Some(linux_auxiliary_bus_shutdown),
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

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    register_linux_bus_type(core::ptr::addr_of!(LINUX_AUXILIARY_BUS_TYPE));
    export_symbol_once(
        "auxiliary_device_init",
        linux_auxiliary_device_init as usize,
        true,
    );
    export_symbol_once(
        "__auxiliary_device_add",
        linux___auxiliary_device_add as usize,
        true,
    );
    export_symbol_once(
        "__auxiliary_driver_register",
        linux___auxiliary_driver_register as usize,
        true,
    );
    export_symbol_once(
        "auxiliary_driver_unregister",
        linux_auxiliary_driver_unregister as usize,
        true,
    );
    export_symbol_once(
        "auxiliary_device_delete",
        linux_auxiliary_device_delete as usize,
        true,
    );
    export_symbol_once(
        "auxiliary_device_uninit",
        linux_auxiliary_device_uninit as usize,
        true,
    );
    export_symbol_once(
        "auxiliary_device_sysfs_irq_add",
        linux_auxiliary_device_sysfs_irq_add as usize,
        true,
    );
    export_symbol_once(
        "auxiliary_device_sysfs_irq_remove",
        linux_auxiliary_device_sysfs_irq_remove as usize,
        true,
    );
    export_symbol_once("dev_is_auxiliary", linux_dev_is_auxiliary as usize, true);
}

unsafe fn read_usize(base: *const c_void, offset: usize) -> usize {
    unsafe { base.cast::<u8>().add(offset).cast::<usize>().read() }
}

unsafe fn read_u32(base: *const c_void, offset: usize) -> u32 {
    unsafe { base.cast::<u8>().add(offset).cast::<u32>().read() }
}

unsafe fn auxdev_name(auxdev: *const LinuxAuxiliaryDevice) -> *const c_char {
    unsafe { read_usize(auxdev.cast(), LINUX_AUXILIARY_DEVICE_NAME_OFFSET) as *const c_char }
}

unsafe fn auxdev_id(auxdev: *const LinuxAuxiliaryDevice) -> u32 {
    unsafe { read_u32(auxdev.cast(), LINUX_AUXILIARY_DEVICE_ID_OFFSET) }
}

unsafe fn c_strlen(ptr: *const c_char, max: usize) -> usize {
    unsafe { crate::lib::string::c_strlen(ptr, max) }
}

unsafe fn device_name(dev: *const LinuxDevice) -> *const c_char {
    if dev.is_null() {
        return core::ptr::null();
    }
    let init_name = unsafe { (*dev).init_name };
    if !init_name.is_null() {
        init_name
    } else {
        unsafe { (*dev).kobj.name }
    }
}

unsafe fn cstr_prefix_eq(cstr: *const c_char, prefix: *const c_char, len: usize) -> bool {
    if cstr.is_null() || prefix.is_null() {
        return false;
    }
    let mut idx = 0usize;
    while idx < len {
        if unsafe { *cstr.add(idx) } != unsafe { *prefix.add(idx) } {
            return false;
        }
        idx += 1;
    }
    true
}

unsafe fn auxiliary_match_id(
    id: *const LinuxAuxiliaryDeviceId,
    auxdev: *const LinuxAuxiliaryDevice,
) -> *const LinuxAuxiliaryDeviceId {
    if id.is_null() || auxdev.is_null() {
        return core::ptr::null();
    }

    let name = unsafe { device_name(core::ptr::addr_of!((*auxdev).dev)) };
    let name_len = unsafe { c_strlen(name, 256) };
    let Some(last_dot) = (0..name_len)
        .rev()
        .find(|idx| unsafe { *name.add(*idx) } == b'.' as c_char)
    else {
        return core::ptr::null();
    };

    let mut cur = id;
    loop {
        let id_name = unsafe { core::ptr::addr_of!((*cur).name).cast::<c_char>() };
        if unsafe { *id_name } == 0 {
            return core::ptr::null();
        }
        let id_len = unsafe { c_strlen(id_name, AUXILIARY_NAME_SIZE) };
        if id_len == last_dot && unsafe { cstr_prefix_eq(name, id_name, last_dot) } {
            return cur;
        }
        cur = unsafe { cur.add(1) };
    }
}

unsafe fn auxiliary_driver_from_device_driver(
    driver: *const LinuxDeviceDriver,
) -> *mut LinuxAuxiliaryDriver {
    if driver.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe {
            driver
                .cast::<u8>()
                .sub(LINUX_AUXILIARY_DRIVER_DRIVER_OFFSET)
                .cast::<LinuxAuxiliaryDriver>()
                .cast_mut()
        }
    }
}

unsafe extern "C" fn linux_auxiliary_match(dev: *mut c_void, drv: *const c_void) -> i32 {
    let auxdev = dev.cast::<LinuxAuxiliaryDevice>();
    let auxdrv = unsafe { auxiliary_driver_from_device_driver(drv.cast::<LinuxDeviceDriver>()) };
    if auxdrv.is_null() {
        return 0;
    }
    let id = unsafe { auxiliary_match_id((*auxdrv).id_table, auxdev) };
    (!id.is_null()) as i32
}

unsafe extern "C" fn linux_auxiliary_bus_probe(dev: *mut c_void) -> i32 {
    let auxdev = dev.cast::<LinuxAuxiliaryDevice>();
    let driver = unsafe { (*dev.cast::<LinuxDevice>()).driver };
    let auxdrv = unsafe { auxiliary_driver_from_device_driver(driver) };
    if auxdrv.is_null() {
        return -EINVAL;
    }
    let id = unsafe { auxiliary_match_id((*auxdrv).id_table, auxdev) };
    if id.is_null() {
        return -EINVAL;
    }
    match unsafe { (*auxdrv).probe } {
        Some(probe) => unsafe { probe(auxdev, id) },
        None => -EINVAL,
    }
}

unsafe extern "C" fn linux_auxiliary_bus_remove(dev: *mut c_void) {
    let auxdev = dev.cast::<LinuxAuxiliaryDevice>();
    let driver = unsafe { (*dev.cast::<LinuxDevice>()).driver };
    let auxdrv = unsafe { auxiliary_driver_from_device_driver(driver) };
    if !auxdrv.is_null() {
        if let Some(remove) = unsafe { (*auxdrv).remove } {
            unsafe { remove(auxdev) };
        }
    }
}

unsafe extern "C" fn linux_auxiliary_bus_shutdown(dev: *mut c_void) {
    let auxdev = dev.cast::<LinuxAuxiliaryDevice>();
    let driver = unsafe { (*dev.cast::<LinuxDevice>()).driver };
    let auxdrv = unsafe { auxiliary_driver_from_device_driver(driver) };
    if !auxdrv.is_null() {
        if let Some(shutdown) = unsafe { (*auxdrv).shutdown } {
            unsafe { shutdown(auxdev) };
        }
    }
}

/// `auxiliary_device_init` - `vendor/linux/drivers/base/auxiliary.c:275`.
#[unsafe(export_name = "auxiliary_device_init")]
pub unsafe extern "C" fn linux_auxiliary_device_init(auxdev: *mut LinuxAuxiliaryDevice) -> i32 {
    if auxdev.is_null() {
        return -EINVAL;
    }
    let dev = unsafe { core::ptr::addr_of_mut!((*auxdev).dev) };
    if unsafe { (*dev).parent.is_null() || auxdev_name(auxdev).is_null() } {
        return -EINVAL;
    }

    unsafe {
        (*dev).bus = core::ptr::addr_of!(LINUX_AUXILIARY_BUS_TYPE);
        linux_device_initialize(dev);
    }
    0
}

fn push_decimal(buf: &mut [u8], pos: &mut usize, mut value: u32) -> Result<(), i32> {
    let mut digits = [0u8; 10];
    let mut len = 0usize;
    if value == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while value != 0 {
            digits[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
    }
    while len != 0 {
        len -= 1;
        if *pos >= buf.len() {
            return Err(-EINVAL);
        }
        buf[*pos] = digits[len];
        *pos += 1;
    }
    Ok(())
}

unsafe fn format_auxiliary_device_name(
    modname: *const c_char,
    name: *const c_char,
    id: u32,
) -> Result<[u8; 64], i32> {
    if modname.is_null() || name.is_null() {
        return Err(-EINVAL);
    }
    let mod_len = unsafe { c_strlen(modname, 64) };
    let name_len = unsafe { c_strlen(name, 64) };
    if mod_len == 0 || name_len == 0 || mod_len + name_len + 13 >= 64 {
        return Err(-EINVAL);
    }

    let mut out = [0u8; 64];
    let mut pos = 0usize;
    unsafe {
        core::ptr::copy_nonoverlapping(modname.cast::<u8>(), out.as_mut_ptr(), mod_len);
    }
    pos += mod_len;
    out[pos] = b'.';
    pos += 1;
    unsafe {
        core::ptr::copy_nonoverlapping(name.cast::<u8>(), out.as_mut_ptr().add(pos), name_len);
    }
    pos += name_len;
    out[pos] = b'.';
    pos += 1;
    push_decimal(&mut out, &mut pos, id)?;
    out[pos] = 0;
    Ok(out)
}

/// `__auxiliary_device_add` - `vendor/linux/drivers/base/auxiliary.c:315`.
#[unsafe(export_name = "__auxiliary_device_add")]
pub unsafe extern "C" fn linux___auxiliary_device_add(
    auxdev: *mut LinuxAuxiliaryDevice,
    modname: *const c_char,
) -> i32 {
    if auxdev.is_null() || modname.is_null() {
        return -EINVAL;
    }
    let dev = unsafe { core::ptr::addr_of_mut!((*auxdev).dev) };
    let name = unsafe { auxdev_name(auxdev) };
    let id = unsafe { auxdev_id(auxdev) };
    let formatted = match unsafe { format_auxiliary_device_name(modname, name, id) } {
        Ok(name) => name,
        Err(err) => return err,
    };
    match unsafe { linux_device_set_name_bytes(dev, &formatted) } {
        Ok(()) => unsafe { linux_device_add(dev) },
        Err(errno) => -errno,
    }
}

unsafe fn alloc_c_string(parts: &[&[u8]]) -> *mut c_char {
    let len = parts.iter().map(|part| part.len()).sum::<usize>();
    let ptr = unsafe { linux___kmalloc_noprof(len + 1, GFP_KERNEL) };
    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    let mut pos = 0usize;
    for part in parts {
        unsafe { core::ptr::copy_nonoverlapping(part.as_ptr(), ptr.add(pos), part.len()) };
        pos += part.len();
    }
    unsafe { *ptr.add(pos) = 0 };
    ptr.cast()
}

unsafe fn c_bytes<'a>(ptr: *const c_char, max: usize) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return None;
    }
    let len = unsafe { c_strlen(ptr, max) };
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

/// `__auxiliary_driver_register` - `vendor/linux/drivers/base/auxiliary.c:350`.
#[unsafe(export_name = "__auxiliary_driver_register")]
pub unsafe extern "C" fn linux___auxiliary_driver_register(
    auxdrv: *mut LinuxAuxiliaryDriver,
    owner: *mut c_void,
    modname: *const c_char,
) -> i32 {
    if auxdrv.is_null() || modname.is_null() {
        return -EINVAL;
    }
    if unsafe { (*auxdrv).probe.is_none() || (*auxdrv).id_table.is_null() } {
        return -EINVAL;
    }

    let Some(modname_bytes) = (unsafe { c_bytes(modname, 64) }) else {
        return -EINVAL;
    };
    let driver_name = if unsafe { (*auxdrv).name.is_null() } {
        unsafe { alloc_c_string(&[modname_bytes]) }
    } else {
        let Some(name_bytes) = (unsafe { c_bytes((*auxdrv).name, 64) }) else {
            return -EINVAL;
        };
        unsafe { alloc_c_string(&[modname_bytes, b".", name_bytes]) }
    };
    if driver_name.is_null() {
        return -ENOMEM;
    }

    let driver = unsafe { core::ptr::addr_of_mut!((*auxdrv).driver) };
    unsafe {
        (*driver).name = driver_name;
        (*driver).owner = owner;
        (*driver).bus = core::ptr::addr_of!(LINUX_AUXILIARY_BUS_TYPE);
        (*driver).mod_name = modname;
    }

    let ret = unsafe { linux_driver_register(driver) };
    if ret != 0 {
        unsafe {
            linux_kfree(driver_name.cast());
            (*driver).name = core::ptr::null();
        }
    }
    ret
}

/// `auxiliary_driver_unregister` - `vendor/linux/drivers/base/auxiliary.c:382`.
#[unsafe(export_name = "auxiliary_driver_unregister")]
pub unsafe extern "C" fn linux_auxiliary_driver_unregister(auxdrv: *mut LinuxAuxiliaryDriver) {
    if auxdrv.is_null() {
        return;
    }
    let driver = unsafe { core::ptr::addr_of_mut!((*auxdrv).driver) };
    let name = unsafe { (*driver).name };
    unsafe {
        linux_driver_unregister(driver);
        if !name.is_null() {
            linux_kfree(name.cast::<u8>().cast_mut());
            (*driver).name = core::ptr::null();
        }
    }
}

/// `auxiliary_device_delete` - `vendor/linux/include/linux/auxiliary_bus.h`.
#[unsafe(export_name = "auxiliary_device_delete")]
pub unsafe extern "C" fn linux_auxiliary_device_delete(auxdev: *mut LinuxAuxiliaryDevice) {
    if !auxdev.is_null() {
        unsafe { linux_device_unregister(core::ptr::addr_of_mut!((*auxdev).dev)) };
    }
}

/// `auxiliary_device_uninit` - `vendor/linux/include/linux/auxiliary_bus.h`.
#[unsafe(export_name = "auxiliary_device_uninit")]
pub unsafe extern "C" fn linux_auxiliary_device_uninit(auxdev: *mut LinuxAuxiliaryDevice) {
    if !auxdev.is_null() {
        unsafe { put_device(core::ptr::addr_of_mut!((*auxdev).dev)) };
    }
}

#[unsafe(export_name = "auxiliary_device_sysfs_irq_add")]
pub unsafe extern "C" fn linux_auxiliary_device_sysfs_irq_add(
    _auxdev: *mut LinuxAuxiliaryDevice,
    _irq: i32,
) -> i32 {
    0
}

#[unsafe(export_name = "auxiliary_device_sysfs_irq_remove")]
pub unsafe extern "C" fn linux_auxiliary_device_sysfs_irq_remove(
    _auxdev: *mut LinuxAuxiliaryDevice,
    _irq: i32,
) {
}

/// `dev_is_auxiliary` - `vendor/linux/drivers/base/auxiliary.c:503`.
#[unsafe(export_name = "dev_is_auxiliary")]
pub unsafe extern "C" fn linux_dev_is_auxiliary(dev: *mut LinuxDevice) -> bool {
    !dev.is_null() && unsafe { (*dev).bus == core::ptr::addr_of!(LINUX_AUXILIARY_BUS_TYPE) }
}

#[allow(dead_code)]
pub unsafe fn auxiliary_get_device(auxdev: *mut LinuxAuxiliaryDevice) -> *mut LinuxDevice {
    if auxdev.is_null() {
        core::ptr::null_mut()
    } else {
        unsafe { get_device(core::ptr::addr_of_mut!((*auxdev).dev)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auxiliary_exports_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/drivers/base/auxiliary.c"
        ));
        assert!(source.contains("EXPORT_SYMBOL_GPL(auxiliary_device_init);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__auxiliary_device_add);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__auxiliary_driver_register);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(auxiliary_driver_unregister);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(dev_is_auxiliary);"));

        register_module_exports();
        assert_eq!(
            find_symbol("auxiliary_device_init"),
            Some(linux_auxiliary_device_init as usize)
        );
        assert_eq!(
            find_symbol("__auxiliary_device_add"),
            Some(linux___auxiliary_device_add as usize)
        );
        assert_eq!(
            find_symbol("__auxiliary_driver_register"),
            Some(linux___auxiliary_driver_register as usize)
        );
    }

    #[test]
    fn auxiliary_device_layout_offsets_match_vendor_probe() {
        assert_eq!(LINUX_STRUCT_DEVICE_SIZE, 760);
        assert_eq!(LINUX_AUXILIARY_DEVICE_NAME_OFFSET, 760);
        assert_eq!(LINUX_AUXILIARY_DEVICE_ID_OFFSET, 768);
        assert_eq!(LINUX_AUXILIARY_DRIVER_DRIVER_OFFSET, 48);
    }

    #[test]
    fn auxiliary_device_add_formats_linux_device_name() {
        #[repr(align(8))]
        struct AlignedStorage([u8; 840]);

        let mut storage = AlignedStorage([0u8; 840]);
        let auxdev = storage.0.as_mut_ptr().cast::<LinuxAuxiliaryDevice>();
        let parent = storage
            .0
            .as_mut_ptr()
            .wrapping_add(800)
            .cast::<LinuxDevice>();
        unsafe {
            (*auxdev).dev.parent = parent;
            storage
                .0
                .as_mut_ptr()
                .add(LINUX_AUXILIARY_DEVICE_NAME_OFFSET)
                .cast::<*const c_char>()
                .write(c"gsc".as_ptr());
            storage
                .0
                .as_mut_ptr()
                .add(LINUX_AUXILIARY_DEVICE_ID_OFFSET)
                .cast::<u32>()
                .write(7);

            assert_eq!(linux_auxiliary_device_init(auxdev), 0);
            assert_eq!(linux___auxiliary_device_add(auxdev, c"i915".as_ptr()), 0);
            let name = device_name(core::ptr::addr_of!((*auxdev).dev));
            assert_eq!(crate::lib::string::c_strlen(name, 64), 10);
            assert_eq!(
                core::slice::from_raw_parts(name.cast::<u8>(), 10),
                b"i915.gsc.7"
            );
            linux_auxiliary_device_delete(auxdev);
            linux_auxiliary_device_uninit(auxdev);
        }
    }
}
