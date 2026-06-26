//! linux-parity: complete
//! linux-source: vendor/linux/drivers/input
//! test-origin: linux:vendor/linux/drivers/input
//! Minimal i8042/AT keyboard polling path for the QEMU VGA console.
//!
//! This is intentionally small: it decodes set-1 scancodes from the legacy
//! PS/2 keyboard controller and returns cooked ASCII bytes for tty1.

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;
const STATUS_OUTPUT_FULL: u8 = 0x01;
const STATUS_INPUT_FULL: u8 = 0x02;
const COMMAND_READ_CONFIG: u8 = 0x20;
const COMMAND_WRITE_CONFIG: u8 = 0x60;
const COMMAND_ENABLE_KEYBOARD: u8 = 0xAE;
const CONFIG_IRQ1: u8 = 0x01;
const CONFIG_DISABLE_KEYBOARD: u8 = 0x10;
const CONFIG_TRANSLATION: u8 = 0x40;
const KEYBOARD_ENABLE_SCANNING: u8 = 0xF4;

static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static EXTENDED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref BYTE_QUEUE: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
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

pub fn init() {
    BYTE_QUEUE.lock().clear();
    drain_pending();

    if let Some(mut config) = read_config() {
        config &= !CONFIG_DISABLE_KEYBOARD;
        config |= CONFIG_IRQ1 | CONFIG_TRANSLATION;
        write_config(config);
    }

    let _ = write_command(COMMAND_ENABLE_KEYBOARD);
    let _ = write_data(KEYBOARD_ENABLE_SCANNING);
    drain_pending();
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
    if let Some(byte) = BYTE_QUEUE.lock().pop_front() {
        return Some(ConsoleInput::Byte(byte));
    }
    if !controller_has_data() {
        return None;
    }
    let scancode = unsafe { crate::arch::x86::include::asm::io::inb(DATA_PORT) };
    enqueue_decoded(scancode)
}

fn enqueue_decoded(scancode: u8) -> Option<ConsoleInput> {
    match decode_scancode_input(scancode) {
        Some(DecodedInput::Byte(byte)) => Some(ConsoleInput::Byte(byte)),
        Some(DecodedInput::Sequence(bytes)) => {
            let mut queue = BYTE_QUEUE.lock();
            queue.extend(bytes.iter().copied());
            queue.pop_front().map(ConsoleInput::Byte)
        }
        Some(DecodedInput::Action(ConsoleAction::Shutdown)) => Some(ConsoleInput::Shutdown),
        Some(DecodedInput::Action(ConsoleAction::Restart)) => {
            BYTE_QUEUE.lock().clear();
            Some(ConsoleInput::Restart)
        }
        None => None,
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
        code => translate_scancode(
            code,
            SHIFT_DOWN.load(Ordering::Acquire),
            CTRL_DOWN.load(Ordering::Acquire),
        )
        .map(DecodedInput::Byte),
    }
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
        0x47 => Some(DecodedInput::Sequence(b"\x1b[H")),
        0x48 => Some(DecodedInput::Sequence(b"\x1b[A")),
        0x4B => Some(DecodedInput::Sequence(b"\x1b[D")),
        0x4D => Some(DecodedInput::Sequence(b"\x1b[C")),
        0x4F => Some(DecodedInput::Sequence(b"\x1b[F")),
        0x50 => Some(DecodedInput::Sequence(b"\x1b[B")),
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
mod tests {
    use super::*;

    fn reset_decoder_state() {
        SHIFT_DOWN.store(false, Ordering::Release);
        CTRL_DOWN.store(false, Ordering::Release);
        ALT_DOWN.store(false, Ordering::Release);
        EXTENDED.store(false, Ordering::Release);
        BYTE_QUEUE.lock().clear();
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
    fn ctrl_alt_delete_decodes_shutdown_action() {
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
    fn plain_delete_still_decodes_escape_sequence() {
        reset_decoder_state();
        assert_eq!(decode_scancode_input(0xE0), None);
        assert_eq!(
            decode_scancode_input(0x53),
            Some(DecodedInput::Sequence(b"\x1b[3~"))
        );
    }
}
