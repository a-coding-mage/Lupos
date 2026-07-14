//! linux-parity: partial
//! linux-source: vendor/linux/drivers/input
//! linux-source: vendor/linux/drivers/input/input.c
//! test-origin: linux:vendor/linux/drivers/input
//! Input subsystem — M58.
//!
//! Mirrors `drivers/input/input.c` + `include/linux/input.h`: `input_dev`,
//! `input_handler`, device registration, and an evdev char-device backend.
//! Remaining work vs Linux for `complete`: full event routing/filtering, the
//! i8042 aux (mouse) channel wiring, and the broader handler set (the current
//! event1 mouse node is a placeholder).
//!
//! Mirrors `drivers/input/input.c` and `include/linux/input.h`.
//! Provides `struct input_dev`, `struct input_handler`, the device
//! registration API, and the evdev character device backend.
//!
//! References:
//!   - `include/linux/input.h:137`    — `struct input_dev`
//!   - `include/linux/input.h:315`    — `struct input_handler`
//!   - `drivers/input/input.c:2312`   — `input_register_device`
//!   - `drivers/input/input.c:2452`   — `input_register_handler`
//!   - `drivers/input/evdev.c`        — evdev handler

extern crate alloc;

pub mod evdev_chardev;
pub mod i8042;
pub mod linux_sources;
pub mod misc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, EINVAL, ENOMEM};
use crate::kernel::sched::wait::WaitQueueHead;
use crate::linux_driver_abi::base::{LinuxDevice, linux_device_set_name_index};
use crate::mm::page_flags::GFP_KERNEL;

// ── input_event ABI — `include/uapi/linux/input.h` ───────────────────────────
// MUST match Linux exactly (used by evdev readers).

/// `struct input_event` — `include/uapi/linux/input.h`.
/// Packed to match the Linux wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct InputEvent {
    /// Seconds component of the event timestamp.
    pub sec: u64,
    /// Microseconds component.
    pub usec: u64,
    /// Event type (EV_KEY, EV_REL, EV_ABS, …).
    pub event_type: u16,
    /// Event code (key code, axis index, …).
    pub code: u16,
    /// Event value (key down=1, up=0, repeat=2; axis delta).
    pub value: i32,
}

// Event type codes — `include/uapi/linux/input-event-codes.h`.
pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;
pub const EV_SW: u16 = 0x05;
pub const EV_SND: u16 = 0x12;
pub const EV_REP: u16 = 0x14;

// Key codes (subset).
pub const KEY_A: u16 = 30;
pub const KEY_ENTER: u16 = 28;
pub const KEY_ESC: u16 = 1;

// Relative axes — `include/uapi/linux/input-event-codes.h`.
pub const REL_X: u16 = 0x00;
pub const REL_Y: u16 = 0x01;
pub const REL_WHEEL: u16 = 0x08;

// Mouse button codes — `include/uapi/linux/input-event-codes.h`.
pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;

// ── raw Linux C `struct input_dev` ABI ─────────────────────────────────────

const LINUX_INPUT_DEV_SIZE: usize = 1384;
const LINUX_INPUT_DEV_ALIGN: usize = 8;
const LINUX_INPUT_VALUE_SIZE: usize = 8;
const LINUX_INPUT_ABSINFO_SIZE: usize = 24;
const LINUX_INPUT_DEV_PROPBITS_OFFSET: usize = 32;
const LINUX_INPUT_DEV_EVBITS_OFFSET: usize = 40;
const LINUX_INPUT_DEV_KEYBITS_OFFSET: usize = 48;
const LINUX_INPUT_DEV_RELBITS_OFFSET: usize = 144;
const LINUX_INPUT_DEV_ABSBITS_OFFSET: usize = 152;
const LINUX_INPUT_DEV_MSCBITS_OFFSET: usize = 160;
const LINUX_INPUT_DEV_LEDBITS_OFFSET: usize = 168;
const LINUX_INPUT_DEV_SNDBITS_OFFSET: usize = 176;
const LINUX_INPUT_DEV_FFBITS_OFFSET: usize = 184;
const LINUX_INPUT_DEV_SWBITS_OFFSET: usize = 200;
const LINUX_INPUT_DEV_MT_OFFSET: usize = 320;
const LINUX_INPUT_DEV_ABSINFO_OFFSET: usize = 328;
const LINUX_INPUT_DEV_KEY_OFFSET: usize = 336;
const LINUX_INPUT_DEV_LED_OFFSET: usize = 432;
const LINUX_INPUT_DEV_SND_OFFSET: usize = 440;
const LINUX_INPUT_DEV_SW_OFFSET: usize = 448;
const LINUX_INPUT_DEV_EVENT_OFFSET: usize = 480;
const LINUX_INPUT_DEV_TIMER_OFFSET: usize = 272;
const LINUX_INPUT_DEV_DEV_OFFSET: usize = 536;
const LINUX_INPUT_DEV_H_LIST_OFFSET: usize = 1296;
const LINUX_INPUT_DEV_NODE_OFFSET: usize = 1312;
const LINUX_INPUT_DEV_NUM_VALS_OFFSET: usize = 1328;
const LINUX_INPUT_DEV_MAX_VALS_OFFSET: usize = 1332;
const LINUX_INPUT_DEV_VALS_OFFSET: usize = 1336;
const LINUX_INPUT_DEV_DEVRES_MANAGED_OFFSET: usize = 1344;

const LINUX_TIMER_LIST_SIZE: usize = 40;
const INPUT_DEFAULT_MAX_VALS: u32 = 10;

const SYN_REPORT: u32 = 0;
const EV_MAX: u32 = 0x1f;
const KEY_RESERVED_RAW: u32 = 0;
const ABS_CNT: usize = 0x40;
const ABS_X: u32 = 0x00;
const ABS_Y: u32 = 0x01;
const ABS_PRESSURE: u32 = 0x18;
const ABS_MT_SLOT: u32 = 0x2f;
const ABS_MT_FIRST: u32 = 0x30;
const ABS_MT_POSITION_X: u32 = 0x35;
const ABS_MT_POSITION_Y: u32 = 0x36;
const ABS_MT_TRACKING_ID: u32 = 0x39;
const ABS_MT_PRESSURE: u32 = 0x3a;
const ABS_MT_LAST: u32 = 0x3d;
const ABS_MT_AXIS_COUNT: usize = (ABS_MT_LAST - ABS_MT_FIRST + 1) as usize;
const ABS_MT_TRACKING_ID_INDEX: usize = (ABS_MT_TRACKING_ID - ABS_MT_FIRST) as usize;
const TRKID_MAX: i32 = 0xffff;
const INPUT_MT_POINTER: u32 = 0x0001;
const INPUT_MT_DIRECT: u32 = 0x0002;
const INPUT_MT_TRACK: u32 = 0x0008;
const INPUT_MT_SEMI_MT: u32 = 0x0010;
const INPUT_PROP_POINTER: u32 = 0x00;
const INPUT_PROP_DIRECT: u32 = 0x01;
const INPUT_PROP_SEMI_MT: u32 = 0x03;
const BTN_TOOL_FINGER: u32 = 0x145;
const BTN_TOOL_QUINTTAP: u32 = 0x148;
const BTN_TOUCH: u32 = 0x14a;
const BTN_TOOL_DOUBLETAP: u32 = 0x14d;
const BTN_TOOL_TRIPLETAP: u32 = 0x14e;
const BTN_TOOL_QUADTAP: u32 = 0x14f;

#[repr(C)]
struct LinuxListHead {
    next: usize,
    prev: usize,
}

#[repr(C)]
struct LinuxTimerList {
    entry_next: usize,
    entry_prev: usize,
    expires: u64,
    function: usize,
    flags: u32,
    _pad_after_flags: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct LinuxInputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[repr(C)]
struct LinuxInputMt {
    trkid: i32,
    num_slots: i32,
    slot: i32,
    flags: u32,
    frame: u32,
    _pad_after_frame: u32,
    red: *mut i32,
}

#[repr(C)]
struct LinuxInputMtSlot {
    abs: [i32; ABS_MT_AXIS_COUNT],
    frame: u32,
    key: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RawInputState {
    Allocated,
    Registered,
}

lazy_static! {
    static ref RAW_INPUT_DEVS: Mutex<BTreeMap<usize, RawInputState>> = Mutex::new(BTreeMap::new());
}

static RAW_INPUT_NO: AtomicU32 = AtomicU32::new(0);

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if crate::kernel::module::find_symbol(name).is_none() {
        crate::kernel::module::export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "input_allocate_device",
        linux_input_allocate_device as usize,
        false,
    );
    export_symbol_once("input_free_device", linux_input_free_device as usize, false);
    export_symbol_once(
        "input_register_device",
        linux_input_register_device as usize,
        false,
    );
    export_symbol_once(
        "input_unregister_device",
        linux_input_unregister_device as usize,
        false,
    );
    export_symbol_once(
        "input_set_capability",
        linux_input_set_capability as usize,
        false,
    );
    export_symbol_once(
        "input_alloc_absinfo",
        linux_input_alloc_absinfo as usize,
        false,
    );
    export_symbol_once(
        "input_set_abs_params",
        linux_input_set_abs_params as usize,
        false,
    );
    export_symbol_once(
        "input_mt_init_slots",
        linux_input_mt_init_slots as usize,
        false,
    );
    export_symbol_once("input_event", linux_input_event as usize, false);
}

fn raw_input_dev(ptr: *mut c_void) -> *mut u8 {
    ptr.cast::<u8>()
}

unsafe fn field_ptr<T>(dev: *mut u8, offset: usize) -> *mut T {
    unsafe { dev.add(offset).cast::<T>() }
}

unsafe fn embedded_device(dev: *mut u8) -> *mut LinuxDevice {
    unsafe { field_ptr(dev, LINUX_INPUT_DEV_DEV_OFFSET) }
}

unsafe fn init_list_head(dev: *mut u8, offset: usize) {
    let head = unsafe { field_ptr::<LinuxListHead>(dev, offset) };
    unsafe {
        (*head).next = head as usize;
        (*head).prev = head as usize;
    }
}

unsafe fn init_timer_list(dev: *mut u8) {
    let timer = unsafe { field_ptr::<LinuxTimerList>(dev, LINUX_INPUT_DEV_TIMER_OFFSET) };
    unsafe {
        (*timer).entry_next = 0;
        (*timer).entry_prev = 0;
        (*timer).expires = 0;
        (*timer).function = 0;
        (*timer).flags = 0;
        (*timer)._pad_after_flags = 0;
    }
}

unsafe fn read_u32(dev: *mut u8, offset: usize) -> u32 {
    unsafe { field_ptr::<u32>(dev, offset).read() }
}

unsafe fn write_u32(dev: *mut u8, offset: usize, value: u32) {
    unsafe { field_ptr::<u32>(dev, offset).write(value) };
}

unsafe fn read_usize(dev: *mut u8, offset: usize) -> usize {
    unsafe { field_ptr::<usize>(dev, offset).read() }
}

unsafe fn write_usize(dev: *mut u8, offset: usize, value: usize) {
    unsafe { field_ptr::<usize>(dev, offset).write(value) };
}

const fn bit_word(bit: u32) -> usize {
    bit as usize / usize::BITS as usize
}

const fn bit_mask(bit: u32) -> usize {
    1usize << (bit as usize % usize::BITS as usize)
}

unsafe fn set_bit(dev: *mut u8, offset: usize, bit: u32) {
    let word =
        unsafe { field_ptr::<usize>(dev, offset + bit_word(bit) * core::mem::size_of::<usize>()) };
    unsafe { word.write(word.read() | bit_mask(bit)) };
}

unsafe fn clear_bit(dev: *mut u8, offset: usize, bit: u32) {
    let word =
        unsafe { field_ptr::<usize>(dev, offset + bit_word(bit) * core::mem::size_of::<usize>()) };
    unsafe { word.write(word.read() & !bit_mask(bit)) };
}

unsafe fn test_bit(dev: *mut u8, offset: usize, bit: u32) -> bool {
    let word =
        unsafe { field_ptr::<usize>(dev, offset + bit_word(bit) * core::mem::size_of::<usize>()) };
    unsafe { word.read() & bit_mask(bit) != 0 }
}

unsafe fn absinfo_ptr(dev: *mut u8) -> *mut LinuxInputAbsInfo {
    unsafe { read_usize(dev, LINUX_INPUT_DEV_ABSINFO_OFFSET) as *mut LinuxInputAbsInfo }
}

unsafe fn mt_ptr(dev: *mut u8) -> *mut LinuxInputMt {
    unsafe { read_usize(dev, LINUX_INPUT_DEV_MT_OFFSET) as *mut LinuxInputMt }
}

unsafe fn free_input_mt(mt: *mut LinuxInputMt) {
    if mt.is_null() {
        return;
    }
    let red = unsafe { (*mt).red };
    if !red.is_null() {
        unsafe { crate::mm::slab::linux_kfree(red.cast::<u8>()) };
    }
    unsafe { crate::mm::slab::linux_kfree(mt.cast::<u8>()) };
}

unsafe fn copy_absinfo(dev: *mut u8, dst: u32, src: u32) {
    if dst as usize >= ABS_CNT || src as usize >= ABS_CNT {
        return;
    }
    if unsafe { !test_bit(dev, LINUX_INPUT_DEV_ABSBITS_OFFSET, src) } {
        return;
    }
    let absinfo = unsafe { absinfo_ptr(dev) };
    if absinfo.is_null() {
        return;
    }
    unsafe {
        let mut info = absinfo.add(src as usize).read();
        info.fuzz = 0;
        absinfo.add(dst as usize).write(info);
        set_bit(dev, LINUX_INPUT_DEV_ABSBITS_OFFSET, dst);
    }
}

const fn capability_offset(event_type: u32) -> Option<usize> {
    match event_type {
        0x01 => Some(LINUX_INPUT_DEV_KEYBITS_OFFSET),
        0x02 => Some(LINUX_INPUT_DEV_RELBITS_OFFSET),
        0x03 => Some(LINUX_INPUT_DEV_ABSBITS_OFFSET),
        0x04 => Some(LINUX_INPUT_DEV_MSCBITS_OFFSET),
        0x05 => Some(LINUX_INPUT_DEV_SWBITS_OFFSET),
        0x11 => Some(LINUX_INPUT_DEV_LEDBITS_OFFSET),
        0x12 => Some(LINUX_INPUT_DEV_SNDBITS_OFFSET),
        0x15 => Some(LINUX_INPUT_DEV_FFBITS_OFFSET),
        _ => None,
    }
}

const fn state_offset(event_type: u32) -> Option<usize> {
    match event_type {
        0x01 => Some(LINUX_INPUT_DEV_KEY_OFFSET),
        0x05 => Some(LINUX_INPUT_DEV_SW_OFFSET),
        0x11 => Some(LINUX_INPUT_DEV_LED_OFFSET),
        0x12 => Some(LINUX_INPUT_DEV_SND_OFFSET),
        _ => None,
    }
}

unsafe fn free_raw_input_device(dev: *mut u8) {
    if dev.is_null() {
        return;
    }
    let mt = unsafe { mt_ptr(dev) };
    if !mt.is_null() {
        unsafe { free_input_mt(mt) };
        unsafe { write_usize(dev, LINUX_INPUT_DEV_MT_OFFSET, 0) };
    }
    let absinfo = unsafe { absinfo_ptr(dev) };
    if !absinfo.is_null() {
        unsafe { crate::mm::slab::linux_kfree(absinfo.cast::<u8>()) };
        unsafe { write_usize(dev, LINUX_INPUT_DEV_ABSINFO_OFFSET, 0) };
    }
    let vals = unsafe { read_usize(dev, LINUX_INPUT_DEV_VALS_OFFSET) as *mut u8 };
    if !vals.is_null() {
        unsafe { crate::mm::slab::linux_kfree(vals) };
    }
    unsafe { crate::linux_driver_abi::base::put_device(embedded_device(dev)) };
    unsafe { crate::mm::slab::linux_kfree(dev) };
}

unsafe extern "C" fn linux_input_allocate_device() -> *mut c_void {
    let dev = unsafe { crate::mm::slab::linux___kmalloc_noprof(LINUX_INPUT_DEV_SIZE, GFP_KERNEL) };
    if dev.is_null() {
        return core::ptr::null_mut();
    }
    if (dev as usize) % LINUX_INPUT_DEV_ALIGN != 0 {
        unsafe { crate::mm::slab::linux_kfree(dev) };
        return core::ptr::null_mut();
    }
    unsafe { core::ptr::write_bytes(dev, 0, LINUX_INPUT_DEV_SIZE) };

    let vals_len = INPUT_DEFAULT_MAX_VALS as usize * LINUX_INPUT_VALUE_SIZE;
    let vals = unsafe { crate::mm::slab::linux___kmalloc_noprof(vals_len, GFP_KERNEL) };
    if vals.is_null() {
        unsafe { crate::mm::slab::linux_kfree(dev) };
        return core::ptr::null_mut();
    }
    unsafe { core::ptr::write_bytes(vals, 0, vals_len) };

    unsafe {
        init_list_head(dev, LINUX_INPUT_DEV_H_LIST_OFFSET);
        init_list_head(dev, LINUX_INPUT_DEV_NODE_OFFSET);
        init_timer_list(dev);
        write_u32(dev, LINUX_INPUT_DEV_MAX_VALS_OFFSET, INPUT_DEFAULT_MAX_VALS);
        write_usize(dev, LINUX_INPUT_DEV_VALS_OFFSET, vals as usize);
        let linux_dev = embedded_device(dev);
        crate::linux_driver_abi::base::linux_device_initialize(linux_dev);
        let index = RAW_INPUT_NO.fetch_add(1, Ordering::AcqRel) as i32;
        let _ = linux_device_set_name_index(linux_dev, b"input", index);
    }

    RAW_INPUT_DEVS
        .lock()
        .insert(dev as usize, RawInputState::Allocated);
    dev.cast()
}

unsafe extern "C" fn linux_input_free_device(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let dev = raw_input_dev(dev);
    let mut registry = RAW_INPUT_DEVS.lock();
    match registry.get(&(dev as usize)).copied() {
        Some(RawInputState::Registered) => return,
        Some(RawInputState::Allocated) => {
            registry.remove(&(dev as usize));
        }
        None => return,
    }
    drop(registry);
    unsafe { free_raw_input_device(dev) };
}

unsafe extern "C" fn linux_input_register_device(dev: *mut c_void) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let dev = raw_input_dev(dev);
    let mut registry = RAW_INPUT_DEVS.lock();
    if registry.get(&(dev as usize)) != Some(&RawInputState::Allocated) {
        return -EINVAL;
    }

    unsafe {
        set_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, EV_SYN as u32);
        clear_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, KEY_RESERVED_RAW);
        if test_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, EV_ABS as u32) && absinfo_ptr(dev).is_null()
        {
            return -EINVAL;
        }
        if read_u32(dev, LINUX_INPUT_DEV_MAX_VALS_OFFSET) == 0
            || read_usize(dev, LINUX_INPUT_DEV_VALS_OFFSET) == 0
        {
            return -ENOMEM;
        }
    }

    let rc = unsafe { crate::linux_driver_abi::base::linux_device_add(embedded_device(dev)) };
    if rc != 0 {
        return rc;
    }
    registry.insert(dev as usize, RawInputState::Registered);
    0
}

unsafe extern "C" fn linux_input_unregister_device(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let dev = raw_input_dev(dev);
    let mut registry = RAW_INPUT_DEVS.lock();
    if registry.remove(&(dev as usize)) != Some(RawInputState::Registered) {
        return;
    }
    drop(registry);
    unsafe {
        crate::linux_driver_abi::base::linux_device_unregister(embedded_device(dev));
        free_raw_input_device(dev);
    }
}

unsafe extern "C" fn linux_input_set_capability(dev: *mut c_void, event_type: u32, code: u32) {
    if dev.is_null() || event_type > EV_MAX {
        return;
    }
    let dev = raw_input_dev(dev);
    if event_type == EV_ABS as u32 {
        unsafe { linux_input_alloc_absinfo(dev.cast()) };
    }
    if let Some(offset) = capability_offset(event_type) {
        unsafe { set_bit(dev, offset, code) };
    }
    unsafe { set_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, event_type) };
}

unsafe extern "C" fn linux_input_alloc_absinfo(dev: *mut c_void) {
    if dev.is_null() {
        return;
    }
    let dev = raw_input_dev(dev);
    if unsafe { !absinfo_ptr(dev).is_null() } {
        return;
    }
    let len = ABS_CNT * LINUX_INPUT_ABSINFO_SIZE;
    let absinfo = unsafe { crate::mm::slab::linux___kmalloc_noprof(len, GFP_KERNEL) };
    if absinfo.is_null() {
        return;
    }
    unsafe {
        core::ptr::write_bytes(absinfo, 0, len);
        write_usize(dev, LINUX_INPUT_DEV_ABSINFO_OFFSET, absinfo as usize);
    }
}

unsafe extern "C" fn linux_input_set_abs_params(
    dev: *mut c_void,
    axis: u32,
    min: i32,
    max: i32,
    fuzz: i32,
    flat: i32,
) {
    if dev.is_null() || axis as usize >= ABS_CNT {
        return;
    }
    let dev = raw_input_dev(dev);
    unsafe {
        set_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, EV_ABS as u32);
        set_bit(dev, LINUX_INPUT_DEV_ABSBITS_OFFSET, axis);
        linux_input_alloc_absinfo(dev.cast());
        let absinfo = absinfo_ptr(dev);
        if absinfo.is_null() {
            return;
        }
        let absinfo = absinfo.add(axis as usize);
        (*absinfo).minimum = min;
        (*absinfo).maximum = max;
        (*absinfo).fuzz = fuzz;
        (*absinfo).flat = flat;
    }
}

unsafe extern "C" fn linux_input_mt_init_slots(
    dev: *mut c_void,
    num_slots: u32,
    flags: u32,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    if num_slots == 0 {
        return 0;
    }
    let dev = raw_input_dev(dev);
    let mt = unsafe { mt_ptr(dev) };
    if !mt.is_null() {
        return if unsafe { (*mt).num_slots } == num_slots as i32 {
            0
        } else {
            -EINVAL
        };
    }
    if num_slots > 1024 {
        return -EINVAL;
    }

    let slots_len = num_slots as usize * core::mem::size_of::<LinuxInputMtSlot>();
    let mt_len = core::mem::size_of::<LinuxInputMt>() + slots_len;
    let mt = unsafe { crate::mm::slab::linux___kmalloc_noprof(mt_len, GFP_KERNEL) };
    if mt.is_null() {
        return -ENOMEM;
    }
    unsafe { core::ptr::write_bytes(mt, 0, mt_len) };
    let mt = mt.cast::<LinuxInputMt>();

    let mut red = core::ptr::null_mut::<i32>();
    if flags & INPUT_MT_TRACK != 0 {
        let red_len = num_slots as usize * num_slots as usize * core::mem::size_of::<i32>();
        red = unsafe { crate::mm::slab::linux___kmalloc_noprof(red_len, GFP_KERNEL).cast::<i32>() };
        if red.is_null() {
            unsafe { crate::mm::slab::linux_kfree(mt.cast::<u8>()) };
            return -ENOMEM;
        }
        unsafe { core::ptr::write_bytes(red.cast::<u8>(), 0, red_len) };
    }

    unsafe {
        (*mt).num_slots = num_slots as i32;
        (*mt).flags = flags;
        (*mt).frame = 1;
        (*mt).red = red;
        let slots = mt.cast::<u8>().add(core::mem::size_of::<LinuxInputMt>());
        for slot_index in 0..num_slots as usize {
            let slot = slots
                .add(slot_index * core::mem::size_of::<LinuxInputMtSlot>())
                .cast::<LinuxInputMtSlot>();
            (*slot).abs[ABS_MT_TRACKING_ID_INDEX] = -1;
        }

        linux_input_set_abs_params(dev.cast(), ABS_MT_SLOT, 0, num_slots as i32 - 1, 0, 0);
        linux_input_set_abs_params(dev.cast(), ABS_MT_TRACKING_ID, 0, TRKID_MAX, 0, 0);

        if flags & (INPUT_MT_POINTER | INPUT_MT_DIRECT) != 0 {
            set_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, EV_KEY as u32);
            set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOUCH);
            copy_absinfo(dev, ABS_X, ABS_MT_POSITION_X);
            copy_absinfo(dev, ABS_Y, ABS_MT_POSITION_Y);
            copy_absinfo(dev, ABS_PRESSURE, ABS_MT_PRESSURE);
        }
        if flags & INPUT_MT_POINTER != 0 {
            set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOOL_FINGER);
            set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOOL_DOUBLETAP);
            if num_slots >= 3 {
                set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOOL_TRIPLETAP);
            }
            if num_slots >= 4 {
                set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOOL_QUADTAP);
            }
            if num_slots >= 5 {
                set_bit(dev, LINUX_INPUT_DEV_KEYBITS_OFFSET, BTN_TOOL_QUINTTAP);
            }
            set_bit(dev, LINUX_INPUT_DEV_PROPBITS_OFFSET, INPUT_PROP_POINTER);
        }
        if flags & INPUT_MT_DIRECT != 0 {
            set_bit(dev, LINUX_INPUT_DEV_PROPBITS_OFFSET, INPUT_PROP_DIRECT);
        }
        if flags & INPUT_MT_SEMI_MT != 0 {
            set_bit(dev, LINUX_INPUT_DEV_PROPBITS_OFFSET, INPUT_PROP_SEMI_MT);
        }
        write_usize(dev, LINUX_INPUT_DEV_MT_OFFSET, mt as usize);
    }
    0
}

unsafe extern "C" fn linux_input_event(dev: *mut c_void, event_type: u32, code: u32, value: i32) {
    if dev.is_null() {
        return;
    }
    let dev = raw_input_dev(dev);
    if event_type != EV_SYN as u32
        && unsafe { !test_bit(dev, LINUX_INPUT_DEV_EVBITS_OFFSET, event_type) }
    {
        return;
    }
    if let Some(offset) = state_offset(event_type) {
        if value != 0 {
            unsafe { set_bit(dev, offset, code) };
        } else {
            unsafe { clear_bit(dev, offset, code) };
        }
    }

    type InputEventCallback =
        unsafe extern "C" fn(*mut c_void, event_type: u32, code: u32, value: i32) -> i32;
    if event_type == EV_SND as u32 || event_type == EV_REP as u32 {
        let callback = unsafe { read_usize(dev, LINUX_INPUT_DEV_EVENT_OFFSET) };
        if callback != 0 {
            let callback: InputEventCallback = unsafe { core::mem::transmute(callback) };
            let _ = unsafe { callback(dev.cast(), event_type, code, value) };
        }
    }
    let _ = SYN_REPORT;
}

// ── input_dev ─────────────────────────────────────────────────────────────────

/// `struct input_dev` — `include/linux/input.h:137`.
pub struct InputDev {
    pub name: String,
    pub id: u32,
    /// Event queue consumed by evdev readers.
    pub events: Mutex<Vec<InputEvent>>,
    /// Readers and poll/epoll callbacks waiting for an evdev packet.
    ///
    /// Linux keeps this waitqueue in each `struct evdev_client`.  Lupos does
    /// not yet materialize per-open evdev clients, so the current single
    /// device queue and its waitqueue have the same lifetime and ownership.
    pub(crate) event_wait: WaitQueueHead,
    /// Handlers attached to this device.
    pub handlers: Mutex<Vec<Arc<InputHandler>>>,
}

impl InputDev {
    pub fn new(name: &str, id: u32) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            id,
            events: Mutex::new(Vec::new()),
            event_wait: WaitQueueHead::new(),
            handlers: Mutex::new(Vec::new()),
        })
    }

    /// `input_event` — inject one event into this device.
    ///
    /// Pushes to the device's event queue and notifies all handlers.
    pub fn input_event(&self, event_type: u16, code: u16, value: i32) {
        let ev = InputEvent {
            sec: 0,
            usec: 0,
            event_type,
            code,
            value,
        };
        self.events.lock().push(ev);
        // `evdev_pass_values()` publishes a completed packet before waking
        // readers and poll callbacks.  The in-tree producers terminate each
        // keyboard/mouse packet with EV_SYN/SYN_REPORT (code zero).
        if event_type == EV_SYN && code == 0 {
            self.event_wait.wake_up_all();
        }
        let handlers: Vec<Arc<InputHandler>> = self.handlers.lock().iter().cloned().collect();
        for h in handlers.iter() {
            (h.event)(self, &ev);
        }
    }

    pub fn drain_events(&self) -> Vec<InputEvent> {
        self.events.lock().drain(..).collect()
    }
}

// ── input_handler ─────────────────────────────────────────────────────────────

pub type InputEventFn = fn(dev: &InputDev, event: &InputEvent);

/// `struct input_handler` — `include/linux/input.h:315`.
pub struct InputHandler {
    pub name: &'static str,
    pub event: InputEventFn,
}

// ── Registries ────────────────────────────────────────────────────────────────

lazy_static! {
    static ref INPUT_DEVICES: Mutex<BTreeMap<u32, Arc<InputDev>>> = Mutex::new(BTreeMap::new());
    static ref INPUT_HANDLERS: Mutex<Vec<Arc<InputHandler>>> = Mutex::new(Vec::new());
}

/// `input_register_device` — `drivers/input/input.c:2312`.
pub fn input_register_device(dev: Arc<InputDev>) -> Result<(), i32> {
    let mut g = INPUT_DEVICES.lock();
    if g.contains_key(&dev.id) {
        return Err(EEXIST);
    }
    // Attach all registered handlers to the new device.
    let handlers: Vec<Arc<InputHandler>> = INPUT_HANDLERS.lock().iter().cloned().collect();
    dev.handlers.lock().extend(handlers);
    g.insert(dev.id, dev);
    Ok(())
}

/// `input_register_handler` — `drivers/input/input.c:2452`.
pub fn input_register_handler(h: Arc<InputHandler>) {
    INPUT_HANDLERS.lock().push(h);
}

pub fn input_device_count() -> usize {
    INPUT_DEVICES.lock().len()
}

pub fn find_input_dev(id: u32) -> Option<Arc<InputDev>> {
    INPUT_DEVICES.lock().get(&id).cloned()
}

/// Register the standard keyboard + mouse evdev devices so userspace can open
/// `/dev/input/event0` (keyboard) and `/dev/input/event1` (mouse).
///
/// Idempotent — repeated calls are silently ignored.
pub fn register_default_evdev_devices() {
    use evdev_chardev::{InputId, register_evdev_device};

    // event0 — i8042 PS/2 keyboard.
    if find_input_dev(0xE001).is_none() {
        let kbd = InputDev::new("AT Translated Set 2 keyboard", 0xE001);
        let _ = input_register_device(kbd.clone());
        // `bustype` 0x11 = `BUS_I8042` — `include/uapi/linux/input.h:262`.
        register_evdev_device(
            0,
            kbd,
            "AT Translated Set 2 keyboard",
            InputId {
                bustype: 0x11,
                vendor: 0x0001,
                product: 0x0001,
                version: 0xab41,
            },
        );
    }

    // event1 — generic mouse placeholder.  The i8042 aux channel isn't wired
    // yet; the node still lets libinput enumerate a pointing device.
    if find_input_dev(0xE002).is_none() {
        let mouse = InputDev::new("ImExPS/2 Generic Explorer Mouse", 0xE002);
        let _ = input_register_device(mouse.clone());
        register_evdev_device(
            1,
            mouse,
            "ImExPS/2 Generic Explorer Mouse",
            InputId {
                bustype: 0x11,
                vendor: 0x0002,
                product: 0x0006,
                version: 0x0000,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_inject_event() {
        let dev = InputDev::new("test-kbd", 0xA001);
        input_register_device(dev.clone()).unwrap();
        dev.input_event(EV_KEY, KEY_A, 1);
        let evs = dev.drain_events();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].event_type, EV_KEY);
        assert_eq!(evs[0].code, KEY_A);
        assert_eq!(evs[0].value, 1);
    }

    #[test]
    fn handler_receives_event() {
        use core::sync::atomic::{AtomicU16, Ordering};
        static LAST_CODE: AtomicU16 = AtomicU16::new(0);
        fn my_handler(_: &InputDev, ev: &InputEvent) {
            LAST_CODE.store(ev.code, Ordering::Release);
        }
        let h = Arc::new(InputHandler {
            name: "test-handler",
            event: my_handler,
        });
        input_register_handler(h);
        let dev = InputDev::new("test-kbd-2", 0xA002);
        input_register_device(dev.clone()).unwrap();
        dev.input_event(EV_KEY, KEY_ENTER, 1);
        assert_eq!(LAST_CODE.load(Ordering::Acquire), KEY_ENTER);
    }

    #[test]
    fn module_exports_track_vendor_input_symbols() {
        let input_source = include_str!("../../../vendor/linux/drivers/input/input.c");
        let mt_source = include_str!("../../../vendor/linux/drivers/input/input-mt.c");
        assert!(input_source.contains("EXPORT_SYMBOL(input_alloc_absinfo);"));
        assert!(input_source.contains("EXPORT_SYMBOL(input_set_abs_params);"));
        assert!(mt_source.contains("EXPORT_SYMBOL(input_mt_init_slots);"));
    }
}
