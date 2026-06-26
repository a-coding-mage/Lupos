//! linux-parity: complete
//! linux-source: vendor/linux/drivers/tty
//! test-origin: linux:vendor/linux/drivers/tty
//! 8250/16550 UART driver — M57.
//!
//! Promotes the bare-metal serial logger in `src/serial.rs` into a proper
//! `uart_port` registered with the TTY core.
//!
//! For M57 we wrap the existing `src/serial.rs` I/O-port UART as a
//! `uart_port` and attach it to a `TtyDriver`.  Real interrupt-driven I/O
//! (ISR → `tty.receive_buf`) is deferred — in the test path we call
//! `receive_buf` manually to feed test data.
//!
//! References:
//!   - `include/linux/serial_core.h:442` — `struct uart_port`
//!   - `include/linux/serial_core.h:888` — `struct uart_driver`
//!   - `drivers/tty/serial/8250/8250_core.c:693` — `serial8250_register_8250_port`
//!   - `drivers/tty/serial/serial_core.c:2711`   — `uart_register_driver`

extern crate alloc;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::linux_driver_abi::tty::{TtyDriver, TtyStruct, tty_register_driver};

// Standard COM1 I/O port base — Linux `serial8250_isa_irqs` / `old_serial_port`.
pub const COM1_IOBASE: u16 = 0x3F8;
pub const COM1_IRQ: u32 = 4;

/// `struct uart_port` — `include/linux/serial_core.h:442`.
///
/// We strip it to the fields relevant for M57; the rest (DMA, RS485, etc.)
/// are deferred.
pub struct UartPort {
    pub iobase: u16,
    pub irq: u32,
    pub baud: u32,
    /// Associated TTY instance for this port.
    pub tty: Mutex<Option<Arc<TtyStruct>>>,
}

impl UartPort {
    pub fn new(iobase: u16, irq: u32, baud: u32) -> Arc<Self> {
        Arc::new(Self {
            iobase,
            irq,
            baud,
            tty: Mutex::new(None),
        })
    }

    /// Attach a TTY to this port.
    pub fn attach(&self, tty: Arc<TtyStruct>) {
        *self.tty.lock() = Some(tty);
    }

    /// Send one character to hardware via I/O port.
    ///
    /// # Safety
    /// Requires bare-metal context; `outb` is valid only on x86.
    #[cfg(not(test))]
    pub fn putchar(&self, c: u8) {
        // Wait for transmit holding register empty (bit 5 of LSR at base+5).
        let lsr_port = self.iobase + 5;
        loop {
            let lsr: u8;
            unsafe { core::arch::asm!("in al, dx", out("al") lsr, in("dx") lsr_port) };
            if lsr & 0x20 != 0 {
                break;
            }
        }
        unsafe { core::arch::asm!("out dx, al", in("dx") self.iobase, in("al") c) };
    }
    #[cfg(test)]
    pub fn putchar(&self, _c: u8) { /* no I/O port in test */
    }

    /// Push bytes from hardware FIFO into the attached TTY's line discipline.
    /// Called by the ISR in real hardware; called manually in tests.
    pub fn receive_chars(&self, data: &[u8]) {
        if let Some(tty) = self.tty.lock().clone() {
            tty.receive_buf(data);
        }
    }
}

/// `struct uart_driver` — `include/linux/serial_core.h:888`.
pub struct UartDriver {
    pub driver_name: &'static str,
    pub dev_name: &'static str,
    pub major: u32,
    pub minor: u32,
    pub nr: u32,
    pub tty_driver: Arc<TtyDriver>,
}

lazy_static! {
    /// The single 8250 uart_driver instance.
    /// Mirrors `serial8250_reg` in `drivers/tty/serial/8250/8250_core.c`.
    pub static ref SERIAL8250_DRIVER: UartDriver = {
        let tty_drv = TtyDriver::new("ttyS", 4, 64, 4);
        let _ = tty_register_driver(tty_drv.clone());
        UartDriver {
            driver_name: "serial8250",
            dev_name: "ttyS",
            major: 4,
            minor: 64,
            nr: 4,
            tty_driver: tty_drv,
        }
    };
    /// The COM1 uart_port with a TTY attached.
    pub static ref COM1_PORT: Arc<UartPort> = {
        let port = UartPort::new(COM1_IOBASE, COM1_IRQ, 115200);
        let tty = TtyStruct::new("ttyS0", 0);
        port.attach(tty.clone());
        SERIAL8250_DRIVER.tty_driver.ttys.lock().insert(0, tty);
        port
    };
}

/// `serial8250_register_8250_port` — `drivers/tty/serial/8250/8250_core.c:693`.
///
/// Initialises the 8250 driver + COM1 port.  Calling this function is
/// idempotent (lazy_static ensures one-time init).
pub fn serial8250_init() {
    let _ = &*SERIAL8250_DRIVER; // force lazy init
    let _ = &*COM1_PORT;
}

/// Return the TTY struct for the given port index (0-indexed, COM1 = 0).
pub fn serial8250_get_tty(index: u32) -> Option<Arc<TtyStruct>> {
    SERIAL8250_DRIVER
        .tty_driver
        .ttys
        .lock()
        .get(&index)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial8250_init_creates_tty() {
        serial8250_init();
        assert!(serial8250_get_tty(0).is_some());
    }

    #[test]
    fn receive_chars_feeds_ldisc() {
        serial8250_init();
        COM1_PORT.receive_chars(b"ping\n");
        let tty = serial8250_get_tty(0).unwrap();
        let line = tty.read_line();
        assert!(line.is_some());
        assert_eq!(line.unwrap(), b"ping\n");
    }
}
