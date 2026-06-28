//! linux-parity: complete
//! linux-source: vendor/linux/drivers/tty/serial/serial_core.c
//! test-origin: linux:vendor/linux/drivers/tty/serial/serial_core.c
/// Minimal 16550 UART serial driver for COM1 (I/O port 0x3F8).
///
/// Replaces the `uart_16550` crate with raw port I/O so we can run in
/// 32-bit protected mode.  Used for early kernel logging and for
/// communicating test results to the QEMU host via `-serial file:...`.
///
/// Ref: https://wiki.osdev.org/Serial_Ports
///      https://en.wikibooks.org/wiki/Serial_Programming/8250_UART_Programming
extern crate alloc;

use core::fmt;
use spin::Mutex;

/// COM1 base I/O port address.
const COM1: u16 = 0x3F8;
const SERIAL_QUEUE_CAP: usize = 128 * 1024;
const SERIAL_INPUT_QUEUE_CAP: usize = 4096;
/// 16550A transmit FIFO depth enabled by `init()`'s FCR write (0xC7).  Linux's
/// `serial8250_tx_chars()` (vendor/linux/drivers/tty/serial/8250/8250_port.c)
/// writes up to `tx_loadsz` bytes back-to-back once the FIFO has signaled
/// empty, without re-checking LSR between bytes — it trusts the FIFO has room
/// for that many right after an empty signal. `flush_budget()` mirrors that.
const TX_FIFO_DEPTH: usize = 16;

struct ByteRing<const CAP: usize> {
    buf: [u8; CAP],
    head: usize,
    len: usize,
}

impl<const CAP: usize> ByteRing<CAP> {
    const fn new() -> Self {
        Self {
            buf: [0; CAP],
            head: 0,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    fn push_drop_oldest(&mut self, byte: u8) {
        if CAP == 0 {
            return;
        }
        if self.len == CAP {
            self.buf[self.head] = byte;
            self.head = (self.head + 1) % CAP;
            return;
        }

        let tail = (self.head + self.len) % CAP;
        self.buf[tail] = byte;
        self.len += 1;
    }

    fn pop_front(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }

        let byte = self.buf[self.head];
        self.head = (self.head + 1) % CAP;
        self.len -= 1;
        Some(byte)
    }
}

struct SerialPort {
    base: u16,
}

impl SerialPort {
    const fn new(base: u16) -> Self {
        Self { base }
    }

    /// Initialize the UART: 115200 baud, 8N1, FIFO enabled.
    ///
    /// Divisor = 1: 1.8432 MHz / 16 / 1 = 115200 baud.
    /// Linux's serial console default; at 38400 (divisor=3) each ~60-char
    /// printk line takes ~15ms, making kernel init appear ~1 second slow.
    #[cfg(not(test))]
    fn init(&self) {
        unsafe {
            use crate::arch::x86::include::asm::io::{inb, outb};

            outb(self.base + 1, 0x00); // Disable all interrupts
            outb(self.base + 3, 0x80); // Enable DLAB (set baud rate divisor)
            outb(self.base + 0, 0x01); // Divisor low byte: 115200 baud
            outb(self.base + 1, 0x00); // Divisor high byte
            outb(self.base + 3, 0x03); // 8 bits, no parity, one stop bit (8N1)
            outb(self.base + 2, 0xC7); // Enable FIFO, clear, 14-byte threshold

            let _ = inb(self.base + 5); // Read LSR to clear any pending state
        }
    }

    #[cfg(test)]
    fn init(&self) {}

    /// Send a single byte, blocking until the transmitter is ready.
    ///
    /// This is the polling-console idiom Linux reserves for
    /// `CONFIG_CONSOLE_POLL` / early-console putchar (`wait_for_xmitr()` in
    /// 8250_port.c), used there only because the caller has no maintenance
    /// loop left to retry from (KDB, panic, pre-scheduler boot prints). Used
    /// here by `flush_all_blocking()`. Do NOT use this from a budgeted
    /// maintenance pass that also needs to keep polling RX — see
    /// `write_byte_unchecked()` / `tx_ready()` below for that path.
    #[cfg(not(test))]
    fn write_byte(&self, byte: u8) {
        unsafe {
            use crate::arch::x86::include::asm::io::{inb, outb};

            // Spin until bit 5 (THR empty) of Line Status Register is set.
            while inb(self.base + 5) & 0x20 == 0 {}
            outb(self.base, byte);
        }
    }

    #[cfg(test)]
    fn write_byte(&self, byte: u8) {
        test_capture::push(byte);
    }

    /// Non-blocking check of LSR bit 5 (THRE): true once the FIFO has fully
    /// drained and is ready to accept a fresh burst. Never spins.
    #[cfg(not(test))]
    fn tx_ready(&self) -> bool {
        unsafe {
            use crate::arch::x86::include::asm::io::inb;
            inb(self.base + 5) & 0x20 != 0
        }
    }

    #[cfg(test)]
    fn tx_ready(&self) -> bool {
        true
    }

    /// Write one byte without checking readiness. Only safe to call as part
    /// of a burst bounded by `TX_FIFO_DEPTH` immediately after `tx_ready()`
    /// confirmed the FIFO is empty (mirrors Linux's `serial8250_tx_chars()`,
    /// which writes `tx_loadsz` bytes per service without re-polling LSR).
    #[cfg(not(test))]
    fn write_byte_unchecked(&self, byte: u8) {
        unsafe {
            use crate::arch::x86::include::asm::io::outb;
            outb(self.base, byte);
        }
    }

    #[cfg(test)]
    fn write_byte_unchecked(&self, byte: u8) {
        test_capture::push(byte);
    }

    /// Read one byte if the UART receiver FIFO has data.
    fn try_read_byte(&self) -> Option<u8> {
        unsafe {
            use crate::arch::x86::include::asm::io::inb;

            if inb(self.base + 5) & 0x01 == 0 {
                return None;
            }
            Some(inb(self.base))
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1));
static SERIAL_QUEUE: Mutex<ByteRing<SERIAL_QUEUE_CAP>> = Mutex::new(ByteRing::new());
static SERIAL_INPUT_QUEUE: Mutex<ByteRing<SERIAL_INPUT_QUEUE_CAP>> = Mutex::new(ByteRing::new());

/// Initialize COM1. Safe to call multiple times.
pub fn init() {
    SERIAL1.lock().init();
}

/// Queue a raw byte slice for COM1.
///
/// This is used by early console devices (e.g. `/dev/console`) and must not
/// allocate, assume UTF-8, or spin on a disconnected serial sink.
pub fn write_bytes(bytes: &[u8]) {
    enqueue_bytes(bytes);
    let _ = flush_budget(SERIAL_QUEUE_CAP);
}

/// Queue raw bytes for later serial transmission.
///
/// The queue stores the CRLF-expanded stream so budgeted flushing can write a
/// simple byte at a time without remembering terminal translation state.
pub fn enqueue_bytes(bytes: &[u8]) {
    let mut queue = SERIAL_QUEUE.lock();
    for &byte in bytes {
        if byte == b'\n' {
            enqueue_one(&mut queue, b'\r');
        }
        enqueue_one(&mut queue, byte);
    }
}

fn enqueue_one<const CAP: usize>(queue: &mut ByteRing<CAP>, byte: u8) {
    queue.push_drop_oldest(byte);
}

/// Flush at most `budget` queued bytes to COM1 without blocking.
///
/// Mirrors Linux's IRQ-driven `serial8250_tx_chars()`: a burst only starts
/// once the FIFO has signaled empty (`tx_ready()`), and once started, up to
/// `TX_FIFO_DEPTH` bytes go out back-to-back without re-checking LSR. If the
/// FIFO isn't ready, this returns immediately instead of spinning.
///
/// A prior version called the spin-until-ready `write_byte()` here, which
/// could busy-wait for the full character time on every one of up to 4096
/// bytes with zero RX polling interleaved. Callers that poll console RX in
/// the same maintenance pass (`console_read()` in src/init/rootfs.rs) would
/// then miss incoming bytes during that window — the root cause of dropped
/// leading characters on scripted/pasted input.
pub fn flush_budget(budget: usize) -> usize {
    let mut drained = 0usize;
    while drained < budget {
        if !SERIAL1.lock().tx_ready() {
            break;
        }
        let burst = core::cmp::min(TX_FIFO_DEPTH, budget - drained);
        let mut wrote = 0usize;
        while wrote < burst {
            let Some(byte) = SERIAL_QUEUE.lock().pop_front() else {
                break;
            };
            SERIAL1.lock().write_byte_unchecked(byte);
            wrote += 1;
        }
        drained += wrote;
        if wrote == 0 {
            break;
        }
    }
    drained
}

/// Flush every queued byte synchronously, blocking on the hardware like
/// Linux's polling console writer (`wait_for_xmitr()` in 8250_port.c). Only
/// used by shutdown/panic paths with no maintenance loop left to retry from,
/// so spinning here is correct — unlike the budgeted path above, which must
/// stay non-blocking to keep RX polling responsive.
pub fn flush_all_blocking() {
    loop {
        let Some(byte) = SERIAL_QUEUE.lock().pop_front() else {
            break;
        };
        SERIAL1.lock().write_byte(byte);
    }
}

pub fn queued_len() -> usize {
    SERIAL_QUEUE.lock().len()
}

fn enqueue_input_byte(byte: u8) {
    let mut queue = SERIAL_INPUT_QUEUE.lock();
    queue.push_drop_oldest(byte);
}

#[cfg(not(test))]
pub fn poll_input_budget(budget: usize) -> usize {
    let mut drained = 0usize;
    while drained < budget {
        let Some(byte) = SERIAL1.lock().try_read_byte() else {
            break;
        };
        enqueue_input_byte(byte);
        drained += 1;
    }
    drained
}

#[cfg(test)]
pub fn poll_input_budget(_budget: usize) -> usize {
    0
}

/// Read a raw byte from COM1 if QEMU/the UART has delivered input.
///
/// This is used by the login console path so the login gate is driven by
/// serial input rather than by an in-kernel transcript.
#[cfg(not(test))]
pub fn try_read_byte() -> Option<u8> {
    if let Some(byte) = SERIAL_INPUT_QUEUE.lock().pop_front() {
        return Some(byte);
    }
    let _ = poll_input_budget(64);
    SERIAL_INPUT_QUEUE.lock().pop_front()
}

#[cfg(test)]
pub fn try_read_byte() -> Option<u8> {
    SERIAL_INPUT_QUEUE.lock().pop_front()
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    use fmt::Write;

    struct QueuedSerialWriter;

    impl fmt::Write for QueuedSerialWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            enqueue_bytes(s.as_bytes());
            Ok(())
        }
    }

    let _ = QueuedSerialWriter.write_fmt(args);
    let _ = flush_budget(SERIAL_QUEUE_CAP);
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::linux_driver_abi::tty::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => { $crate::linux_driver_abi::tty::serial_print!("\n") };
    ($fmt:expr) => { $crate::linux_driver_abi::tty::serial_print!(concat!($fmt, "\n")) };
    ($fmt:expr, $($arg:tt)*) => { $crate::linux_driver_abi::tty::serial_print!(concat!($fmt, "\n"), $($arg)*) };
}

#[cfg(test)]
mod test_capture {
    use alloc::vec::Vec;
    use lazy_static::lazy_static;
    use spin::Mutex;

    lazy_static! {
        static ref CAPTURED: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    }

    pub fn push(byte: u8) {
        CAPTURED.lock().push(byte);
    }

    pub fn clear() {
        CAPTURED.lock().clear();
    }

    pub fn bytes() -> Vec<u8> {
        CAPTURED.lock().clone()
    }
}

#[cfg(test)]
pub fn clear_capture_for_tests() {
    SERIAL_QUEUE.lock().clear();
    SERIAL_INPUT_QUEUE.lock().clear();
    test_capture::clear();
}

#[cfg(test)]
pub fn captured_bytes_for_tests() -> alloc::vec::Vec<u8> {
    test_capture::bytes()
}

#[cfg(test)]
pub fn push_input_for_tests(bytes: &[u8]) {
    for &byte in bytes {
        enqueue_input_byte(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_is_nonblocking_until_budget_flush() {
        clear_capture_for_tests();
        enqueue_bytes(b"a\nb");
        assert_eq!(queued_len(), 4);
        assert!(captured_bytes_for_tests().is_empty());

        assert_eq!(flush_budget(2), 2);
        assert_eq!(captured_bytes_for_tests(), b"a\r");
        assert_eq!(queued_len(), 2);

        flush_all_blocking();
        assert_eq!(captured_bytes_for_tests(), b"a\r\nb");
        assert_eq!(queued_len(), 0);
    }

    #[test]
    fn input_queue_buffers_serial_bytes_for_later_reads() {
        clear_capture_for_tests();
        push_input_for_tests(b"abcdef");

        assert_eq!(try_read_byte(), Some(b'a'));
        assert_eq!(try_read_byte(), Some(b'b'));
        assert_eq!(poll_input_budget(64), 0);
    }
}
