//! linux-parity: partial
//! linux-source: vendor/linux/fs/char_dev.c
//! Character-device number registration for Linux-built modules.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};

const MINORBITS: u32 = 20;
const MINORMASK: u32 = (1 << MINORBITS) - 1;
const FIRST_DYNAMIC_MAJOR: u32 = 234;
const LAST_DYNAMIC_MAJOR: u32 = 511;

#[derive(Clone)]
struct CharDeviceRegion {
    major: u32,
    baseminor: u32,
    count: u32,
    name: String,
    fops: usize,
}

lazy_static! {
    static ref CHRDEVS: Mutex<Vec<CharDeviceRegion>> = Mutex::new(Vec::new());
    static ref DYNAMIC_CDEVS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "alloc_chrdev_region",
        linux_alloc_chrdev_region as usize,
        false,
    );
    export_symbol_once(
        "register_chrdev_region",
        linux_register_chrdev_region as usize,
        false,
    );
    export_symbol_once(
        "unregister_chrdev_region",
        linux_unregister_chrdev_region as usize,
        false,
    );
    export_symbol_once("__register_chrdev", linux___register_chrdev as usize, false);
    export_symbol_once(
        "__unregister_chrdev",
        linux___unregister_chrdev as usize,
        false,
    );
    export_symbol_once("cdev_init", linux_cdev_init as usize, false);
    export_symbol_once("cdev_alloc", linux_cdev_alloc as usize, false);
    export_symbol_once("cdev_add", linux_cdev_add as usize, false);
    export_symbol_once("cdev_del", linux_cdev_del as usize, false);
    export_symbol_once("cdev_set_parent", linux_cdev_set_parent as usize, false);
    export_symbol_once("cdev_device_add", linux_cdev_device_add as usize, false);
    export_symbol_once("cdev_device_del", linux_cdev_device_del as usize, false);
}

pub fn registered_chrdev_fops(major: u32) -> Option<usize> {
    CHRDEVS
        .lock()
        .iter()
        .find(|region| region.major == major)
        .map(|region| region.fops)
}

fn mkdev(major: u32, minor: u32) -> u32 {
    (major << MINORBITS) | (minor & MINORMASK)
}

const LINUX_CDEV_OPS_OFFSET: usize = 72;
const LINUX_CDEV_DEV_OFFSET: usize = 96;
const LINUX_CDEV_COUNT_OFFSET: usize = 100;
const LINUX_CDEV_SIZE: usize = 104;
const LINUX_DEVICE_DEVT_OFFSET: usize = 668;

unsafe fn cdev_write_ops(cdev: *mut c_void, fops: *const c_void) {
    unsafe {
        cdev.cast::<u8>()
            .add(LINUX_CDEV_OPS_OFFSET)
            .cast::<*const c_void>()
            .write(fops);
    }
}

unsafe fn cdev_write_dev_count(cdev: *mut c_void, dev: u32, count: u32) {
    unsafe {
        cdev.cast::<u8>()
            .add(LINUX_CDEV_DEV_OFFSET)
            .cast::<u32>()
            .write(dev);
        cdev.cast::<u8>()
            .add(LINUX_CDEV_COUNT_OFFSET)
            .cast::<u32>()
            .write(count);
    }
}

unsafe fn linux_device_devt(dev: *const c_void) -> u32 {
    if dev.is_null() {
        0
    } else {
        unsafe {
            dev.cast::<u8>()
                .add(LINUX_DEVICE_DEVT_OFFSET)
                .cast::<u32>()
                .read()
        }
    }
}

fn ranges_overlap(a_base: u32, a_count: u32, b_base: u32, b_count: u32) -> bool {
    let Some(a_end) = a_base.checked_add(a_count) else {
        return true;
    };
    let Some(b_end) = b_base.checked_add(b_count) else {
        return true;
    };
    a_base < b_end && b_base < a_end
}

unsafe fn c_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let len = unsafe { crate::lib::string::c_strlen(ptr, 256) };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    String::from(core::str::from_utf8(bytes).unwrap_or(""))
}

fn register_chrdev_region(
    requested_major: u32,
    baseminor: u32,
    count: u32,
    name: String,
    fops: usize,
) -> Result<u32, i32> {
    if count == 0 || baseminor > MINORMASK || count - 1 > MINORMASK - baseminor {
        return Err(EINVAL);
    }

    let mut regions = CHRDEVS.lock();
    let major = if requested_major != 0 {
        requested_major
    } else {
        (FIRST_DYNAMIC_MAJOR..=LAST_DYNAMIC_MAJOR)
            .find(|candidate| !regions.iter().any(|region| region.major == *candidate))
            .ok_or(EBUSY)?
    };

    if regions.iter().any(|region| {
        region.major == major && ranges_overlap(region.baseminor, region.count, baseminor, count)
    }) {
        return Err(EBUSY);
    }

    regions.push(CharDeviceRegion {
        major,
        baseminor,
        count,
        name,
        fops,
    });
    Ok(major)
}

unsafe extern "C" fn linux_register_chrdev_region(
    from: u32,
    count: u32,
    name: *const c_char,
) -> i32 {
    let major = from >> MINORBITS;
    let baseminor = from & MINORMASK;
    match register_chrdev_region(major, baseminor, count, unsafe { c_string(name) }, 0) {
        Ok(_) => 0,
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux_alloc_chrdev_region(
    dev: *mut u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    match register_chrdev_region(0, baseminor, count, unsafe { c_string(name) }, 0) {
        Ok(major) => {
            unsafe { dev.write(mkdev(major, baseminor)) };
            0
        }
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux___register_chrdev(
    major: u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
    fops: *const c_void,
) -> i32 {
    match register_chrdev_region(
        major,
        baseminor,
        count,
        unsafe { c_string(name) },
        fops as usize,
    ) {
        Ok(allocated_major) if major == 0 => allocated_major as i32,
        Ok(_) => 0,
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux_unregister_chrdev_region(from: u32, count: u32) {
    let major = from >> MINORBITS;
    let baseminor = from & MINORMASK;
    CHRDEVS.lock().retain(|region| {
        !(region.major == major && ranges_overlap(region.baseminor, region.count, baseminor, count))
    });
}

unsafe extern "C" fn linux___unregister_chrdev(
    major: u32,
    baseminor: u32,
    count: u32,
    name: *const c_char,
) {
    let name = unsafe { c_string(name) };
    CHRDEVS.lock().retain(|region| {
        !(region.major == major
            && region.baseminor == baseminor
            && region.count == count
            && (name.is_empty() || region.name == name))
    });
}

unsafe extern "C" fn linux_cdev_init(cdev: *mut c_void, fops: *const c_void) {
    if cdev.is_null() {
        return;
    }
    unsafe {
        core::ptr::write_bytes(cdev.cast::<u8>(), 0, LINUX_CDEV_SIZE);
        cdev_write_ops(cdev, fops);
    }
}

unsafe extern "C" fn linux_cdev_alloc() -> *mut c_void {
    let cdev = Box::into_raw(Box::new([0u8; LINUX_CDEV_SIZE])).cast::<c_void>();
    DYNAMIC_CDEVS.lock().push(cdev as usize);
    cdev
}

unsafe extern "C" fn linux_cdev_add(cdev: *mut c_void, dev: u32, count: u32) -> i32 {
    if cdev.is_null() || count == 0 {
        return -EINVAL;
    }
    unsafe { cdev_write_dev_count(cdev, dev, count) };
    0
}

unsafe extern "C" fn linux_cdev_del(cdev: *mut c_void) {
    if cdev.is_null() {
        return;
    }
    let mut dynamic = DYNAMIC_CDEVS.lock();
    if let Some(pos) = dynamic.iter().position(|ptr| *ptr == cdev as usize) {
        dynamic.swap_remove(pos);
        unsafe {
            let _ = Box::from_raw(cdev.cast::<[u8; LINUX_CDEV_SIZE]>());
        }
    }
}

unsafe extern "C" fn linux_cdev_set_parent(_cdev: *mut c_void, _kobj: *mut c_void) {}

unsafe extern "C" fn linux_cdev_device_add(
    cdev: *mut c_void,
    dev: *mut crate::linux_driver_abi::base::LinuxDevice,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let devt = unsafe { linux_device_devt(dev.cast_const().cast()) };
    if devt != 0 {
        let rc = unsafe { linux_cdev_add(cdev, devt, 1) };
        if rc != 0 {
            return rc;
        }
    }
    unsafe { crate::linux_driver_abi::base::linux_device_add(dev) }
}

unsafe extern "C" fn linux_cdev_device_del(
    cdev: *mut c_void,
    dev: *mut crate::linux_driver_abi::base::LinuxDevice,
) {
    if !dev.is_null() {
        unsafe { crate::linux_driver_abi::base::linux_device_unregister(dev) };
    }
    if !cdev.is_null() {
        unsafe { linux_cdev_del(cdev) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_chrdev_region_returns_dynamic_dev_t() {
        let name = b"test\0";
        let mut dev = 0;
        let rc = unsafe { linux_alloc_chrdev_region(&mut dev, 2, 4, name.as_ptr().cast()) };
        assert_eq!(rc, 0);
        assert_eq!(dev & MINORMASK, 2);
        assert!(dev >> MINORBITS >= FIRST_DYNAMIC_MAJOR);
    }
}
