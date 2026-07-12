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
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::EEXIST;
use crate::kernel::sched::wait::WaitQueueHead;

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
}
