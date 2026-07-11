//! linux-parity: complete
//! linux-source: vendor/linux/drivers/input
//! test-origin: linux:vendor/linux/drivers/input
//! Evdev character device — `/dev/input/eventN`.
//!
//! Exposes the existing [`InputDev`] registry as Linux-style evdev character
//! devices so userspace consumers (libinput, X.Org, Weston) can open
//! `/dev/input/event0`, poll for readability, read `struct input_event`
//! records, and issue the canonical evdev ioctls.
//!
//! References:
//!   - `vendor/linux/drivers/input/evdev.c`               — the upstream handler
//!   - `vendor/linux/include/uapi/linux/input.h`          — ioctls + input_id
//!   - `vendor/linux/include/uapi/linux/input-event-codes.h` — event/key codes

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use core::mem::size_of;
use lazy_static::lazy_static;
use spin::Mutex;

use super::{EV_ABS, EV_KEY, EV_REL, EV_SYN, InputDev, InputEvent};
use crate::fs::ops::{FileOps, IoctlFn, PollFn};
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EFAULT, EINVAL, ENODEV, ENOTTY};

// ── Evdev ABI constants — `include/uapi/linux/input.h` ────────────────────────

/// The version returned by `EVIOCGVERSION`. Linux ships `0x010001`.
pub const EV_VERSION: u32 = 0x0001_0001;

// Direction / type / size encoding for the legacy `_IOC` macro family.
const IOC_DIRSHIFT: u32 = 30;
const IOC_TYPESHIFT: u32 = 8;
const IOC_NRSHIFT: u32 = 0;
const IOC_SIZESHIFT: u32 = 16;
const IOC_NRMASK: u32 = 0xff;
const IOC_TYPEMASK: u32 = 0xff;
const IOC_SIZEMASK: u32 = 0x3fff;

const IOC_NONE: u32 = 0;
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u32 {
    (dir << IOC_DIRSHIFT) | (ty << IOC_TYPESHIFT) | (nr << IOC_NRSHIFT) | (size << IOC_SIZESHIFT)
}

const fn ior(ty: u32, nr: u32, size: u32) -> u32 {
    ioc(IOC_READ, ty, nr, size)
}
const fn iow(ty: u32, nr: u32, size: u32) -> u32 {
    ioc(IOC_WRITE, ty, nr, size)
}
const fn iorw(ty: u32, nr: u32, size: u32) -> u32 {
    ioc(IOC_READ | IOC_WRITE, ty, nr, size)
}
const fn io(ty: u32, nr: u32) -> u32 {
    ioc(IOC_NONE, ty, nr, 0)
}

const EVDEV_IOC_TYPE: u32 = b'E' as u32;

/// `EVIOCGVERSION` — get evdev driver version.
pub const EVIOCGVERSION: u32 = ior(EVDEV_IOC_TYPE, 0x01, size_of::<u32>() as u32);
/// `EVIOCGID` — get device bus/vendor/product/version (`struct input_id`).
pub const EVIOCGID: u32 = ior(EVDEV_IOC_TYPE, 0x02, size_of::<InputId>() as u32);
/// `EVIOCGRAB` — grab/ungrab device for exclusive access.
pub const EVIOCGRAB: u32 = iow(EVDEV_IOC_TYPE, 0x90, size_of::<i32>() as u32);
/// `EVIOCSCLOCKID` — select clock source for event timestamps.
pub const EVIOCSCLOCKID: u32 = iow(EVDEV_IOC_TYPE, 0xa0, size_of::<i32>() as u32);

/// `EVIOCGNAME(len)` strips the size from the encoded ioctl so the dispatcher
/// can match any caller-supplied buffer length.
pub const EVIOCGNAME_BASE: u32 = EVDEV_IOC_TYPE << IOC_TYPESHIFT | 0x06;
pub const EVIOCGPHYS_BASE: u32 = EVDEV_IOC_TYPE << IOC_TYPESHIFT | 0x07;
pub const EVIOCGUNIQ_BASE: u32 = EVDEV_IOC_TYPE << IOC_TYPESHIFT | 0x08;
/// `EVIOCGBIT(ev, len)` — capability bitmap for event type `ev` (low byte of nr).
pub const EVIOCGBIT_BASE: u32 = EVDEV_IOC_TYPE << IOC_TYPESHIFT | 0x20;
/// `EVIOCGABS(abs)` — per-axis absinfo. Stripped form (size + abs index masked out).
pub const EVIOCGABS_BASE: u32 = EVDEV_IOC_TYPE << IOC_TYPESHIFT | 0x40;

/// `struct input_id` — `include/uapi/linux/input.h:54`. ABI-pinned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct InputId {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

/// `struct input_absinfo` — `include/uapi/linux/input.h:34`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct InputAbsinfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

// ── Per-device registry keyed by evdev minor (event0, event1, …) ──────────────

struct EvdevSlot {
    dev: Arc<InputDev>,
    /// Static device name returned by `EVIOCGNAME`.
    name: &'static str,
    id: InputId,
}

lazy_static! {
    static ref EVDEV_BY_MINOR: Mutex<BTreeMap<u32, EvdevSlot>> = Mutex::new(BTreeMap::new());
}

/// Register a device under `/dev/input/eventN`.
pub fn register_evdev_device(minor: u32, dev: Arc<InputDev>, name: &'static str, id: InputId) {
    EVDEV_BY_MINOR
        .lock()
        .insert(minor, EvdevSlot { dev, name, id });
}

#[cfg(test)]
pub fn reset_for_tests() {
    EVDEV_BY_MINOR.lock().clear();
}

fn slot_for_path(name: &str) -> Option<u32> {
    // dentry name is "event0", "event1", …
    let digits = name.strip_prefix("event")?;
    digits.parse::<u32>().ok()
}

fn with_slot<R>(file: &FileRef, f: impl FnOnce(&EvdevSlot) -> Result<R, i32>) -> Result<R, i32> {
    let minor = slot_for_path(&file.dentry.name).ok_or(ENODEV)?;
    let guard = EVDEV_BY_MINOR.lock();
    let slot = guard.get(&minor).ok_or(ENODEV)?;
    f(slot)
}

// ── FileOps implementations ───────────────────────────────────────────────────

fn evdev_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    let stride = size_of::<InputEvent>();
    if buf.len() < stride {
        return Err(EINVAL);
    }
    let events = with_slot(file, |slot| Ok(slot.dev.drain_events()))?;
    let take = core::cmp::min(buf.len() / stride, events.len());
    if take == 0 {
        // Linux blocks here when O_NONBLOCK is not set; return EAGAIN so userspace
        // can fall back to poll.
        return Err(crate::include::uapi::errno::EAGAIN);
    }
    for (i, ev) in events.iter().take(take).enumerate() {
        let dst = &mut buf[i * stride..(i + 1) * stride];
        let bytes =
            unsafe { core::slice::from_raw_parts((ev as *const InputEvent) as *const u8, stride) };
        dst.copy_from_slice(bytes);
    }
    // Re-queue any events that didn't fit — preserves order across reads.
    if events.len() > take {
        let dev = with_slot(file, |slot| Ok(slot.dev.clone()))?;
        let mut q = dev.events.lock();
        for ev in events.into_iter().skip(take).rev() {
            q.insert(0, ev);
        }
    }
    Ok(take * stride)
}

fn evdev_poll(file: &FileRef) -> u32 {
    use crate::fs::eventpoll::EPOLLIN;
    with_slot(file, |slot| {
        let queued = !slot.dev.events.lock().is_empty();
        Ok(if queued { EPOLLIN } else { 0 })
    })
    .unwrap_or(0)
}

fn evdev_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    // The length-encoded evdev ioctls (EVIOCGNAME, EVIOCGBIT, EVIOCGABS, …)
    // vary their size field per call and encode the event/axis index in the
    // low nibble of `nr`, so they cannot be matched against a fixed `cmd`.
    // Dispatch on the `type`+`nr` fields, ignoring `dir` and `size`.
    let nr = cmd & IOC_NRMASK;
    let ty = (cmd >> IOC_TYPESHIFT) & IOC_TYPEMASK;
    let size = ((cmd >> IOC_SIZESHIFT) & IOC_SIZEMASK) as usize;

    match cmd {
        EVIOCGVERSION => {
            if arg == 0 {
                return Err(EFAULT);
            }
            unsafe { crate::arch::x86::kernel::uaccess::put_user_u32(arg as *mut u32, EV_VERSION) }
                .map_err(|e| -e)?;
            Ok(0)
        }
        EVIOCGID => {
            if arg == 0 {
                return Err(EFAULT);
            }
            let id = with_slot(file, |slot| Ok(slot.id))?;
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &id as *const InputId as *const u8,
                    size_of::<InputId>(),
                )
            };
            if not_copied == 0 { Ok(0) } else { Err(EFAULT) }
        }
        EVIOCGRAB | EVIOCSCLOCKID => {
            // Accept and ignore — single-reader model means EVIOCGRAB always
            // succeeds, and event timestamps already use a monotonic source.
            Ok(0)
        }
        _ if ty == EVDEV_IOC_TYPE => match nr {
            // EVIOCGNAME(len) — device name.
            0x06 => evdev_get_name(file, arg, size),
            // EVIOCGPHYS(len) / EVIOCGUNIQ(len) — physical/unique id (empty).
            0x07 | 0x08 => evdev_get_string(arg, size, b""),
            // EVIOCGPROP(len) — input properties bitmap. We expose none.
            0x09 => evdev_get_zeroed(arg, size),
            // EVIOCGKEY / EVIOCGLED / EVIOCGSW — current key/LED/switch state.
            // All released; report an all-zero bitmap.
            0x18 | 0x19 | 0x1b => evdev_get_zeroed(arg, size),
            // EVIOCGBIT(ev, len) — capability bitmap for event type `ev`
            // (nr == 0x20 + ev; 0x20 == the supported-event-types bitmap).
            0x20..=0x3f => evdev_get_bit(file, arg, size, (nr - 0x20) as u16),
            // EVIOCGABS(abs, len) — per-axis absinfo. No absolute axes.
            0x40..=0x7f => evdev_get_abs(arg, size),
            _ => Err(ENOTTY),
        },
        _ => Err(ENOTTY),
    }
}

fn evdev_get_name(file: &FileRef, arg: u64, size: usize) -> Result<i64, i32> {
    if arg == 0 || size == 0 {
        return Err(EFAULT);
    }
    let bytes = with_slot(file, |slot| Ok(slot.name.as_bytes()))?;
    evdev_copy_string(arg, size, bytes)
}

fn evdev_get_string(arg: u64, size: usize, src: &[u8]) -> Result<i64, i32> {
    if arg == 0 || size == 0 {
        return Err(EFAULT);
    }
    evdev_copy_string(arg, size, src)
}

fn evdev_copy_string(arg: u64, size: usize, src: &[u8]) -> Result<i64, i32> {
    // Copy at most size-1 bytes plus a trailing NUL.
    let n = core::cmp::min(src.len(), size.saturating_sub(1));
    if n > 0 {
        let not_copied = unsafe {
            crate::arch::x86::kernel::uaccess::copy_to_user(arg as *mut u8, src.as_ptr(), n)
        };
        if not_copied != 0 {
            return Err(EFAULT);
        }
    }
    unsafe { crate::arch::x86::kernel::uaccess::put_user_u8((arg as *mut u8).add(n), 0) }
        .map_err(|e| -e)?;
    Ok((n + 1) as i64)
}

fn evdev_get_bit(file: &FileRef, arg: u64, size: usize, ev_type: u16) -> Result<i64, i32> {
    if arg == 0 || size == 0 {
        return Err(EFAULT);
    }
    // Capabilities are per device: `/dev/input/event0` (minor 0) is the AT
    // keyboard (EV_SYN | EV_KEY only — advertising EV_REL/EV_ABS would make
    // X.Org's evdev driver misclassify it as a pointer); other minors are the
    // PS/2 mouse (EV_SYN | EV_KEY | EV_REL — only the BTN_* buttons and the
    // X/Y/wheel relative axes).
    let minor = slot_for_path(&file.dentry.name).unwrap_or(0);
    let is_keyboard = minor == 0;
    let mut buf = [0u8; 128];
    let n = core::cmp::min(size, buf.len());
    match ev_type {
        0 => {
            // Bitmap of supported event types. Bit 0 = EV_SYN, 1 = EV_KEY, …
            buf[0] |= 1 << EV_SYN;
            buf[0] |= 1 << EV_KEY;
            if !is_keyboard {
                buf[0] |= 1 << EV_REL;
            }
        }
        EV_KEY if is_keyboard => {
            // Report support for the keyboard key range (codes 0x00..0xff).
            // Leaving the BTN_* range (0x100+) clear keeps the keyboard from
            // also looking like it has pointer buttons.
            let keys = core::cmp::min(n, 32);
            for byte in &mut buf[..keys] {
                *byte = 0xff;
            }
        }
        EV_KEY => {
            // Pointer: advertise only the three mouse buttons so X.Org's evdev
            // driver classifies it as a pointer (which requires BTN_LEFT), not
            // a keyboard.  BTN_LEFT=0x110, BTN_RIGHT=0x111, BTN_MIDDLE=0x112.
            set_code_bit(&mut buf, n, 0x110);
            set_code_bit(&mut buf, n, 0x111);
            set_code_bit(&mut buf, n, 0x112);
        }
        EV_REL if !is_keyboard => {
            // REL_X=0, REL_Y=1, REL_WHEEL=8.
            set_code_bit(&mut buf, n, 0x00);
            set_code_bit(&mut buf, n, 0x01);
            set_code_bit(&mut buf, n, 0x08);
        }
        _ => {
            // Unsupported event type for this device — all-zero bitmap.
        }
    }
    let not_copied =
        unsafe { crate::arch::x86::kernel::uaccess::copy_to_user(arg as *mut u8, buf.as_ptr(), n) };
    if not_copied == 0 {
        Ok(n as i64)
    } else {
        Err(EFAULT)
    }
}

/// Set the bit for evdev event `code` in a little-endian capability bitmap,
/// if the byte holding it fits within the caller-supplied length `n`.
fn set_code_bit(buf: &mut [u8], n: usize, code: u16) {
    let byte = (code / 8) as usize;
    if byte < n && byte < buf.len() {
        buf[byte] |= 1 << (code % 8);
    }
}

/// Copy an all-zero bitmap of `size` bytes to userspace. Used by the state
/// ioctls (EVIOCGKEY/EVIOCGLED/EVIOCGSW) and EVIOCGPROP, which report "nothing
/// currently set / no properties".
fn evdev_get_zeroed(arg: u64, size: usize) -> Result<i64, i32> {
    if arg == 0 || size == 0 {
        return Err(EFAULT);
    }
    let buf = [0u8; 128];
    let n = core::cmp::min(size, buf.len());
    let not_copied =
        unsafe { crate::arch::x86::kernel::uaccess::copy_to_user(arg as *mut u8, buf.as_ptr(), n) };
    if not_copied == 0 {
        Ok(n as i64)
    } else {
        Err(EFAULT)
    }
}

/// EVIOCGABS(abs) — per-axis absinfo. We expose no absolute axes, so return a
/// zeroed `input_absinfo`.
fn evdev_get_abs(arg: u64, size: usize) -> Result<i64, i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    if size < size_of::<InputAbsinfo>() {
        return Err(EINVAL);
    }
    let abs = InputAbsinfo::default();
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            arg as *mut u8,
            &abs as *const InputAbsinfo as *const u8,
            size_of::<InputAbsinfo>(),
        )
    };
    if not_copied == 0 { Ok(0) } else { Err(EFAULT) }
}

/// `file_operations` for the evdev character device.
pub const EVDEV_FILE_OPS: FileOps = FileOps {
    name: "evdev",
    read: Some(evdev_read),
    write: None,
    llseek: None,
    fsync: None,
    poll: Some(evdev_poll as PollFn),
    ioctl: Some(evdev_ioctl as IoctlFn),
    mmap: None,
    release: None,
    readdir: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::dcache::d_alloc_child;
    use crate::fs::types::{Dentry, File};
    use crate::linux_driver_abi::input::{KEY_A, KEY_ENTER, input_register_device};

    fn make_file(name: &str) -> FileRef {
        // We don't need a real inode for read/poll — the FileOps only touches
        // the dentry name to identify the minor.
        let parent = Dentry::new_negative("input");
        let child = d_alloc_child(&parent, name);
        File::new(child, 0, 0o666, &EVDEV_FILE_OPS)
    }

    // Each test uses a unique (minor, device_id, dentry name) so cargo's
    // parallel test runner doesn't make them race over the global registry.
    fn setup_dev(minor: u32, dev_id: u32) -> (Arc<InputDev>, FileRef) {
        let dev = InputDev::new("kbd", dev_id);
        let _ = input_register_device(dev.clone());
        register_evdev_device(minor, dev.clone(), "kbd", InputId::default());
        let name = alloc::format!("event{}", minor);
        let file = make_file(&name);
        (dev, file)
    }

    #[test]
    fn evdev_read_drains_events() {
        let (dev, file) = setup_dev(100, 0xE100);
        dev.input_event(EV_KEY, KEY_A, 1);
        dev.input_event(EV_KEY, KEY_A, 0);

        let mut buf = [0u8; 256];
        let mut pos = 0u64;
        let n = evdev_read(&file, &mut buf, &mut pos).unwrap();
        assert_eq!(n, 2 * size_of::<InputEvent>());

        let second = evdev_read(&file, &mut buf, &mut pos);
        assert_eq!(second, Err(crate::include::uapi::errno::EAGAIN));
    }

    #[test]
    fn evdev_poll_reports_pollin_when_queued() {
        let (dev, file) = setup_dev(101, 0xE101);
        assert_eq!(evdev_poll(&file), 0);
        dev.input_event(EV_KEY, KEY_ENTER, 1);
        assert_eq!(evdev_poll(&file), crate::fs::eventpoll::EPOLLIN);
    }

    #[test]
    fn evdev_partial_read_requeues_remaining() {
        let (dev, file) = setup_dev(102, 0xE102);
        for _ in 0..3 {
            dev.input_event(EV_KEY, KEY_A, 1);
        }
        let mut buf = [0u8; size_of::<InputEvent>()]; // room for exactly one
        let mut pos = 0u64;
        let n = evdev_read(&file, &mut buf, &mut pos).unwrap();
        assert_eq!(n, size_of::<InputEvent>());
        // Two events should remain queued for the next read.
        assert_eq!(dev.events.lock().len(), 2);
    }

    #[test]
    fn ioctl_unknown_returns_enotty() {
        let (_dev, file) = setup_dev(103, 0xE103);
        let r = evdev_ioctl(&file, 0xDEAD_BEEF, 0);
        assert_eq!(r, Err(ENOTTY));
    }
}
