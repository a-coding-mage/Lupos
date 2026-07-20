//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base
//! test-origin: linux:vendor/linux/drivers/base
//! `struct device` — `vendor/linux/include/linux/device.h:611`.
//!
//! Lupos `Device` carries the same shape and lifecycle Linux exposes:
//!   * `name`, `init_name`     — sysfs name and seed parent.
//!   * `parent`                — `Option<Arc<Device>>`, mirrors `dev->parent`.
//!   * `bus`                   — `Option<Arc<BusType>>`; bus dispatches probe.
//!   * `driver`                — `Option<Arc<DeviceDriver>>`; bound after probe.
//!   * `class`                 — `Option<Arc<Class>>`; appears under `/sys/class`.
//!   * `kobj`                  — kobject for sysfs.
//!   * `driver_data`           — per-driver opaque (atomic `usize`).
//!   * `compatible`            — match string (Linux uses OF/ACPI tables; we
//!                                accept a single compatible string for now).
//!
//! Lifecycle:
//!   `device_initialize` (in `Device::new`) → `device_add` →
//!   `bus_probe_device` → `driver_probe_device` →
//!   on unload: `device_del` → `device_unregister`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENODEV};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::lib::kobject::{KObject, kobject_add};
use crate::linux_driver_abi::base::bus::{
    BusType, LinuxBusType, bus_probe_device, linux_bus_type_registered,
};
use crate::linux_driver_abi::base::class::Class;
use crate::linux_driver_abi::base::driver::{DeviceDriver, LinuxDeviceDriver};

pub struct Device {
    pub name: String,
    pub compatible: Mutex<Option<String>>,
    pub parent: Mutex<Option<Arc<Device>>>,
    pub bus: Mutex<Option<Arc<BusType>>>,
    pub driver: Mutex<Option<Arc<DeviceDriver>>>,
    pub class: Mutex<Option<Arc<Class>>>,
    pub kobj: Arc<KObject>,
    pub driver_data: AtomicUsize,
    pub state: Mutex<DeviceState>,
    me: Mutex<Weak<Device>>,
}

/// `struct list_head` — `vendor/linux/include/linux/types.h:204`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LinuxListHead {
    pub next: *mut c_void,
    pub prev: *mut c_void,
}

/// Prefix of `struct kobject` — `vendor/linux/include/linux/kobject.h`.
#[repr(C)]
pub struct LinuxKObject {
    pub name: *const c_char,
    pub entry: LinuxListHead,
    pub parent: *mut LinuxKObject,
    pub kset: *mut c_void,
    pub ktype: *const c_void,
    pub sd: *mut c_void,
    pub kref: i32,
    pub state_flags: u32,
}

/// Size and release-field offset of `struct device` in the configured vendor
/// x86_64 ABI.
pub const LINUX_STRUCT_DEVICE_SIZE: usize = 760;
pub const LINUX_DEVICE_RELEASE_OFFSET: usize = 712;
const LINUX_DEVICE_PREFIX_SIZE: usize = 128;
const LINUX_DEVICE_TYPE_DEVNODE_OFFSET: usize = 24;
const LINUX_DEVICE_DEVT_OFFSET: usize = 668;
const LINUX_DEVICE_CLASS_OFFSET: usize = 696;
const LINUX_CLASS_NAME_OFFSET: usize = 0;
const LINUX_CLASS_DEVNODE_OFFSET: usize = 32;

/// `struct device` with its directly consumed prefix and ABI-preserving tail.
///
/// Source: `vendor/linux/include/linux/device.h:628`. Fields after
/// `driver_data` remain opaque here, but the allocation has the complete
/// vendor size so subsystems such as runtime PM can safely access them at
/// their probed ABI offsets.
#[repr(C)]
pub struct LinuxDevice {
    pub kobj: LinuxKObject,
    pub parent: *mut LinuxDevice,
    pub p: *mut c_void,
    pub init_name: *const c_char,
    pub type_: *const c_void,
    pub bus: *const LinuxBusType,
    pub driver: *mut LinuxDeviceDriver,
    pub platform_data: *mut c_void,
    pub driver_data: *mut c_void,
    pub _abi_tail: [u8; LINUX_STRUCT_DEVICE_SIZE - LINUX_DEVICE_PREFIX_SIZE],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxDeviceRegistration {
    pub device: usize,
    pub bus: usize,
    pub driver: usize,
    pub name: usize,
}

struct LinuxDevicePrivate {
    device: usize,
    bus: usize,
    driver: usize,
    name: [u8; 64],
}

const KOBJ_STATE_INITIALIZED: u32 = 1 << 0;
const KOBJ_STATE_IN_SYSFS: u32 = 1 << 1;
pub type LinuxDeviceReleaseFn = unsafe extern "C" fn(*mut LinuxDevice);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceState {
    Initialized,
    Added,
    Bound,
    Unbound,
    Removed,
}

impl Device {
    pub fn new(name: &str) -> Arc<Self> {
        let kobj = KObject::new(name, None);
        let dev = Arc::new(Self {
            name: String::from(name),
            compatible: Mutex::new(None),
            parent: Mutex::new(None),
            bus: Mutex::new(None),
            driver: Mutex::new(None),
            class: Mutex::new(None),
            kobj,
            driver_data: AtomicUsize::new(0),
            state: Mutex::new(DeviceState::Initialized),
            me: Mutex::new(Weak::new()),
        });
        *dev.me.lock() = Arc::downgrade(&dev);
        dev
    }

    /// Linux equivalent: `dev_set_drvdata(dev, p)`.
    pub fn set_drvdata(&self, ptr: usize) {
        self.driver_data.store(ptr, Ordering::Release);
    }
    /// Linux equivalent: `dev_get_drvdata(dev)`.
    pub fn get_drvdata(&self) -> usize {
        self.driver_data.load(Ordering::Acquire)
    }

    pub fn this(&self) -> Arc<Device> {
        self.me.lock().upgrade().expect("Device::this after drop")
    }
}

// ── Registry — mirrors `devices_kset` plus class/bus listings ────────────────

lazy_static! {
    /// Every registered device, keyed by name.  Linux indexes via the kobject
    /// glue; the key is the unique device name (e.g. `synthetic.0`,
    /// `0000:00:01.0`).
    pub(crate) static ref DEVICES: Mutex<BTreeMap<String, Arc<Device>>> =
        Mutex::new(BTreeMap::new());
    static ref LINUX_DEVICES: Mutex<Vec<LinuxDeviceRegistration>> = Mutex::new(Vec::new());
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("device_initialize", linux_device_initialize as usize, true);
    export_symbol_once("device_add", linux_device_add as usize, true);
    export_symbol_once("device_attach", linux_device_attach as usize, true);
    export_symbol_once(
        "device_bind_driver",
        linux_device_bind_driver as usize,
        true,
    );
    export_symbol_once("device_register", linux_device_register as usize, true);
    export_symbol_once(
        "device_release_driver",
        linux_device_release_driver as usize,
        true,
    );
    export_symbol_once("device_unregister", linux_device_unregister as usize, true);
    export_symbol_once(
        "device_set_wakeup_enable",
        device_set_wakeup_enable as usize,
        true,
    );
    export_symbol_once("device_wakeup_enable", device_wakeup_enable as usize, true);
    export_symbol_once(
        "device_wakeup_disable",
        device_wakeup_disable as usize,
        true,
    );
    export_symbol_once(
        "device_set_wakeup_capable",
        device_set_wakeup_capable as usize,
        true,
    );
    export_symbol_once("get_device", get_device as usize, true);
    export_symbol_once("put_device", put_device as usize, true);
    export_symbol_once("device_match_name", linux_device_match_name as usize, true);
    export_symbol_once("device_match_any", linux_device_match_any as usize, true);
    export_symbol_once(
        "device_match_acpi_handle",
        linux_device_match_acpi_handle as usize,
        false,
    );
    export_symbol_once("dev_err_probe", linux_dev_err_probe as usize, true);
    export_symbol_once("_dev_printk", linux_dev_printk as usize, false);
    export_symbol_once("_dev_emerg", linux_dev_emerg as usize, false);
    export_symbol_once("_dev_err", linux_dev_err as usize, false);
    export_symbol_once("_dev_warn", linux_dev_warn as usize, false);
    export_symbol_once("_dev_notice", linux_dev_notice as usize, false);
    export_symbol_once("_dev_info", linux_dev_info as usize, false);
}

macro_rules! define_linux_dev_printk_level {
    ($function:ident, $symbol:literal, $level:expr) => {
        /// x86-64 C-variadic trampoline for the corresponding Linux
        /// `define_dev_printk_level()` entry point.
        #[unsafe(naked)]
        #[unsafe(export_name = $symbol)]
        pub unsafe extern "C" fn $function() {
            core::arch::naked_asm!(
                "sub rsp, 40",
                "mov qword ptr [rsp], rdx",
                "mov qword ptr [rsp + 8], rcx",
                "mov qword ptr [rsp + 16], r8",
                "mov qword ptr [rsp + 24], r9",
                "lea rcx, [rsp]",
                "lea r8, [rsp + 48]",
                "mov edx, {level}",
                "call {helper}",
                "add rsp, 40",
                "ret",
                level = const $level,
                helper = sym linux_dev_printk_helper,
            );
        }
    };
}

define_linux_dev_printk_level!(linux_dev_emerg, "_dev_emerg", 0);
define_linux_dev_printk_level!(linux_dev_err, "_dev_err", 3);
define_linux_dev_printk_level!(linux_dev_warn, "_dev_warn", 4);
define_linux_dev_printk_level!(linux_dev_notice, "_dev_notice", 5);
define_linux_dev_printk_level!(linux_dev_info, "_dev_info", 6);

/// x86-64 C-variadic trampoline for Linux
/// `_dev_printk(const char *level, const struct device *dev, const char *fmt, ...)`.
#[unsafe(naked)]
#[unsafe(export_name = "_dev_printk")]
pub unsafe extern "C" fn linux_dev_printk() {
    core::arch::naked_asm!(
        "sub rsp, 32",
        "mov qword ptr [rsp], rcx",
        "mov qword ptr [rsp + 8], r8",
        "mov qword ptr [rsp + 16], r9",
        "lea rcx, [rsp]",
        "lea r8, [rsp + 40]",
        "call {helper}",
        "add rsp, 32",
        "ret",
        helper = sym linux_dev_printk_level_helper,
    );
}

unsafe fn linux_dev_printk_level(level: *const c_char) -> u32 {
    if level.is_null() {
        return 6;
    }
    let bytes = level.cast::<u8>();
    let first = unsafe { *bytes };
    let second = unsafe { *bytes.add(1) };
    if first == 1 && (b'0'..=b'7').contains(&second) {
        (second - b'0') as u32
    } else {
        6
    }
}

#[inline(never)]
unsafe extern "C" fn linux_dev_printk_level_helper(
    level: *const c_char,
    dev: *const LinuxDevice,
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) {
    let level = unsafe { linux_dev_printk_level(level) };
    unsafe { linux_dev_printk_helper(dev, fmt, level, register_args, stack_args) };
}

unsafe fn linux_device_c_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    let len = unsafe { crate::lib::string::c_strlen(ptr, 512) };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    core::str::from_utf8(bytes).unwrap_or("")
}

unsafe fn linux_sysfs_streq(left: *const c_char, right: *const c_char) -> bool {
    if left.is_null() || right.is_null() {
        return false;
    }

    let mut idx = 0usize;
    loop {
        let l = unsafe { *left.cast::<u8>().add(idx) };
        let r = unsafe { *right.cast::<u8>().add(idx) };
        if l == r {
            if l == 0 {
                return true;
            }
            idx += 1;
            continue;
        }

        return (l == 0 && r == b'\n' && unsafe { *right.cast::<u8>().add(idx + 1) } == 0)
            || (l == b'\n' && r == 0 && unsafe { *left.cast::<u8>().add(idx + 1) } == 0);
    }
}

#[inline(never)]
unsafe extern "C" fn linux_dev_printk_helper(
    dev: *const LinuxDevice,
    fmt: *const c_char,
    level: u32,
    register_args: *const usize,
    stack_args: *const usize,
) {
    let mut message_buf = [0u8; crate::kernel::printk::log::MSG_CAP];
    let message_len = unsafe {
        super::printf::vscnprintf(
            message_buf.as_mut_ptr(),
            message_buf.len(),
            fmt,
            register_args,
            stack_args,
        )
    };
    let message = core::str::from_utf8(&message_buf[..message_len]).unwrap_or("");
    // Lupos's console renderer appends the record newline. Linux stores the
    // terminator as LOG_NEWLINE rather than as message text, so remove exactly
    // one terminal newline before handing the record to that renderer.
    let message = message.strip_suffix('\n').unwrap_or(message);
    let log_level = match level {
        0..=3 => crate::kernel::printk::log::Level::Error,
        4 => crate::kernel::printk::log::Level::Warn,
        _ => crate::kernel::printk::log::Level::Info,
    };

    if dev.is_null() {
        crate::kernel::printk::log::_log(log_level, "", format_args!("(NULL device *): {message}"));
        return;
    }

    let device_name = unsafe { linux_device_c_str(linux_dev_name(&*dev)) };
    let driver = unsafe { (*dev).driver };
    let driver_name = if !driver.is_null() {
        unsafe { linux_device_c_str((*driver).name) }
    } else {
        let bus = unsafe { (*dev).bus };
        if bus.is_null() {
            ""
        } else {
            unsafe { linux_device_c_str((*bus).name) }
        }
    };

    crate::kernel::printk::log::_log(
        log_level,
        driver_name,
        format_args!("{device_name}: {message}"),
    );
}

pub fn registered_linux_device_count() -> usize {
    LINUX_DEVICES.lock().len()
}

pub fn linux_device_registered(dev: *const LinuxDevice) -> bool {
    LINUX_DEVICES
        .lock()
        .iter()
        .any(|registered| registered.device == dev as usize)
}

pub fn linux_device_driver(dev: *const LinuxDevice) -> *mut LinuxDeviceDriver {
    LINUX_DEVICES
        .lock()
        .iter()
        .find(|registered| registered.device == dev as usize)
        .map(|registered| registered.driver as *mut LinuxDeviceDriver)
        .unwrap_or(core::ptr::null_mut())
}

pub fn linux_devices_on_bus(bus: *const LinuxBusType) -> Vec<*mut LinuxDevice> {
    LINUX_DEVICES
        .lock()
        .iter()
        .filter(|registered| registered.bus == bus as usize)
        .map(|registered| registered.device as *mut LinuxDevice)
        .collect()
}

fn record_linux_device_driver(dev: *mut LinuxDevice, driver: *mut LinuxDeviceDriver) {
    let dev_addr = dev as usize;
    let driver_addr = driver as usize;
    if let Some(registered) = LINUX_DEVICES
        .lock()
        .iter_mut()
        .find(|registered| registered.device == dev_addr)
    {
        registered.driver = driver_addr;
    }
    unsafe {
        if !(*dev).p.is_null() {
            (*(*dev).p.cast::<LinuxDevicePrivate>()).driver = driver_addr;
        }
    }
}

unsafe fn linux_device_probe_driver(dev: *mut LinuxDevice, driver: *mut LinuxDeviceDriver) -> bool {
    if dev.is_null() || driver.is_null() {
        return false;
    }

    let bus = unsafe { (*dev).bus };
    if bus.is_null() || unsafe { (*driver).bus } != bus {
        return false;
    }
    if unsafe { !(*dev).driver.is_null() } {
        return false;
    }

    let matches = unsafe {
        (*bus)
            .match_fn
            .map(|match_fn| match_fn(dev.cast(), driver.cast_const().cast()))
            .unwrap_or(1)
    };
    if matches <= 0 {
        return false;
    }

    unsafe {
        (*dev).driver = driver;
    }
    let probe_ret = unsafe {
        if let Some(probe) = (*bus).probe {
            probe(dev.cast())
        } else if let Some(probe) = (*driver).probe {
            probe(dev.cast())
        } else {
            0
        }
    };
    if probe_ret == 0 {
        true
    } else {
        unsafe {
            (*dev).driver = core::ptr::null_mut();
        }
        false
    }
}

/// Attach a newly registered raw C driver to devices already present on its bus.
///
/// Linux performs this through `driver_attach()`/`__driver_attach()` after
/// `driver_register()` (`vendor/linux/drivers/base/driver.c` and
/// `vendor/linux/drivers/base/dd.c`). This helper keeps the raw C ABI path
/// symmetrical with `device_add()`: all matching and probing still goes through
/// the bus and driver callbacks supplied by Linux-built code.
pub unsafe fn linux_driver_attach_existing(driver: *mut LinuxDeviceDriver) -> usize {
    if driver.is_null() {
        return 0;
    }
    let bus = unsafe { (*driver).bus };
    if bus.is_null() {
        return 0;
    }

    let devices: Vec<*mut LinuxDevice> = LINUX_DEVICES
        .lock()
        .iter()
        .filter(|registered| registered.bus == bus as usize && registered.driver == 0)
        .map(|registered| registered.device as *mut LinuxDevice)
        .collect();

    let mut attached = 0usize;
    for dev in devices {
        if unsafe { linux_device_probe_driver(dev, driver) } {
            record_linux_device_driver(dev, driver);
            attached += 1;
        }
    }
    attached
}

fn linux_dev_name(dev: &LinuxDevice) -> *const c_char {
    if !dev.init_name.is_null() {
        dev.init_name
    } else {
        dev.kobj.name
    }
}

struct LinuxDevnode {
    name: String,
    class_name: String,
    mode: u16,
    uid: u32,
    gid: u32,
    major: u32,
    minor: u32,
}

unsafe fn linux_device_class_name(dev: *mut LinuxDevice) -> String {
    if dev.is_null() {
        return String::new();
    }
    let class = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_CLASS_OFFSET)
            .cast::<*const u8>()
            .read()
    };
    if class.is_null() {
        return String::new();
    }
    let name = unsafe {
        class
            .add(LINUX_CLASS_NAME_OFFSET)
            .cast::<*const c_char>()
            .read()
    };
    String::from(unsafe { linux_device_c_str(name) })
}

/// Exact `device_get_devnode()` selection order from
/// `vendor/linux/drivers/base/core.c`: device type, class, then `dev_name()`.
unsafe fn linux_device_get_devnode(dev: *mut LinuxDevice) -> Option<LinuxDevnode> {
    if dev.is_null() {
        return None;
    }
    let internal_devt = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_DEVT_OFFSET)
            .cast::<u32>()
            .read()
    };
    let major = crate::init::noinitramfs::major(internal_devt);
    let minor = crate::init::noinitramfs::minor(internal_devt);
    if major == 0 {
        return None;
    }

    let class = unsafe {
        dev.cast::<u8>()
            .add(LINUX_DEVICE_CLASS_OFFSET)
            .cast::<*const u8>()
            .read()
    };
    let class_name = unsafe { linux_device_class_name(dev) };

    let mut mode = 0u16;
    let mut uid = 0u32;
    let mut gid = 0u32;
    let mut allocated = core::ptr::null_mut::<c_char>();

    let type_ = unsafe { (*dev).type_.cast::<u8>() };
    if !type_.is_null() {
        let callback = unsafe {
            type_
                .add(LINUX_DEVICE_TYPE_DEVNODE_OFFSET)
                .cast::<usize>()
                .read()
        };
        if callback != 0 {
            let callback: unsafe extern "C" fn(
                *const LinuxDevice,
                *mut u16,
                *mut u32,
                *mut u32,
            ) -> *mut c_char = unsafe { core::mem::transmute(callback) };
            allocated = unsafe { callback(dev, &mut mode, &mut uid, &mut gid) };
        }
    }
    if allocated.is_null() && !class.is_null() {
        let callback = unsafe { class.add(LINUX_CLASS_DEVNODE_OFFSET).cast::<usize>().read() };
        if callback != 0 {
            let callback: unsafe extern "C" fn(*const LinuxDevice, *mut u16) -> *mut c_char =
                unsafe { core::mem::transmute(callback) };
            allocated = unsafe { callback(dev, &mut mode) };
        }
    }

    let name = if allocated.is_null() {
        let fallback = unsafe { linux_device_c_str(linux_dev_name(&*dev)) };
        if fallback.is_empty() {
            return None;
        }
        String::from(fallback).replace('!', "/")
    } else {
        let name = String::from(unsafe { linux_device_c_str(allocated) });
        unsafe { crate::mm::slab::linux_kfree(allocated.cast()) };
        name
    };
    if name.is_empty() {
        return None;
    }
    Some(LinuxDevnode {
        name,
        class_name,
        mode,
        uid,
        gid,
        major,
        minor,
    })
}

unsafe fn ensure_linux_device_private(dev: *mut LinuxDevice) {
    if unsafe { (*dev).p.is_null() } {
        let private = Box::into_raw(Box::new(LinuxDevicePrivate {
            device: dev as usize,
            bus: 0,
            driver: 0,
            name: [0; 64],
        }));
        unsafe {
            (*dev).p = private.cast();
        }
    }
}

pub unsafe fn linux_device_set_name_index(
    dev: *mut LinuxDevice,
    prefix: &[u8],
    mut index: i32,
) -> Result<(), i32> {
    if dev.is_null() || index < 0 {
        return Err(EINVAL);
    }
    unsafe {
        ensure_linux_device_private(dev);
        let private = (*dev).p.cast::<LinuxDevicePrivate>();
        let name = &mut (*private).name;
        if prefix.len() + 11 >= name.len() {
            return Err(EINVAL);
        }

        let mut pos = 0usize;
        for byte in prefix.iter().copied() {
            name[pos] = byte;
            pos += 1;
        }

        let mut digits = [0u8; 10];
        let mut len = 0usize;
        if index == 0 {
            digits[0] = b'0';
            len = 1;
        } else {
            while index > 0 {
                digits[len] = b'0' + (index % 10) as u8;
                index /= 10;
                len += 1;
            }
        }
        while len > 0 {
            len -= 1;
            name[pos] = digits[len];
            pos += 1;
        }
        name[pos] = 0;
        (*dev).kobj.name = name.as_ptr().cast::<c_char>();
        (*dev).init_name = core::ptr::null();
    }
    Ok(())
}

pub unsafe fn linux_device_set_name_bytes(dev: *mut LinuxDevice, bytes: &[u8]) -> Result<(), i32> {
    if dev.is_null() {
        return Err(EINVAL);
    }
    unsafe {
        ensure_linux_device_private(dev);
        let private = (*dev).p.cast::<LinuxDevicePrivate>();
        let name = &mut (*private).name;
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if len == 0 || len >= name.len() {
            return Err(EINVAL);
        }

        core::ptr::write_bytes(name.as_mut_ptr(), 0, name.len());
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), name.as_mut_ptr(), len);
        (*dev).kobj.name = name.as_ptr().cast::<c_char>();
        (*dev).init_name = core::ptr::null();
    }
    Ok(())
}

unsafe fn free_linux_device_private(dev: *mut LinuxDevice) {
    let private = unsafe { (*dev).p };
    if !private.is_null() {
        unsafe {
            let _ = Box::from_raw(private.cast::<LinuxDevicePrivate>());
            (*dev).p = core::ptr::null_mut();
        }
    }
}

unsafe fn linux_device_release_fn(dev: *mut LinuxDevice) -> Option<LinuxDeviceReleaseFn> {
    if dev.is_null() {
        return None;
    }
    unsafe {
        core::ptr::read(
            dev.cast::<u8>()
                .add(LINUX_DEVICE_RELEASE_OFFSET)
                .cast::<Option<LinuxDeviceReleaseFn>>(),
        )
    }
}

/// Install `dev->release` in a full vendor-layout `struct device`.
///
/// # Safety
///
/// `dev` must point to at least [`LINUX_STRUCT_DEVICE_SIZE`] writable bytes,
/// as it does for module-owned devices and Lupos allocations that embed one.
pub unsafe fn linux_device_set_release(
    dev: *mut LinuxDevice,
    release: Option<LinuxDeviceReleaseFn>,
) {
    if dev.is_null() {
        return;
    }
    unsafe {
        core::ptr::write(
            dev.cast::<u8>()
                .add(LINUX_DEVICE_RELEASE_OFFSET)
                .cast::<Option<LinuxDeviceReleaseFn>>(),
            release,
        );
    }
}

unsafe fn linux_device_release_at_zero(dev: *mut LinuxDevice) {
    let private = unsafe { (*dev).p };
    let release = unsafe { linux_device_release_fn(dev) };
    if let Some(release) = release {
        unsafe { release(dev) };
        if !private.is_null() {
            unsafe {
                let _ = Box::from_raw(private.cast::<LinuxDevicePrivate>());
            }
        }
    } else {
        unsafe { free_linux_device_private(dev) };
    }
}

/// `device_initialize` — `vendor/linux/drivers/base/core.c`.
#[unsafe(export_name = "device_initialize")]
pub unsafe extern "C" fn linux_device_initialize(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    unsafe {
        ensure_linux_device_private(dev);
        crate::lib::kobject::init_linux_kobject_raw(
            core::ptr::addr_of_mut!((*dev).kobj).cast(),
            core::ptr::null(),
        );
        crate::kernel::power::runtime_pm_init_device(dev.cast());
    }
}

/// `device_add` — `vendor/linux/drivers/base/core.c:3573`.
///
/// This raw C ABI path registers the device on its raw bus and attempts
/// Linux-style bus matching. Probe callbacks are called only from raw Linux
/// bus/driver callbacks; no local Rust device driver behavior is synthesized.
#[unsafe(export_name = "device_add")]
pub unsafe extern "C" fn linux_device_add(dev: *mut LinuxDevice) -> i32 {
    if dev.is_null() {
        crate::log_warn!("device", "device_add: null device");
        return -EINVAL;
    }

    unsafe {
        if (*dev).p.is_null() {
            ensure_linux_device_private(dev);
        }
    }

    let bus = unsafe { (*dev).bus };
    if !bus.is_null() && !linux_bus_type_registered(bus) {
        crate::log_warn!(
            "device",
            "device_add: unregistered bus dev={:p} bus={:p}",
            dev,
            bus
        );
        return -EINVAL;
    }

    let name = unsafe { linux_dev_name(&*dev) };
    if name.is_null() {
        crate::log_warn!("device", "device_add: unnamed device {:p}", dev);
        return -EINVAL;
    }

    let dev_addr = dev as usize;
    {
        let devices = LINUX_DEVICES.lock();
        if devices
            .iter()
            .any(|registered| registered.device == dev_addr)
        {
            crate::log_warn!("device", "device_add: duplicate device {:p}", dev);
            return -EBUSY;
        }
    }

    let mut bound_driver = core::ptr::null_mut();
    if !bus.is_null() {
        for driver in crate::linux_driver_abi::base::linux_device_drivers_on_bus(bus) {
            if unsafe { linux_device_probe_driver(dev, driver) } {
                bound_driver = driver;
                break;
            }
        }
    }

    unsafe {
        if !(*dev).init_name.is_null() && (*dev).kobj.name.is_null() {
            (*dev).kobj.name = (*dev).init_name;
        }
        (*dev).init_name = core::ptr::null();
        (*dev).kobj.state_flags |= KOBJ_STATE_IN_SYSFS;
        if !(*dev).p.is_null() {
            let private = (*dev).p.cast::<LinuxDevicePrivate>();
            (*private).bus = bus as usize;
            (*private).driver = bound_driver as usize;
        }
    }
    let name = unsafe { linux_dev_name(&*dev) };

    let mut devices = LINUX_DEVICES.lock();
    if devices
        .iter()
        .any(|registered| registered.device == dev_addr)
    {
        crate::log_warn!("device", "device_add: duplicate device {:p}", dev);
        return -EBUSY;
    }
    devices.push(LinuxDeviceRegistration {
        device: dev_addr,
        bus: bus as usize,
        driver: bound_driver as usize,
        name: name as usize,
    });
    drop(devices);

    let node = unsafe { linux_device_get_devnode(dev) };
    let class_name = unsafe { linux_device_class_name(dev) };
    let device_name = String::from(unsafe { linux_device_c_str(name) });
    let class_devpath = if class_name.is_empty() {
        None
    } else {
        Some(crate::fs::sysfs::mount_ops::publish_linux_class_device(
            dev_addr,
            unsafe { (*dev).parent as usize },
            &class_name,
            &device_name,
            node.as_ref().map(|node| node.name.as_str()),
            node.as_ref().map_or(0, |node| node.major),
            node.as_ref().map_or(0, |node| node.minor),
        ))
    };

    if let Some(node) = node.as_ref() {
        // Like vendor devtmpfs, node creation failure does not fail
        // `device_add()`: userspace discovery still receives the uevent.
        let create_result = crate::init::rootfs::devtmpfs_create_linux_char_node(
            &node.name, node.mode, node.uid, node.gid, node.major, node.minor,
        );
        if let Err(errno) = create_result {
            crate::log_warn!(
                "device",
                "devtmpfs create failed name={} dev={}:{} errno={}",
                node.name,
                node.major,
                node.minor,
                errno
            );
        } else {
            crate::log_info!(
                "device",
                "devtmpfs created /dev/{} dev={}:{} mode={:o}",
                node.name,
                node.major,
                node.minor,
                if node.mode == 0 { 0o600 } else { node.mode }
            );
        }
    }

    if let Some(devpath) = class_devpath {
        crate::net::uevent::announce_class_device_at_path(
            crate::net::uevent::UeventAction::Add,
            &devpath,
            &class_name,
            node.as_ref().map(|node| node.name.as_str()),
            node.as_ref().map(|node| (node.major, node.minor)),
        );
    } else if let Some(node) = node.as_ref() {
        let subsystem = if node.class_name.is_empty() {
            "char"
        } else {
            &node.class_name
        };
        crate::net::uevent::announce_virtual_device(
            crate::net::uevent::UeventAction::Add,
            subsystem,
            unsafe { linux_device_c_str(name) },
            subsystem,
            &node.name,
            node.major,
            node.minor,
        );
    }

    0
}

/// `device_attach` — `vendor/linux/drivers/base/dd.c:1139`.
#[unsafe(export_name = "device_attach")]
pub unsafe extern "C" fn linux_device_attach(dev: *mut LinuxDevice) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }

    let dev_addr = dev as usize;
    let registered = LINUX_DEVICES
        .lock()
        .iter()
        .find(|registered| registered.device == dev_addr)
        .copied();
    let Some(registered) = registered else {
        return -ENODEV;
    };

    if registered.driver != 0 || unsafe { !(*dev).driver.is_null() } {
        return 1;
    }

    let bus = unsafe { (*dev).bus };
    if bus.is_null() || !linux_bus_type_registered(bus) {
        return 0;
    }

    for driver in crate::linux_driver_abi::base::linux_device_drivers_on_bus(bus) {
        if unsafe { linux_device_probe_driver(dev, driver) } {
            record_linux_device_driver(dev, driver);
            return 1;
        }
    }

    0
}

/// `device_bind_driver` — `vendor/linux/drivers/base/dd.c:543`.
///
/// Linux callers set `dev->driver` before calling this helper. The sysfs
/// links and notifier side effects are not modeled yet; keep the raw device
/// registry consistent so driver-core users observe the forced binding.
#[unsafe(export_name = "device_bind_driver")]
pub unsafe extern "C" fn linux_device_bind_driver(dev: *mut LinuxDevice) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }

    let driver = unsafe { (*dev).driver };
    if driver.is_null() {
        return -ENODEV;
    }

    record_linux_device_driver(dev, driver);
    0
}

/// `device_register` — `vendor/linux/drivers/base/core.c:3795`.
#[unsafe(export_name = "device_register")]
pub unsafe extern "C" fn linux_device_register(dev: *mut LinuxDevice) -> i32 {
    unsafe {
        linux_device_initialize(dev);
        linux_device_add(dev)
    }
}

/// `device_unregister` — `vendor/linux/drivers/base/core.c:3918`.
#[unsafe(export_name = "device_unregister")]
pub unsafe extern "C" fn linux_device_unregister(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    let node = unsafe { linux_device_get_devnode(dev) };
    if let Some((devpath, class, devname, major, minor)) =
        crate::fs::sysfs::mount_ops::unpublish_linux_class_device(dev as usize)
    {
        crate::net::uevent::announce_class_device_at_path(
            crate::net::uevent::UeventAction::Remove,
            &devpath,
            &class,
            devname.as_deref(),
            devname.as_ref().map(|_| (major, minor)),
        );
    } else if let Some(node) = node.as_ref() {
        let subsystem = if node.class_name.is_empty() {
            "char"
        } else {
            &node.class_name
        };
        crate::net::uevent::announce_virtual_device(
            crate::net::uevent::UeventAction::Remove,
            subsystem,
            unsafe { linux_device_c_str(linux_dev_name(&*dev)) },
            subsystem,
            &node.name,
            node.major,
            node.minor,
        );
    }
    if let Some(node) = node {
        let _ = crate::init::rootfs::devtmpfs_delete_linux_char_node(&node.name);
    }
    LINUX_DEVICES
        .lock()
        .retain(|registered| registered.device != dev as usize);
    unsafe {
        (*dev).driver = core::ptr::null_mut();
        (*dev).kobj.state_flags &= !KOBJ_STATE_IN_SYSFS;
        free_linux_device_private(dev);
    }
}

/// `device_release_driver` — `vendor/linux/drivers/base/dd.c:1388`.
#[unsafe(export_name = "device_release_driver")]
pub unsafe extern "C" fn linux_device_release_driver(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    let driver = unsafe { (*dev).driver };
    if driver.is_null() {
        record_linux_device_driver(dev, core::ptr::null_mut());
        return;
    }

    let bus = unsafe { (*dev).bus };
    let removed_by_bus = if bus.is_null() {
        false
    } else {
        unsafe {
            (*bus)
                .remove
                .map(|remove| {
                    remove(dev.cast());
                    true
                })
                .unwrap_or(false)
        }
    };
    if !removed_by_bus {
        unsafe {
            if let Some(remove) = (*driver).remove {
                let _ = remove(dev.cast());
            }
        }
    }
    unsafe {
        if let Some(post_unbind) = (*driver).p_cb.post_unbind_rust {
            post_unbind(dev.cast());
        }
        (*dev).driver = core::ptr::null_mut();
    }
    record_linux_device_driver(dev, core::ptr::null_mut());
}

#[unsafe(export_name = "device_wakeup_enable")]
pub unsafe extern "C" fn device_wakeup_enable(dev: *mut LinuxDevice) -> i32 {
    if dev.is_null() { -EINVAL } else { 0 }
}

#[unsafe(export_name = "device_wakeup_disable")]
pub unsafe extern "C" fn device_wakeup_disable(_dev: *mut LinuxDevice) {}

#[unsafe(export_name = "device_set_wakeup_enable")]
pub unsafe extern "C" fn device_set_wakeup_enable(dev: *mut LinuxDevice, enable: bool) -> i32 {
    if enable {
        unsafe { device_wakeup_enable(dev) }
    } else {
        unsafe { device_wakeup_disable(dev) };
        0
    }
}

#[unsafe(export_name = "device_set_wakeup_capable")]
pub unsafe extern "C" fn device_set_wakeup_capable(_dev: *mut LinuxDevice, _capable: bool) {}

/// `get_device` - `vendor/linux/drivers/base/core.c:3800`.
#[unsafe(export_name = "get_device")]
pub unsafe extern "C" fn get_device(dev: *mut LinuxDevice) -> *mut LinuxDevice {
    if dev.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        (*dev).kobj.kref = (*dev).kobj.kref.saturating_add(1);
    }
    dev
}

/// `put_device` - `vendor/linux/drivers/base/core.c:3810`.
#[unsafe(export_name = "put_device")]
pub unsafe extern "C" fn put_device(dev: *mut LinuxDevice) {
    if dev.is_null() {
        return;
    }

    unsafe {
        if (*dev).kobj.kref > 0 {
            (*dev).kobj.kref -= 1;
            if (*dev).kobj.kref == 0 {
                linux_device_release_at_zero(dev);
            }
        }
    }
}

/// `device_match_any` - `vendor/linux/drivers/base/core.c:5430`.
pub unsafe extern "C" fn linux_device_match_any(
    _dev: *mut LinuxDevice,
    _unused: *const c_void,
) -> i32 {
    1
}

/// `device_match_name` - `vendor/linux/drivers/base/core.c:5388`.
unsafe extern "C" fn linux_device_match_name(dev: *mut LinuxDevice, name: *const c_void) -> i32 {
    if dev.is_null() || name.is_null() {
        return 0;
    }

    unsafe { linux_sysfs_streq(linux_dev_name(&*dev), name.cast::<c_char>()) as i32 }
}

/// `device_match_acpi_handle` - `vendor/linux/drivers/base/core.c:5424`.
unsafe extern "C" fn linux_device_match_acpi_handle(
    _dev: *mut LinuxDevice,
    _handle: *const c_void,
) -> i32 {
    0
}

/// `dev_err_probe` - `vendor/linux/drivers/base/core.c:5145`.
unsafe extern "C" fn linux_dev_err_probe(
    _dev: *const LinuxDevice,
    err: i32,
    _fmt: *const c_char,
    _arg0: usize,
    _arg1: usize,
    _arg2: usize,
    _arg3: usize,
) -> i32 {
    err
}

/// `device_add` — `drivers/base/core.c:3573`.
///
/// Registers `dev` under its bus and class (if set), creates the sysfs
/// kobject, and dispatches to the driver model probe path.
pub fn device_add(dev: Arc<Device>) -> Result<(), i32> {
    {
        let mut s = dev.state.lock();
        if *s != DeviceState::Initialized && *s != DeviceState::Removed {
            return Err(EBUSY);
        }
        *s = DeviceState::Added;
    }

    if dev.name.is_empty() {
        return Err(EINVAL);
    }

    DEVICES.lock().insert(dev.name.clone(), dev.clone());
    let _ = kobject_add(dev.kobj.clone());

    if let Some(bus) = dev.bus.lock().clone() {
        bus.devices.lock().push(dev.clone());
        bus_probe_device(&bus, &dev);
    }
    if let Some(class) = dev.class.lock().clone() {
        class.devices.lock().push(dev.clone());
    }
    Ok(())
}

/// `device_register` — `drivers/base/core.c:3770`.
///
/// In Linux this is `device_initialize() + device_add()`.  Our `Device::new`
/// already initializes, so this is just `device_add`.
pub fn device_register(dev: Arc<Device>) -> Result<(), i32> {
    device_add(dev)
}

/// `device_del` — `drivers/base/core.c:3834`.
pub fn device_del(dev: &Arc<Device>) -> Result<(), i32> {
    {
        let s = *dev.state.lock();
        if s == DeviceState::Removed {
            return Ok(());
        }
    }

    // If bound, call driver->remove first.
    let bound_driver = { dev.driver.lock().clone() };
    if let Some(drv) = bound_driver {
        if let Some(remove) = drv.remove {
            remove(dev);
        }
        if let Some(bus) = { dev.bus.lock().clone() } {
            bus.devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
        }
        drv.bound_devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
        *dev.driver.lock() = None;
    }
    if let Some(class) = { dev.class.lock().clone() } {
        class.devices.lock().retain(|d| !Arc::ptr_eq(d, dev));
    }

    DEVICES.lock().remove(&dev.name);
    *dev.state.lock() = DeviceState::Removed;
    Ok(())
}

/// `device_unregister` — `drivers/base/core.c:3918`.
pub fn device_unregister(dev: &Arc<Device>) -> Result<(), i32> {
    device_del(dev)
}

/// Diagnostic — count of currently registered devices.
pub fn registered_devices() -> Vec<String> {
    DEVICES.lock().keys().cloned().collect()
}

/// Look up a registered device by name.  Used by the platform-bus tests.
pub fn find_device(name: &str) -> Option<Arc<Device>> {
    DEVICES.lock().get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_driver_abi::base::{
        LinuxDeviceDriver, linux_driver_register, linux_driver_unregister, register_linux_bus_type,
        unregister_linux_bus_type,
    };
    use core::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn linux_device_c_layout_prefix_matches_vendor_header() {
        use core::mem::{offset_of, size_of};

        assert_eq!(offset_of!(LinuxKObject, name), 0);
        assert_eq!(offset_of!(LinuxKObject, entry), 8);
        assert_eq!(offset_of!(LinuxKObject, parent), 24);
        assert_eq!(offset_of!(LinuxKObject, kset), 32);
        assert_eq!(offset_of!(LinuxKObject, ktype), 40);
        assert_eq!(offset_of!(LinuxKObject, sd), 48);
        assert_eq!(offset_of!(LinuxKObject, kref), 56);
        assert_eq!(offset_of!(LinuxKObject, state_flags), 60);
        assert_eq!(size_of::<LinuxKObject>(), 64);

        assert_eq!(offset_of!(LinuxDevice, kobj), 0);
        assert_eq!(offset_of!(LinuxDevice, parent), 64);
        assert_eq!(offset_of!(LinuxDevice, p), 72);
        assert_eq!(offset_of!(LinuxDevice, init_name), 80);
        assert_eq!(offset_of!(LinuxDevice, type_), 88);
        assert_eq!(offset_of!(LinuxDevice, bus), 96);
        assert_eq!(offset_of!(LinuxDevice, driver), 104);
        assert_eq!(offset_of!(LinuxDevice, platform_data), 112);
        assert_eq!(offset_of!(LinuxDevice, driver_data), 120);
        assert_eq!(offset_of!(LinuxDevice, _abi_tail), 128);
        assert_eq!(size_of::<LinuxDevice>(), 760);
        assert_eq!(LINUX_STRUCT_DEVICE_SIZE, 760);
        assert_eq!(LINUX_DEVICE_RELEASE_OFFSET, 712);
    }

    #[test]
    fn linux_device_core_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("device_initialize"),
            Some(linux_device_initialize as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_add"),
            Some(linux_device_add as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_attach"),
            Some(linux_device_attach as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_bind_driver"),
            Some(linux_device_bind_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_register"),
            Some(linux_device_register as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_release_driver"),
            Some(linux_device_release_driver as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_unregister"),
            Some(linux_device_unregister as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("get_device"),
            Some(get_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("put_device"),
            Some(put_device as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_match_any"),
            Some(linux_device_match_any as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_match_name"),
            Some(linux_device_match_name as usize)
        );
    }

    #[test]
    fn linux_device_core_exports_track_vendor_sources() {
        let core = include_str!("../../../vendor/linux/drivers/base/core.c");
        let dd = include_str!("../../../vendor/linux/drivers/base/dd.c");

        assert!(core.contains("EXPORT_SYMBOL_GPL(device_match_name);"));
        assert!(dd.contains("EXPORT_SYMBOL_GPL(device_bind_driver);"));

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("device_bind_driver"),
            Some(true)
        );
        assert_eq!(
            crate::kernel::module::find_symbol_gpl_only("device_match_name"),
            Some(true)
        );
    }

    #[test]
    fn linux_get_put_device_update_kobject_refcount() {
        unsafe {
            let mut backing = [0usize; 95];
            let dev = backing.as_mut_ptr().cast::<LinuxDevice>();
            assert!(get_device(core::ptr::null_mut()).is_null());
            put_device(core::ptr::null_mut());

            linux_device_initialize(dev);
            assert_eq!((*dev).kobj.kref, 1);
            assert_eq!(get_device(dev), dev);
            assert_eq!((*dev).kobj.kref, 2);
            put_device(dev);
            assert_eq!((*dev).kobj.kref, 1);
            put_device(dev);
            assert_eq!((*dev).kobj.kref, 0);
            put_device(dev);
            assert_eq!((*dev).kobj.kref, 0);
        }
    }

    static DEVICE_RELEASE_COUNT: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn test_device_release(_dev: *mut LinuxDevice) {
        DEVICE_RELEASE_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn linux_put_device_calls_release_at_zero() {
        unsafe {
            DEVICE_RELEASE_COUNT.store(0, Ordering::SeqCst);
            let mut backing = [0usize; 95];
            let bytes = backing.as_mut_ptr().cast::<u8>();
            let dev = bytes.cast::<LinuxDevice>();
            linux_device_initialize(dev);
            *bytes
                .add(LINUX_DEVICE_RELEASE_OFFSET)
                .cast::<Option<LinuxDeviceReleaseFn>>() = Some(test_device_release);

            put_device(dev);
            assert_eq!(DEVICE_RELEASE_COUNT.load(Ordering::SeqCst), 1);
            put_device(dev);
            assert_eq!(DEVICE_RELEASE_COUNT.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn linux_device_register_tracks_raw_device_and_dispatches_raw_probe() {
        static PROBE_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "C" fn always_match(_dev: *mut c_void, _drv: *const c_void) -> i32 {
            1
        }

        unsafe extern "C" fn probe(_dev: *mut c_void) -> i32 {
            PROBE_COUNT.fetch_add(1, Ordering::AcqRel);
            0
        }

        unsafe {
            assert_eq!(linux_device_add(core::ptr::null_mut()), -EINVAL);

            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"device-bus-test\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            bus.match_fn = Some(always_match);
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);
            register_linux_bus_type(bus_ptr);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"device-driver-test\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            driver.probe = Some(probe);
            assert_eq!(linux_driver_register(&mut driver), 0);

            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"device0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            dev.bus = bus_ptr;

            let before = registered_linux_device_count();
            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);
            assert_eq!(dev.driver, &mut driver as *mut _);
            assert_eq!(dev.kobj.name, dev_name.as_ptr().cast::<c_char>());
            assert!(dev.init_name.is_null());
            assert!(dev.kobj.state_flags & KOBJ_STATE_INITIALIZED != 0);
            assert!(dev.kobj.state_flags & KOBJ_STATE_IN_SYSFS != 0);
            assert_eq!(PROBE_COUNT.load(Ordering::Acquire), 1);
            assert_eq!(registered_linux_device_count(), before + 1);
            assert_eq!(linux_device_add(&mut dev), -EBUSY);

            linux_device_unregister(&mut dev);
            assert!(dev.p.is_null());
            assert!(dev.driver.is_null());
            assert!(!linux_device_registered(&dev));
            assert_eq!(registered_linux_device_count(), before);

            linux_driver_unregister(&mut driver);
            unregister_linux_bus_type(bus_ptr);
        }
    }

    #[test]
    fn linux_device_add_allows_probe_to_rename_device() {
        unsafe extern "C" fn always_match(_dev: *mut c_void, _drv: *const c_void) -> i32 {
            1
        }

        unsafe extern "C" fn probe_renames_device(dev: *mut c_void) -> i32 {
            unsafe {
                linux_device_set_name_bytes(dev.cast::<LinuxDevice>(), b"renamed0\0")
                    .map(|()| 0)
                    .unwrap_or_else(|errno| -errno)
            }
        }

        unsafe {
            let mut bus = core::mem::zeroed::<LinuxBusType>();
            let bus_name = b"device-rename-bus\0";
            bus.name = bus_name.as_ptr().cast::<c_char>();
            bus.match_fn = Some(always_match);
            let bus_ptr = &bus as *const LinuxBusType;
            unregister_linux_bus_type(bus_ptr);
            register_linux_bus_type(bus_ptr);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            let driver_name = b"device-rename-driver\0";
            driver.name = driver_name.as_ptr().cast::<c_char>();
            driver.bus = bus_ptr;
            driver.probe = Some(probe_renames_device);
            assert_eq!(linux_driver_register(&mut driver), 0);

            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"rename-source0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            dev.bus = bus_ptr;

            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);
            let name = core::slice::from_raw_parts(dev.kobj.name.cast::<u8>(), 9);
            assert_eq!(name, b"renamed0\0");

            linux_device_unregister(&mut dev);
            linux_driver_unregister(&mut driver);
            unregister_linux_bus_type(bus_ptr);
        }
    }

    #[test]
    fn linux_device_add_accepts_named_busless_devices() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"class-device0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();

            let before = registered_linux_device_count();
            assert_eq!(linux_device_register(&mut dev), 0);
            assert!(linux_device_registered(&dev));
            assert!(linux_device_driver(&dev).is_null());
            assert!(dev.driver.is_null());
            assert_eq!(dev.kobj.name, dev_name.as_ptr().cast::<c_char>());
            assert!(dev.init_name.is_null());
            assert_eq!(registered_linux_device_count(), before + 1);
            assert_eq!(linux_device_add(&mut dev), -EBUSY);

            linux_device_unregister(&mut dev);
            assert!(!linux_device_registered(&dev));
            assert_eq!(registered_linux_device_count(), before);
        }
    }

    #[test]
    fn linux_device_bind_driver_records_preselected_driver() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"bind-driver0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();
            assert_eq!(linux_device_register(&mut dev), 0);

            let mut driver = core::mem::zeroed::<LinuxDeviceDriver>();
            dev.driver = &mut driver;

            assert_eq!(linux_device_bind_driver(core::ptr::null_mut()), -EINVAL);
            assert_eq!(linux_device_bind_driver(&mut dev), 0);
            assert_eq!(linux_device_driver(&dev), &mut driver as *mut _);

            linux_device_unregister(&mut dev);
        }
    }

    #[test]
    fn linux_device_match_name_uses_sysfs_string_rules() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();
            let dev_name = b"phy0\0";
            dev.init_name = dev_name.as_ptr().cast::<c_char>();

            assert_eq!(
                linux_device_match_name(&mut dev, b"phy0\0".as_ptr().cast()),
                1
            );
            assert_eq!(
                linux_device_match_name(&mut dev, b"phy0\n\0".as_ptr().cast()),
                1
            );
            assert_eq!(
                linux_device_match_name(&mut dev, b"phy1\0".as_ptr().cast()),
                0
            );
            assert_eq!(linux_device_match_name(&mut dev, core::ptr::null()), 0);
        }
    }

    #[test]
    fn linux_device_set_name_bytes_sets_exact_name() {
        unsafe {
            let mut dev = core::mem::zeroed::<LinuxDevice>();

            linux_device_initialize(&mut dev);
            assert_eq!(linux_device_set_name_bytes(&mut dev, b"host7\0"), Ok(()));
            assert_eq!(dev.kobj.name, linux_dev_name(&dev));
            let name = core::slice::from_raw_parts(dev.kobj.name.cast::<u8>(), 6);
            assert_eq!(name, b"host7\0");
            assert!(dev.init_name.is_null());

            free_linux_device_private(&mut dev);
        }
    }
}
