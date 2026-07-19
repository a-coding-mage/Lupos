//! linux-parity: partial
//! linux-source: vendor/linux/drivers/input
//! test-origin: linux:vendor/linux/drivers/input
//! Minimal i8042/AT keyboard and mouse path for the QEMU VGA console.
//!
//! The hard-IRQ path mirrors Linux's interrupt-driven controller ownership but
//! defers allocation and compatibility event publication through a fixed
//! lock-free byte ring. The remaining global evdev/cooked-console bridge is
//! Lupos-specific and is therefore not complete Linux input-core parity.

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;
const STATUS_OUTPUT_FULL: u8 = 0x01;
const STATUS_INPUT_FULL: u8 = 0x02;
/// Status bit 5 (`0x20`): the byte in the output buffer came from the **aux**
/// (PS/2 mouse) channel rather than the keyboard.  Linux `i8042.c` calls this
/// `I8042_STR_AUXDATA`.
const STATUS_AUX_DATA: u8 = 0x20;
const COMMAND_READ_CONFIG: u8 = 0x20;
const COMMAND_WRITE_CONFIG: u8 = 0x60;
const COMMAND_ENABLE_KEYBOARD: u8 = 0xAE;
/// `0xA8` — enable the aux (mouse) port / clock.
const COMMAND_ENABLE_AUX: u8 = 0xA8;
/// `0xD4` — the next byte written to the data port is routed to the mouse.
const COMMAND_WRITE_AUX: u8 = 0xD4;
const CONFIG_IRQ1: u8 = 0x01;
/// Controller-config bit 1: raise IRQ12 when aux data arrives.
const CONFIG_IRQ12: u8 = 0x02;
const CONFIG_DISABLE_KEYBOARD: u8 = 0x10;
/// Controller-config bit 5: aux (mouse) clock disabled.  Cleared to let mouse
/// packets flow.
const CONFIG_DISABLE_MOUSE: u8 = 0x20;
const CONFIG_TRANSLATION: u8 = 0x40;
const KEYBOARD_ENABLE_SCANNING: u8 = 0xF4;
const KEYBOARD_SET_LEDS: u8 = 0xED;
const KEYBOARD_ACK: u8 = 0xFA;
const KEYBOARD_RESEND: u8 = 0xFE;
pub const LED_NUML: u16 = 0;
pub const LED_CAPSL: u16 = 1;
pub const LED_SCROLLL: u16 = 2;
/// PS/2 mouse commands (`drivers/input/mouse/psmouse-base.c`).
const MOUSE_SET_DEFAULTS: u8 = 0xF6;
const MOUSE_ENABLE_REPORTING: u8 = 0xF4;
/// Standard PS/2 device acknowledge byte.
const MOUSE_ACK: u8 = 0xFA;
const I8042_QUEUE_CAPACITY: usize = 256;

/// evdev id of `/dev/input/event1` (the PS/2 mouse) — see
/// `input::register_default_evdev_devices`.
const EVDEV_MOUSE_ID: u32 = 0xE002;

static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static EXTENDED: AtomicBool = AtomicBool::new(false);
/// Linux evdev LED bitmap: bit 0 Num Lock, bit 1 Caps Lock, bit 2 Scroll Lock.
/// [`evdev_leds_to_ps2`] translates it to the PS/2 wire ordering.
static KEYBOARD_LEDS: AtomicU8 = AtomicU8::new(0);
static IRQ_DRIVEN: AtomicBool = AtomicBool::new(false);
static IRQ_DROPPED_BYTES: AtomicUsize = AtomicUsize::new(0);
static mut I8042_IRQ_COOKIE: u8 = 0;

/// Separate extended-prefix latch for the evdev bridge.  The console decoder
/// consumes its own [`EXTENDED`] latch inside `decode_scancode_input`, so the
/// raw-scancode → evdev path tracks the `0xE0` prefix independently.
static EVDEV_EXTENDED: AtomicBool = AtomicBool::new(false);

/// evdev id of `/dev/input/event0` (the AT keyboard) — see
/// `input::register_default_evdev_devices`.
const EVDEV_KEYBOARD_ID: u32 = 0xE001;

struct IrqByteQueue {
    slots: [AtomicU16; I8042_QUEUE_CAPACITY],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl IrqByteQueue {
    const fn new() -> Self {
        Self {
            slots: [const { AtomicU16::new(0) }; I8042_QUEUE_CAPACITY],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    fn push(&self, byte: u8, aux: bool) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % I8042_QUEUE_CAPACITY;
        if next == self.tail.load(Ordering::Acquire) {
            return false;
        }
        self.slots[head].store(
            u16::from(byte) | if aux { 1 << 8 } else { 0 },
            Ordering::Relaxed,
        );
        self.head.store(next, Ordering::Release);
        true
    }

    fn pop(&self) -> Option<(u8, bool)> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        let value = self.slots[tail].load(Ordering::Relaxed);
        self.tail
            .store((tail + 1) % I8042_QUEUE_CAPACITY, Ordering::Release);
        Some((value as u8, value & (1 << 8) != 0))
    }

    fn clear(&self) {
        self.tail
            .store(self.head.load(Ordering::Acquire), Ordering::Release);
    }
}

static IRQ_BYTE_QUEUE: IrqByteQueue = IrqByteQueue::new();

lazy_static! {
    static ref BYTE_QUEUE: Mutex<VecDeque<ConsoleInput>> = Mutex::new(VecDeque::new());
    /// Scancodes observed while synchronously waiting for a keyboard-command
    /// ACK. They must re-enter the normal decoder instead of being discarded.
    static ref PENDING_KEYBOARD_SCANCODES: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
    static ref MOUSE_STATE: Mutex<MousePacket> = Mutex::new(MousePacket::new());
}

/// Whether the aux (mouse) channel initialised and enabled stream reporting.
static MOUSE_PRESENT: AtomicBool = AtomicBool::new(false);

/// Accumulates the 3-byte PS/2 mouse movement packet
/// (`drivers/input/mouse/psmouse-base.c`).
struct MousePacket {
    bytes: [u8; 3],
    index: usize,
    /// Previous button bitmap (bit0 left, bit1 right, bit2 middle) so we only
    /// emit `EV_KEY` transitions.
    buttons: u8,
}

impl MousePacket {
    const fn new() -> Self {
        Self {
            bytes: [0; 3],
            index: 0,
            buttons: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodedInput {
    Byte(u8),
    Sequence(&'static [u8]),
    Action(ConsoleAction),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleAction {
    Shutdown,
    Restart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleInput {
    Byte(u8),
    Shutdown,
    Restart,
}

pub fn init(use_irqs: bool) {
    BYTE_QUEUE.lock().clear();
    PENDING_KEYBOARD_SCANCODES.lock().clear();
    *MOUSE_STATE.lock() = MousePacket::new();
    SHIFT_DOWN.store(false, Ordering::Release);
    CTRL_DOWN.store(false, Ordering::Release);
    ALT_DOWN.store(false, Ordering::Release);
    EXTENDED.store(false, Ordering::Release);
    EVDEV_EXTENDED.store(false, Ordering::Release);
    KEYBOARD_LEDS.store(0, Ordering::Release);
    MOUSE_PRESENT.store(false, Ordering::Release);
    IRQ_DRIVEN.store(false, Ordering::Release);
    IRQ_DROPPED_BYTES.store(0, Ordering::Release);
    IRQ_BYTE_QUEUE.clear();
    drain_pending();

    // Linux requests both i8042 IRQs before enabling the corresponding port.
    // Keep the controller's interrupt bits clear while commands and ACKs are
    // exchanged, then enable only after the handlers and deferred consumer
    // are installed.
    let mut config = read_config().unwrap_or(CONFIG_TRANSLATION);
    config &= !(CONFIG_IRQ1 | CONFIG_IRQ12);
    write_config(config);

    let (keyboard_irq, aux_irq) = if use_irqs {
        crate::kernel::softirq::open_softirq(
            crate::kernel::softirq::SoftIrqVec::IrqPoll,
            drain_irq_bytes,
        );
        let dev_id = core::ptr::addr_of_mut!(I8042_IRQ_COOKIE).cast();
        let aux_irq = crate::kernel::irq::request_irq(
            12,
            i8042_interrupt,
            crate::kernel::irq::IRQF_SHARED,
            "i8042",
            dev_id,
        )
        .is_ok();
        let keyboard_irq = crate::kernel::irq::request_irq(
            1,
            i8042_interrupt,
            crate::kernel::irq::IRQF_SHARED,
            "i8042",
            dev_id,
        )
        .is_ok();
        (keyboard_irq, aux_irq)
    } else {
        (false, false)
    };

    // Enable the aux (mouse) port before touching the config byte so the
    // controller accepts the mouse-directed writes below.
    let _ = write_command(COMMAND_ENABLE_AUX);
    let _ = write_command(COMMAND_ENABLE_KEYBOARD);
    let _ = write_data(KEYBOARD_ENABLE_SCANNING);

    init_mouse();
    drain_pending();

    config &= !(CONFIG_DISABLE_KEYBOARD | CONFIG_DISABLE_MOUSE);
    config |= CONFIG_TRANSLATION;
    if keyboard_irq {
        config |= CONFIG_IRQ1;
    }
    if aux_irq {
        config |= CONFIG_IRQ12;
    }
    write_config(config);
    IRQ_DRIVEN.store(keyboard_irq, Ordering::Release);
    if !use_irqs {
        crate::log_info!("i8042", "polling fallback selected by lupos.i8042_poll=1");
    } else if keyboard_irq {
        crate::log_info!(
            "i8042",
            "IRQ-driven input online: keyboard=1 aux={}",
            usize::from(aux_irq)
        );
    } else {
        crate::log_warn!(
            "i8042",
            "IRQ1 registration failed; retaining polling fallback"
        );
    }
}

unsafe extern "C" fn i8042_interrupt(_irq: u32, _dev_id: *mut core::ffi::c_void) -> i32 {
    let mut handled = 0usize;
    // QEMU may coalesce a short make/break burst while the edge-triggered ISA
    // line is asserted. Drain the controller output buffer completely so a
    // byte left behind cannot wait forever for an edge that never re-arms.
    // The fixed bound prevents a broken controller from trapping the CPU in
    // hard-IRQ context.
    for _ in 0..32 {
        let status = unsafe { crate::arch::x86::include::asm::io::inb(STATUS_PORT) };
        if status & STATUS_OUTPUT_FULL == 0 {
            break;
        }
        let byte = unsafe { crate::arch::x86::include::asm::io::inb(DATA_PORT) };
        if !IRQ_BYTE_QUEUE.push(byte, status & STATUS_AUX_DATA != 0) {
            IRQ_DROPPED_BYTES.fetch_add(1, Ordering::Relaxed);
        }
        handled += 1;
    }
    if handled == 0 {
        return crate::kernel::irq::IRQ_NONE;
    }
    crate::kernel::softirq::raise_softirq(crate::kernel::softirq::SoftIrqVec::IrqPoll);
    crate::kernel::irq::IRQ_HANDLED
}

fn drain_irq_bytes() {
    while let Some((byte, aux)) = IRQ_BYTE_QUEUE.pop() {
        if aux {
            feed_mouse_byte(byte);
            continue;
        }
        feed_evdev_scancode(byte);
        if crate::linux_driver_abi::tty::compat_cooked_keyboard_input_enabled() {
            queue_decoded(byte);
        }
    }
    let dropped = IRQ_DROPPED_BYTES.swap(0, Ordering::AcqRel);
    if dropped != 0 {
        crate::log_warn!("i8042", "deferred input ring dropped {} bytes", dropped);
    }
}

/// Initialise the PS/2 mouse: restore defaults, then enable stream-mode data
/// reporting.  Each command is answered with `0xFA` (ACK).  Sets
/// [`MOUSE_PRESENT`] when reporting is successfully enabled.
fn init_mouse() {
    // Set-defaults settles sample rate / resolution / scaling.
    let _ = mouse_command(MOUSE_SET_DEFAULTS);
    // Enable data reporting — the mouse now streams 3-byte movement packets.
    if mouse_command(MOUSE_ENABLE_REPORTING) {
        MOUSE_PRESENT.store(true, Ordering::Release);
    }
}

/// Send one byte to the mouse (via the `0xD4` prefix) and consume its ACK.
/// Returns true when the mouse acknowledged (`0xFA`).
fn mouse_command(command: u8) -> bool {
    if !write_command(COMMAND_WRITE_AUX) {
        return false;
    }
    if !write_data(command) {
        return false;
    }
    // The ACK is delivered on the aux channel; a set-defaults/reset can also
    // emit a self-test byte first, so scan a few responses for the ACK.
    for _ in 0..4 {
        match read_data_timeout() {
            Some(MOUSE_ACK) => return true,
            Some(_) => continue,
            None => return false,
        }
    }
    false
}

/// Whether the PS/2 mouse initialised and is streaming movement packets.
pub fn mouse_present() -> bool {
    MOUSE_PRESENT.load(Ordering::Acquire)
}

fn wait_input_clear() -> bool {
    for _ in 0..100_000 {
        if unsafe {
            (crate::arch::x86::include::asm::io::inb(STATUS_PORT) & STATUS_INPUT_FULL) == 0
        } {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn write_command(command: u8) -> bool {
    if !wait_input_clear() {
        return false;
    }
    unsafe {
        crate::arch::x86::include::asm::io::outb(COMMAND_PORT, command);
    }
    true
}

fn write_data(data: u8) -> bool {
    if !wait_input_clear() {
        return false;
    }
    unsafe {
        crate::arch::x86::include::asm::io::outb(DATA_PORT, data);
    }
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyboardResponse {
    Ack,
    Resend,
}

/// Wait for a response to a command sent to the keyboard. Mouse packets are
/// decoded in place and any interleaved keyboard scancodes are retained for
/// the ordinary input pump.
#[cfg(not(test))]
fn wait_keyboard_response() -> Option<KeyboardResponse> {
    for _ in 0..100_000 {
        let status = unsafe { crate::arch::x86::include::asm::io::inb(STATUS_PORT) };
        if status & STATUS_OUTPUT_FULL == 0 {
            core::hint::spin_loop();
            continue;
        }
        let byte = unsafe { crate::arch::x86::include::asm::io::inb(DATA_PORT) };
        if status & STATUS_AUX_DATA != 0 {
            feed_mouse_byte(byte);
            continue;
        }
        match byte {
            KEYBOARD_ACK => return Some(KeyboardResponse::Ack),
            KEYBOARD_RESEND => return Some(KeyboardResponse::Resend),
            scancode => PENDING_KEYBOARD_SCANCODES.lock().push_back(scancode),
        }
    }
    None
}

#[cfg(not(test))]
fn write_keyboard_byte_with_ack(byte: u8) -> bool {
    for _ in 0..2 {
        if !write_data(byte) {
            return false;
        }
        match wait_keyboard_response() {
            Some(KeyboardResponse::Ack) => return true,
            Some(KeyboardResponse::Resend) => continue,
            None => return false,
        }
    }
    false
}

#[cfg(not(test))]
fn program_keyboard_leds(leds: u8) {
    if write_keyboard_byte_with_ack(KEYBOARD_SET_LEDS) {
        let _ = write_keyboard_byte_with_ack(evdev_leds_to_ps2(leds));
    }
}

#[cfg(test)]
fn program_keyboard_leds(_leds: u8) {}

fn evdev_leds_to_ps2(leds: u8) -> u8 {
    ((leds & (1 << LED_NUML)) << 1)
        | ((leds & (1 << LED_CAPSL)) << 1)
        | ((leds & (1 << LED_SCROLLL)) >> 2)
}

/// Current evdev-compatible keyboard LED bitmap.
pub fn keyboard_leds() -> u8 {
    KEYBOARD_LEDS.load(Ordering::Acquire)
}

/// Apply one Linux `EV_LED` value and forward the complete bitmap to the PS/2
/// keyboard. Returns false for an unsupported LED code.
pub fn set_keyboard_led(code: u16, enabled: bool) -> bool {
    if !matches!(code, LED_NUML | LED_CAPSL | LED_SCROLLL) {
        return false;
    }
    let mask = 1u8 << code;
    let previous = if enabled {
        KEYBOARD_LEDS.fetch_or(mask, Ordering::AcqRel)
    } else {
        KEYBOARD_LEDS.fetch_and(!mask, Ordering::AcqRel)
    };
    let next = if enabled {
        previous | mask
    } else {
        previous & !mask
    };
    if next != previous {
        program_keyboard_leds(next);
    }
    true
}

fn read_data_timeout() -> Option<u8> {
    for _ in 0..100_000 {
        if controller_has_data() {
            return Some(unsafe { crate::arch::x86::include::asm::io::inb(DATA_PORT) });
        }
        core::hint::spin_loop();
    }
    None
}

fn read_config() -> Option<u8> {
    if !write_command(COMMAND_READ_CONFIG) {
        return None;
    }
    read_data_timeout()
}

fn write_config(config: u8) {
    if write_command(COMMAND_WRITE_CONFIG) {
        let _ = write_data(config);
    }
}

fn drain_pending() {
    for _ in 0..32 {
        if !controller_has_data() {
            break;
        }
        unsafe {
            let _ = crate::arch::x86::include::asm::io::inb(DATA_PORT);
        }
    }
}

fn controller_has_data() -> bool {
    unsafe { (crate::arch::x86::include::asm::io::inb(STATUS_PORT) & STATUS_OUTPUT_FULL) != 0 }
}

pub fn try_read_byte() -> Option<u8> {
    match try_read_input() {
        Some(ConsoleInput::Byte(byte)) => Some(byte),
        _ => None,
    }
}

pub fn try_read_input() -> Option<ConsoleInput> {
    if crate::linux_driver_abi::tty::compat_cooked_keyboard_input_enabled() {
        if let Some(input) = BYTE_QUEUE.lock().pop_front() {
            return Some(input);
        }
    } else {
        BYTE_QUEUE.lock().clear();
    }
    // Once IRQ1 is registered, the hard handler is the sole reader of port
    // 0x60. Keeping the old syscall/epoll poller active would race it and can
    // consume bytes before the IRQ path publishes the evdev frame.
    if IRQ_DRIVEN.load(Ordering::Acquire) {
        return None;
    }
    // Port 0x60 is shared by the keyboard and the aux (mouse) channel; the
    // status byte's AUX bit says which produced the pending byte.  This polled
    // path is the single reader (pumped from the idle loop), so drain until we
    // either surface a console byte or the buffer empties — mouse bytes and
    // bare modifiers must not stop the drain, or a keystroke queued behind a
    // mouse packet would stall.
    loop {
        let pending = PENDING_KEYBOARD_SCANCODES.lock().pop_front();
        let (byte, aux) = if let Some(byte) = pending {
            (byte, false)
        } else {
            let status = unsafe { crate::arch::x86::include::asm::io::inb(STATUS_PORT) };
            if status & STATUS_OUTPUT_FULL == 0 {
                return None;
            }
            (
                unsafe { crate::arch::x86::include::asm::io::inb(DATA_PORT) },
                status & STATUS_AUX_DATA != 0,
            )
        };
        if aux {
            feed_mouse_byte(byte);
            continue;
        }
        // Mirror every raw scancode into the evdev `/dev/input/event0` device
        // so X.Org's evdev driver receives keystrokes, while the cooked ASCII
        // path keeps feeding the console/tty.
        feed_evdev_scancode(byte);
        if let Some(input) = enqueue_decoded_if_cooked(
            byte,
            crate::linux_driver_abi::tty::compat_cooked_keyboard_input_enabled(),
        ) {
            return Some(input);
        }
    }
}

/// Feed one aux-channel byte into the PS/2 mouse packet decoder and, on a
/// complete 3-byte packet, emit the corresponding evdev events on
/// `/dev/input/event1` (`EV_REL` motion + `EV_KEY` button transitions).
///
/// PS/2 packet layout (`drivers/input/mouse/psmouse-base.c`):
///   byte0: YO XO YS XS 1 M R L   (overflow, sign, always-1, buttons)
///   byte1: X movement (9-bit two's complement with XS)
///   byte2: Y movement (9-bit two's complement with YS)
fn feed_mouse_byte(byte: u8) {
    use super::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, EV_KEY, EV_REL, EV_SYN, REL_X, REL_Y};

    let (dx, dy, buttons, prev) = {
        let mut pkt = MOUSE_STATE.lock();
        // Resync: the first packet byte always has bit3 set.  If it is clear we
        // are mid-stream out of alignment, so drop the byte.
        if pkt.index == 0 && (byte & 0x08) == 0 {
            return;
        }
        let idx = pkt.index;
        pkt.bytes[idx] = byte;
        pkt.index += 1;
        if pkt.index < 3 {
            return;
        }
        pkt.index = 0;

        let flags = pkt.bytes[0];
        let mut dx = pkt.bytes[1] as i32;
        let mut dy = pkt.bytes[2] as i32;
        // Sign-extend the 9-bit deltas.
        if flags & 0x10 != 0 {
            dx -= 0x100;
        }
        if flags & 0x20 != 0 {
            dy -= 0x100;
        }
        // Overflow bits mean the deltas are unreliable — drop the motion but
        // still honour the button state.
        if flags & 0xC0 != 0 {
            dx = 0;
            dy = 0;
        }
        let buttons = flags & 0x07;
        let prev = pkt.buttons;
        pkt.buttons = buttons;
        (dx, dy, buttons, prev)
    };

    let Some(dev) = super::find_input_dev(EVDEV_MOUSE_ID) else {
        return;
    };

    let mut emitted = false;
    for (mask, code) in [(0x01u8, BTN_LEFT), (0x02, BTN_RIGHT), (0x04, BTN_MIDDLE)] {
        if (buttons & mask) != (prev & mask) {
            dev.input_event(EV_KEY, code, if buttons & mask != 0 { 1 } else { 0 });
            emitted = true;
        }
    }
    if dx != 0 {
        dev.input_event(EV_REL, REL_X, dx);
        emitted = true;
    }
    if dy != 0 {
        // PS/2 reports +Y as "up"; evdev/screen coordinates grow downward.
        dev.input_event(EV_REL, REL_Y, -dy);
        emitted = true;
    }
    if emitted {
        dev.input_event(EV_SYN, 0, 0);
    }
}

/// Bridge a raw AT set-1 scancode into an evdev `EV_KEY` event on
/// `/dev/input/event0`.  For the set-1 base range (`0x01..=0x58`) the Linux
/// keycode equals the scancode value — Linux's keycode table is aligned to AT
/// set-1 — and the high bit is the key-release flag.  Extended (`0xE0`-prefixed)
/// codes use a small side table.  A `SYN_REPORT` follows each key event, as
/// evdev readers expect.
fn feed_evdev_scancode(scancode: u8) {
    // Multi-byte prefixes: latch 0xE0, ignore the 0xE1 (Pause) lead-in.
    if scancode == 0xE0 {
        EVDEV_EXTENDED.store(true, Ordering::Release);
        return;
    }
    if scancode == 0xE1 {
        return;
    }
    let extended = EVDEV_EXTENDED.swap(false, Ordering::AcqRel);
    let Some((keycode, value)) = evdev_key_event(scancode, extended) else {
        return;
    };
    let Some(dev) = super::find_input_dev(EVDEV_KEYBOARD_ID) else {
        return;
    };
    dev.input_event(super::EV_KEY, keycode, value);
    // SYN_REPORT (code 0) closes the event frame.
    dev.input_event(super::EV_SYN, 0, 0);
}

/// Pure scancode → `(keycode, value)` translation for the evdev bridge.
/// `value` is 1 for press, 0 for release.  Returns `None` for scancodes with
/// no evdev mapping (unknown extended codes, out-of-range base codes).
fn evdev_key_event(scancode: u8, extended: bool) -> Option<(u16, i32)> {
    let released = (scancode & 0x80) != 0;
    let base = scancode & 0x7F;
    let keycode = if extended {
        evdev_extended_keycode(base)?
    } else if (0x01..=0x58).contains(&base) {
        base as u16
    } else {
        return None;
    };
    Some((keycode, if released { 0 } else { 1 }))
}

/// Map the common `0xE0`-prefixed set-1 scancodes to their Linux keycodes
/// (`include/uapi/linux/input-event-codes.h`).  Only the keys a login session
/// realistically needs are covered; anything else is dropped.
fn evdev_extended_keycode(base: u8) -> Option<u16> {
    Some(match base {
        0x1C => 96,  // KEY_KPENTER
        0x1D => 97,  // KEY_RIGHTCTRL
        0x35 => 98,  // KEY_KPSLASH
        0x38 => 100, // KEY_RIGHTALT
        0x47 => 102, // KEY_HOME
        0x48 => 103, // KEY_UP
        0x49 => 104, // KEY_PAGEUP
        0x4B => 105, // KEY_LEFT
        0x4D => 106, // KEY_RIGHT
        0x4F => 107, // KEY_END
        0x50 => 108, // KEY_DOWN
        0x51 => 109, // KEY_PAGEDOWN
        0x52 => 110, // KEY_INSERT
        0x53 => 111, // KEY_DELETE
        0x5B => 125, // KEY_LEFTMETA
        0x5C => 126, // KEY_RIGHTMETA
        _ => return None,
    })
}

fn enqueue_decoded(scancode: u8) -> Option<ConsoleInput> {
    match decode_scancode_input(scancode) {
        Some(DecodedInput::Byte(byte)) => Some(ConsoleInput::Byte(byte)),
        Some(DecodedInput::Sequence(bytes)) => {
            let mut queue = BYTE_QUEUE.lock();
            queue.extend(bytes.iter().copied().map(ConsoleInput::Byte));
            queue.pop_front()
        }
        Some(DecodedInput::Action(ConsoleAction::Shutdown)) => Some(ConsoleInput::Shutdown),
        Some(DecodedInput::Action(ConsoleAction::Restart)) => {
            BYTE_QUEUE.lock().clear();
            Some(ConsoleInput::Restart)
        }
        None => None,
    }
}

fn queue_decoded(scancode: u8) {
    let Some(decoded) = decode_scancode_input(scancode) else {
        return;
    };
    let mut queue = BYTE_QUEUE.lock();
    match decoded {
        DecodedInput::Byte(byte) => queue.push_back(ConsoleInput::Byte(byte)),
        DecodedInput::Sequence(bytes) => {
            queue.extend(bytes.iter().copied().map(ConsoleInput::Byte))
        }
        DecodedInput::Action(ConsoleAction::Shutdown) => queue.push_back(ConsoleInput::Shutdown),
        DecodedInput::Action(ConsoleAction::Restart) => queue.push_back(ConsoleInput::Restart),
    }
}

fn enqueue_decoded_if_cooked(scancode: u8, cooked: bool) -> Option<ConsoleInput> {
    if cooked {
        enqueue_decoded(scancode)
    } else {
        None
    }
}

pub fn decode_scancode(scancode: u8) -> Option<u8> {
    match decode_scancode_input(scancode) {
        Some(DecodedInput::Byte(byte)) => Some(byte),
        _ => None,
    }
}

pub fn decode_scancode_input(scancode: u8) -> Option<DecodedInput> {
    if scancode == 0xE0 {
        EXTENDED.store(true, Ordering::Release);
        return None;
    }

    let extended = EXTENDED.swap(false, Ordering::AcqRel);
    if extended {
        return decode_extended_scancode(scancode);
    }

    match scancode {
        0x45 => {
            let enabled = keyboard_leds() & (1 << LED_NUML) == 0;
            let _ = set_keyboard_led(LED_NUML, enabled);
            None
        }
        0x2A | 0x36 => {
            SHIFT_DOWN.store(true, Ordering::Release);
            None
        }
        0xAA | 0xB6 => {
            SHIFT_DOWN.store(false, Ordering::Release);
            None
        }
        0x1D => {
            CTRL_DOWN.store(true, Ordering::Release);
            None
        }
        0x9D => {
            CTRL_DOWN.store(false, Ordering::Release);
            None
        }
        0x38 => {
            ALT_DOWN.store(true, Ordering::Release);
            None
        }
        0xB8 => {
            ALT_DOWN.store(false, Ordering::Release);
            None
        }
        code if (code & 0x80) != 0 => None,
        0x37 => Some(DecodedInput::Byte(b'*')),
        0x4A => Some(DecodedInput::Byte(b'-')),
        0x4E => Some(DecodedInput::Byte(b'+')),
        code @ (0x47..=0x49 | 0x4B..=0x4D | 0x4F..=0x53) => Some(decode_keypad_scancode(
            code,
            keyboard_leds() & (1 << LED_NUML) != 0,
        )),
        code => translate_scancode(
            code,
            SHIFT_DOWN.load(Ordering::Acquire),
            CTRL_DOWN.load(Ordering::Acquire),
        )
        .map(DecodedInput::Byte),
    }
}

fn decode_keypad_scancode(scancode: u8, num_lock: bool) -> DecodedInput {
    if num_lock {
        return DecodedInput::Byte(match scancode {
            0x47 => b'7',
            0x48 => b'8',
            0x49 => b'9',
            0x4B => b'4',
            0x4C => b'5',
            0x4D => b'6',
            0x4F => b'1',
            0x50 => b'2',
            0x51 => b'3',
            0x52 => b'0',
            0x53 => b'.',
            _ => unreachable!(),
        });
    }

    DecodedInput::Sequence(match scancode {
        0x47 => b"\x1b[1~",
        0x48 => b"\x1b[A",
        0x49 => b"\x1b[5~",
        0x4B => b"\x1b[D",
        0x4C => b"\x1b[G",
        0x4D => b"\x1b[C",
        0x4F => b"\x1b[4~",
        0x50 => b"\x1b[B",
        0x51 => b"\x1b[6~",
        0x52 => b"\x1b[2~",
        0x53 => b"\x1b[3~",
        _ => unreachable!(),
    })
}

fn decode_extended_scancode(scancode: u8) -> Option<DecodedInput> {
    match scancode {
        0x1D => {
            CTRL_DOWN.store(true, Ordering::Release);
            None
        }
        0x9D => {
            CTRL_DOWN.store(false, Ordering::Release);
            None
        }
        0x38 => {
            ALT_DOWN.store(true, Ordering::Release);
            None
        }
        0xB8 => {
            ALT_DOWN.store(false, Ordering::Release);
            None
        }
        code if (code & 0x80) != 0 => None,
        0x1C => Some(DecodedInput::Byte(b'\n')),
        0x35 => Some(DecodedInput::Byte(b'/')),
        0x47 => Some(DecodedInput::Sequence(b"\x1b[H")),
        0x48 => Some(DecodedInput::Sequence(b"\x1b[A")),
        0x49 => Some(DecodedInput::Sequence(b"\x1b[5~")),
        0x4B => Some(DecodedInput::Sequence(b"\x1b[D")),
        0x4D => Some(DecodedInput::Sequence(b"\x1b[C")),
        0x4F => Some(DecodedInput::Sequence(b"\x1b[F")),
        0x50 => Some(DecodedInput::Sequence(b"\x1b[B")),
        0x51 => Some(DecodedInput::Sequence(b"\x1b[6~")),
        0x52 => Some(DecodedInput::Sequence(b"\x1b[2~")),
        0x53 => decode_delete_key(),
        _ => None,
    }
}

fn decode_delete_key() -> Option<DecodedInput> {
    let ctrl = CTRL_DOWN.load(Ordering::Acquire);
    let shift = SHIFT_DOWN.load(Ordering::Acquire);
    let alt = ALT_DOWN.load(Ordering::Acquire);
    if ctrl && shift {
        Some(DecodedInput::Action(ConsoleAction::Restart))
    } else if ctrl && alt {
        Some(DecodedInput::Action(ConsoleAction::Shutdown))
    } else {
        Some(DecodedInput::Sequence(b"\x1b[3~"))
    }
}

pub fn translate_scancode(scancode: u8, shift: bool, ctrl: bool) -> Option<u8> {
    let byte = match scancode {
        0x01 => 0x1B,
        0x02 => {
            if shift {
                b'!'
            } else {
                b'1'
            }
        }
        0x03 => {
            if shift {
                b'@'
            } else {
                b'2'
            }
        }
        0x04 => {
            if shift {
                b'#'
            } else {
                b'3'
            }
        }
        0x05 => {
            if shift {
                b'$'
            } else {
                b'4'
            }
        }
        0x06 => {
            if shift {
                b'%'
            } else {
                b'5'
            }
        }
        0x07 => {
            if shift {
                b'^'
            } else {
                b'6'
            }
        }
        0x08 => {
            if shift {
                b'&'
            } else {
                b'7'
            }
        }
        0x09 => {
            if shift {
                b'*'
            } else {
                b'8'
            }
        }
        0x0A => {
            if shift {
                b'('
            } else {
                b'9'
            }
        }
        0x0B => {
            if shift {
                b')'
            } else {
                b'0'
            }
        }
        0x0C => {
            if shift {
                b'_'
            } else {
                b'-'
            }
        }
        0x0D => {
            if shift {
                b'+'
            } else {
                b'='
            }
        }
        0x0E => 0x7F,
        0x0F => b'\t',
        0x10 => letter(b'q', shift),
        0x11 => letter(b'w', shift),
        0x12 => letter(b'e', shift),
        0x13 => letter(b'r', shift),
        0x14 => letter(b't', shift),
        0x15 => letter(b'y', shift),
        0x16 => letter(b'u', shift),
        0x17 => letter(b'i', shift),
        0x18 => letter(b'o', shift),
        0x19 => letter(b'p', shift),
        0x1A => {
            if shift {
                b'{'
            } else {
                b'['
            }
        }
        0x1B => {
            if shift {
                b'}'
            } else {
                b']'
            }
        }
        0x1C => b'\n',
        0x1E => letter(b'a', shift),
        0x1F => letter(b's', shift),
        0x20 => letter(b'd', shift),
        0x21 => letter(b'f', shift),
        0x22 => letter(b'g', shift),
        0x23 => letter(b'h', shift),
        0x24 => letter(b'j', shift),
        0x25 => letter(b'k', shift),
        0x26 => letter(b'l', shift),
        0x27 => {
            if shift {
                b':'
            } else {
                b';'
            }
        }
        0x28 => {
            if shift {
                b'"'
            } else {
                b'\''
            }
        }
        0x29 => {
            if shift {
                b'~'
            } else {
                b'`'
            }
        }
        0x2B => {
            if shift {
                b'|'
            } else {
                b'\\'
            }
        }
        0x2C => letter(b'z', shift),
        0x2D => letter(b'x', shift),
        0x2E => letter(b'c', shift),
        0x2F => letter(b'v', shift),
        0x30 => letter(b'b', shift),
        0x31 => letter(b'n', shift),
        0x32 => letter(b'm', shift),
        0x33 => {
            if shift {
                b'<'
            } else {
                b','
            }
        }
        0x34 => {
            if shift {
                b'>'
            } else {
                b'.'
            }
        }
        0x35 => {
            if shift {
                b'?'
            } else {
                b'/'
            }
        }
        0x39 => b' ',
        _ => return None,
    };

    if ctrl { ctrl_byte(byte) } else { Some(byte) }
}

fn letter(lower: u8, shift: bool) -> u8 {
    if shift {
        lower.to_ascii_uppercase()
    } else {
        lower
    }
}

fn ctrl_byte(byte: u8) -> Option<u8> {
    match byte.to_ascii_lowercase() {
        b'a'..=b'z' => Some(byte.to_ascii_lowercase() - b'a' + 1),
        b'[' => Some(0x1B),
        b'\\' => Some(0x1C),
        b']' => Some(0x1D),
        b'^' => Some(0x1E),
        b'_' => Some(0x1F),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) static I8042_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_decoder_state() {
        SHIFT_DOWN.store(false, Ordering::Release);
        CTRL_DOWN.store(false, Ordering::Release);
        ALT_DOWN.store(false, Ordering::Release);
        EXTENDED.store(false, Ordering::Release);
        EVDEV_EXTENDED.store(false, Ordering::Release);
        KEYBOARD_LEDS.store(0, Ordering::Release);
        BYTE_QUEUE.lock().clear();
        PENDING_KEYBOARD_SCANCODES.lock().clear();
    }

    #[test]
    fn translates_letters_and_shifted_symbols() {
        assert_eq!(translate_scancode(0x1E, false, false), Some(b'a'));
        assert_eq!(translate_scancode(0x1E, true, false), Some(b'A'));
        assert_eq!(translate_scancode(0x02, true, false), Some(b'!'));
    }

    #[test]
    fn translates_enter_backspace_and_ctrl_c() {
        assert_eq!(translate_scancode(0x1C, false, false), Some(b'\n'));
        assert_eq!(translate_scancode(0x0E, false, false), Some(0x7F));
        assert_eq!(translate_scancode(0x2E, false, true), Some(0x03));
    }

    #[test]
    fn ctrl_c_scancode_sequence_emits_linux_vintr_byte() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0x1D), None);
        assert_eq!(decode_scancode_input(0x2E), Some(DecodedInput::Byte(0x03)));
        assert_eq!(decode_scancode_input(0x9D), None);
        assert_eq!(decode_scancode_input(0x2E), Some(DecodedInput::Byte(b'c')));
    }

    #[test]
    fn ignores_controller_ack_bytes() {
        assert_eq!(decode_scancode(0xFA), None);
        assert_eq!(translate_scancode(0x1E, false, false), Some(b'a'));
    }

    #[test]
    fn decodes_extended_arrow_sequences() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x4B),
            Some(DecodedInput::Sequence(b"\x1b[D"))
        );
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x48),
            Some(DecodedInput::Sequence(b"\x1b[A"))
        );
    }

    #[test]
    fn num_lock_enables_every_keypad_digit_and_decimal() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0x45), None);
        assert_ne!(keyboard_leds() & (1 << LED_NUML), 0);

        for (scancode, expected) in [
            (0x47, b'7'),
            (0x48, b'8'),
            (0x49, b'9'),
            (0x4B, b'4'),
            (0x4C, b'5'),
            (0x4D, b'6'),
            (0x4F, b'1'),
            (0x50, b'2'),
            (0x51, b'3'),
            (0x52, b'0'),
            (0x53, b'.'),
        ] {
            assert_eq!(
                decode_scancode_input(scancode),
                Some(DecodedInput::Byte(expected))
            );
        }

        // The break code must not toggle the lock; the next make does.
        assert_eq!(decode_scancode_input(0xC5), None);
        assert_ne!(keyboard_leds() & (1 << LED_NUML), 0);
        assert_eq!(decode_scancode_input(0x45), None);
        assert_eq!(keyboard_leds() & (1 << LED_NUML), 0);
    }

    #[test]
    fn evdev_led_bitmap_is_translated_to_ps2_wire_order() {
        assert_eq!(evdev_leds_to_ps2(1 << LED_NUML), 0b010);
        assert_eq!(evdev_leds_to_ps2(1 << LED_CAPSL), 0b100);
        assert_eq!(evdev_leds_to_ps2(1 << LED_SCROLLL), 0b001);
        assert_eq!(evdev_leds_to_ps2(0b111), 0b111);
    }

    #[test]
    fn graphical_raw_keyboard_input_is_not_cooked_into_the_console() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(enqueue_decoded_if_cooked(0x1E, false), None);
        assert_eq!(
            enqueue_decoded_if_cooked(0x1E, true),
            Some(ConsoleInput::Byte(b'a'))
        );
    }

    #[test]
    fn hardirq_byte_ring_preserves_channel_and_order_without_allocating() {
        let queue = IrqByteQueue::new();
        assert!(queue.push(0x1e, false));
        assert!(queue.push(0x08, true));
        assert_eq!(queue.pop(), Some((0x1e, false)));
        assert_eq!(queue.pop(), Some((0x08, true)));
        assert_eq!(queue.pop(), None);

        for byte in 0..I8042_QUEUE_CAPACITY - 1 {
            assert!(queue.push(byte as u8, byte & 1 != 0));
        }
        assert!(!queue.push(0xff, false), "bounded IRQ ring must not grow");
    }

    #[test]
    fn keypad_without_num_lock_emits_linux_console_navigation_and_operators() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        for (scancode, expected) in [
            (0x47, b"\x1b[1~".as_slice()),
            (0x48, b"\x1b[A".as_slice()),
            (0x49, b"\x1b[5~".as_slice()),
            (0x4B, b"\x1b[D".as_slice()),
            (0x4C, b"\x1b[G".as_slice()),
            (0x4D, b"\x1b[C".as_slice()),
            (0x4F, b"\x1b[4~".as_slice()),
            (0x50, b"\x1b[B".as_slice()),
            (0x51, b"\x1b[6~".as_slice()),
            (0x52, b"\x1b[2~".as_slice()),
            (0x53, b"\x1b[3~".as_slice()),
        ] {
            assert_eq!(
                decode_scancode_input(scancode),
                Some(DecodedInput::Sequence(expected))
            );
        }
        assert_eq!(decode_scancode_input(0x37), Some(DecodedInput::Byte(b'*')));
        assert_eq!(decode_scancode_input(0x4A), Some(DecodedInput::Byte(b'-')));
        assert_eq!(decode_scancode_input(0x4E), Some(DecodedInput::Byte(b'+')));
    }

    #[test]
    fn extended_keypad_slash_and_enter_emit_bytes() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(decode_scancode_input(0x35), Some(DecodedInput::Byte(b'/')));
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(decode_scancode_input(0x1C), Some(DecodedInput::Byte(b'\n')));
    }

    #[test]
    fn ctrl_alt_delete_decodes_shutdown_action() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0x1D), None);
        assert_eq!(decode_scancode_input(0x38), None);
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x53),
            Some(DecodedInput::Action(ConsoleAction::Shutdown))
        );
    }

    #[test]
    fn ctrl_shift_delete_decodes_restart_action() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0x1D), None);
        assert_eq!(decode_scancode_input(0x2A), None);
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x53),
            Some(DecodedInput::Action(ConsoleAction::Restart))
        );
    }

    #[test]
    fn evdev_maps_base_scancodes_to_matching_keycodes() {
        // AT set-1 base range: keycode == scancode, high bit = release.
        assert_eq!(evdev_key_event(0x1E, false), Some((30, 1))); // KEY_A press
        assert_eq!(evdev_key_event(0x9E, false), Some((30, 0))); // KEY_A release
        assert_eq!(evdev_key_event(0x1C, false), Some((28, 1))); // KEY_ENTER
        assert_eq!(evdev_key_event(0x2A, false), Some((42, 1))); // KEY_LEFTSHIFT
        assert_eq!(evdev_key_event(0x39, false), Some((57, 1))); // KEY_SPACE
        assert_eq!(evdev_key_event(0x45, false), Some((69, 1))); // KEY_NUMLOCK
        assert_eq!(evdev_key_event(0x47, false), Some((71, 1))); // KEY_KP7
        assert_eq!(evdev_key_event(0xCF, false), Some((79, 0))); // KEY_KP1 release
        assert_eq!(evdev_key_event(0x52, false), Some((82, 1))); // KEY_KP0
        assert_eq!(evdev_key_event(0x53, false), Some((83, 1))); // KEY_KPDOT
    }

    #[test]
    fn evdev_maps_extended_scancodes_and_drops_unknowns() {
        assert_eq!(evdev_key_event(0x48, true), Some((103, 1))); // KEY_UP press
        assert_eq!(evdev_key_event(0xC8, true), Some((103, 0))); // KEY_UP release
        assert_eq!(evdev_key_event(0x1D, true), Some((97, 1))); // KEY_RIGHTCTRL
        assert_eq!(evdev_key_event(0x2A, true), None); // fake-shift filler, dropped
        assert_eq!(evdev_key_event(0x00, false), None); // out of range
    }

    #[test]
    fn ps2_mouse_packet_decodes_to_evdev_rel_and_buttons() {
        let _guard = I8042_TEST_LOCK.lock();
        use super::super::{
            BTN_LEFT, EV_KEY, EV_REL, EV_SYN, InputDev, REL_X, REL_Y, find_input_dev,
            input_register_device,
        };
        // Register the event1 pointer device (idempotent across parallel tests
        // would race on the shared 0xE002 id, so use it directly here — this is
        // the canonical mouse id the decoder targets).
        if find_input_dev(EVDEV_MOUSE_ID).is_none() {
            let _ = input_register_device(InputDev::new("test-mouse", EVDEV_MOUSE_ID));
        }
        let dev = find_input_dev(EVDEV_MOUSE_ID).unwrap();
        let _ = dev.drain_events();
        *MOUSE_STATE.lock() = MousePacket::new();

        // Left button held, move +5 in X and +3 in Y (PS/2 up-positive).
        // byte0: bit3 set (0x08) + left button (0x01) = 0x09.
        feed_mouse_byte(0x09);
        feed_mouse_byte(5);
        feed_mouse_byte(3);

        let events = dev.drain_events();
        // Expect: BTN_LEFT press, REL_X +5, REL_Y -3 (inverted), SYN.
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EV_KEY && e.code == BTN_LEFT && e.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EV_REL && e.code == REL_X && e.value == 5)
        );
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EV_REL && e.code == REL_Y && e.value == -3)
        );
        assert!(events.iter().any(|e| e.event_type == EV_SYN));
    }

    #[test]
    fn ps2_mouse_resyncs_on_missing_sync_bit() {
        let _guard = I8042_TEST_LOCK.lock();
        // A first packet byte without bit3 set is dropped (out-of-sync).
        *MOUSE_STATE.lock() = MousePacket::new();
        feed_mouse_byte(0x00); // no sync bit → ignored
        assert_eq!(MOUSE_STATE.lock().index, 0);
        feed_mouse_byte(0x08); // valid first byte
        assert_eq!(MOUSE_STATE.lock().index, 1);
    }

    #[test]
    fn plain_delete_still_decodes_escape_sequence() {
        let _guard = I8042_TEST_LOCK.lock();
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x53),
            Some(DecodedInput::Sequence(b"\x1b[3~"))
        );
    }
}
