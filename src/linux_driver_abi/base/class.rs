//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base/class.c
//! test-origin: linux:vendor/linux/drivers/base/class.c
//! `struct class` — `vendor/linux/include/linux/device/class.h`.
//!
//! A class is a logical grouping of devices that share a userspace contract
//! (e.g. `block`, `net`, `tty`, `input`).  Each registered class appears as
//! `/sys/class/<name>/`; member devices are linked under it.
//!
//! Mirrors `drivers/base/class.c:178` (`class_register`).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::base::device::Device;
use crate::mm::frame::PAGE_SIZE;
use crate::mm::page_flags::GFP_KERNEL;

const LINUX_CLASS_NAME_OFFSET: usize = 0;
const LINUX_CLASS_SIZE: usize = 96;
const LINUX_CLASS_ATTRIBUTE_STRING_STR_OFFSET: usize = 32;

pub struct Class {
    pub name: &'static str,
    pub devices: Mutex<Vec<Arc<Device>>>,
}

impl Class {
    pub fn new(name: &'static str) -> Arc<Self> {
        Arc::new(Self {
            name,
            devices: Mutex::new(Vec::new()),
        })
    }
}

lazy_static! {
    pub(crate) static ref CLASSES: Mutex<BTreeMap<String, Arc<Class>>> =
        Mutex::new(BTreeMap::new());
}

/// `class_register` — `drivers/base/class.c:178`.
pub fn class_register(class: Arc<Class>) -> Result<(), i32> {
    let mut g = CLASSES.lock();
    if g.contains_key(class.name) {
        return Err(EEXIST);
    }
    g.insert(String::from(class.name), class);
    Ok(())
}

pub fn registered_classes() -> Vec<&'static str> {
    CLASSES.lock().values().map(|c| c.name).collect()
}

pub fn find_class(name: &str) -> Option<Arc<Class>> {
    CLASSES.lock().get(name).cloned()
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("class_create", linux_class_create as usize, true);
    export_symbol_once("class_destroy", linux_class_destroy as usize, true);
    export_symbol_once(
        "class_create_file_ns",
        linux_class_create_file_ns as usize,
        true,
    );
    export_symbol_once(
        "class_remove_file_ns",
        linux_class_remove_file_ns as usize,
        true,
    );
    export_symbol_once(
        "show_class_attr_string",
        linux_show_class_attr_string as usize,
        true,
    );
}

#[cfg(not(test))]
fn kzalloc(size: usize) -> *mut u8 {
    unsafe {
        crate::mm::slab::linux___kmalloc_noprof(
            size,
            GFP_KERNEL | crate::mm::page_flags::__GFP_ZERO,
        )
    }
}

#[cfg(not(test))]
fn kfree(ptr: *mut u8) {
    unsafe { crate::mm::slab::linux_kfree(ptr) };
}

#[cfg(test)]
fn kzalloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    unsafe {
        let block = alloc::alloc::alloc_zeroed(layout);
        if block.is_null() {
            return block;
        }
        *(block as *mut usize) = size;
        block.add(16)
    }
}

#[cfg(test)]
fn kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let block = ptr.sub(16);
        let size = *(block as *const usize);
        let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
        alloc::alloc::dealloc(block, layout);
    }
}

fn err_ptr<T>(errno: i32) -> *mut T {
    (-(errno as isize)) as *mut T
}

fn is_err_or_null(ptr: *const c_void) -> bool {
    ptr.is_null() || (ptr as usize) >= usize::MAX - 4095
}

/// `class_create` - `vendor/linux/drivers/base/class.c:266`.
pub unsafe extern "C" fn linux_class_create(name: *const c_char) -> *mut c_void {
    if name.is_null() {
        return err_ptr(EINVAL);
    }
    let class = kzalloc(LINUX_CLASS_SIZE);
    if class.is_null() {
        return err_ptr(ENOMEM);
    }
    unsafe {
        class
            .add(LINUX_CLASS_NAME_OFFSET)
            .cast::<*const c_char>()
            .write(name);
    }
    class.cast()
}

/// `class_destroy` - `vendor/linux/drivers/base/class.c:299`.
pub unsafe extern "C" fn linux_class_destroy(class: *mut c_void) {
    if !is_err_or_null(class) {
        kfree(class.cast());
    }
}

/// `class_create_file_ns` - `vendor/linux/drivers/base/class.c:129`.
pub unsafe extern "C" fn linux_class_create_file_ns(
    class: *const c_void,
    attr: *const c_void,
    _ns: *const c_void,
) -> i32 {
    if is_err_or_null(class) || attr.is_null() {
        -EINVAL
    } else {
        0
    }
}

/// `class_remove_file_ns` - `vendor/linux/drivers/base/class.c:145`.
pub unsafe extern "C" fn linux_class_remove_file_ns(
    _class: *const c_void,
    _attr: *const c_void,
    _ns: *const c_void,
) {
}

/// `show_class_attr_string` - `vendor/linux/drivers/base/class.c:550`.
pub unsafe extern "C" fn linux_show_class_attr_string(
    _class: *const c_void,
    attr: *const c_void,
    buf: *mut c_char,
) -> isize {
    if attr.is_null() || buf.is_null() {
        return 0;
    }

    let str_ptr = unsafe {
        attr.cast::<u8>()
            .add(LINUX_CLASS_ATTRIBUTE_STRING_STR_OFFSET)
            .cast::<*const c_char>()
            .read()
    };
    let source = if str_ptr.is_null() {
        c"(null)".as_ptr()
    } else {
        str_ptr
    };

    let mut written = 0usize;
    while written + 1 < PAGE_SIZE {
        let byte = unsafe { source.add(written).read() };
        if byte == 0 {
            break;
        }
        unsafe {
            buf.add(written).write(byte);
        }
        written += 1;
    }
    if written + 1 < PAGE_SIZE {
        unsafe {
            buf.add(written).write(b'\n' as c_char);
        }
        written += 1;
    }
    unsafe {
        buf.add(written).write(0);
    }
    written as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_create_returns_raw_class_with_name() {
        unsafe {
            let name = c"drm";
            let class = linux_class_create(name.as_ptr());
            assert!(!is_err_or_null(class));
            assert_eq!(class.cast::<*const c_char>().read(), name.as_ptr());
            linux_class_destroy(class);
        }
    }

    #[test]
    fn class_attr_string_show_appends_newline() {
        unsafe {
            let text = c"version";
            let mut attr = [0u8; 40];
            attr.as_mut_ptr()
                .add(LINUX_CLASS_ATTRIBUTE_STRING_STR_OFFSET)
                .cast::<*const c_char>()
                .write(text.as_ptr());
            let mut buf = [0 as c_char; 32];
            let len = linux_show_class_attr_string(
                core::ptr::null(),
                attr.as_ptr().cast(),
                buf.as_mut_ptr(),
            );
            assert_eq!(len, 8);
            assert_eq!(
                core::ffi::CStr::from_ptr(buf.as_ptr()).to_bytes(),
                b"version\n"
            );
        }
    }
}
