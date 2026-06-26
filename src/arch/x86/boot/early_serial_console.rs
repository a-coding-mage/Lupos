//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/early_serial_console.c
//! test-origin: linux:vendor/linux/arch/x86/boot/early_serial_console.c
//! Early 8250 serial-console setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/early_serial_console.c
//! - vendor/linux/arch/x86/boot/io.h
//! - vendor/linux/arch/x86/boot/tty.c

use super::cmdline::cmdline_find_option;
use super::tty::Uart;

pub const DEFAULT_SERIAL_PORT: u16 = 0x3f8;
pub const DEFAULT_BAUD: u32 = 9600;
pub const BASE_BAUD: u32 = 1_843_200 / 16;

pub const DLAB: u8 = 0x80;
pub const TXR: u16 = 0;
pub const RXR: u16 = 0;
pub const IER: u16 = 1;
pub const IIR: u16 = 2;
pub const FCR: u16 = 2;
pub const LCR: u16 = 3;
pub const MCR: u16 = 4;
pub const LSR: u16 = 5;
pub const MSR: u16 = 6;
pub const DLL: u16 = 0;
pub const DLH: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EarlySerialConfig {
    pub port: u16,
    pub baud: u32,
}

pub fn early_serial_init<U: Uart>(uart: &mut U, port: u16, baud: u32) -> EarlySerialConfig {
    let baud = if baud == 0 { DEFAULT_BAUD } else { baud };
    let divisor = BASE_BAUD / baud;

    uart.outb(port + LCR, 0x03);
    uart.outb(port + IER, 0);
    uart.outb(port + FCR, 0);
    uart.outb(port + MCR, 0x03);

    let c = uart.inb(port + LCR);
    uart.outb(port + LCR, c | DLAB);
    uart.outb(port + DLL, (divisor & 0xff) as u8);
    uart.outb(port + DLH, ((divisor >> 8) & 0xff) as u8);
    uart.outb(port + LCR, c & !DLAB);

    EarlySerialConfig { port, baud }
}

pub fn parse_earlyprintk(cmdline: &[u8]) -> Option<EarlySerialConfig> {
    let arg = cmdline_find_option(cmdline, "earlyprintk")?;
    parse_earlyprintk_arg(arg)
}

pub fn parse_earlyprintk_arg(arg: &[u8]) -> Option<EarlySerialConfig> {
    let mut baud = DEFAULT_BAUD;
    let mut pos = 0usize;
    let mut port = 0u16;

    if starts_with_at(arg, pos, b"serial") {
        port = DEFAULT_SERIAL_PORT;
        pos += 6;
    }

    if arg.get(pos) == Some(&b',') {
        pos += 1;
    }

    if pos == 7 && starts_with_at(arg, pos, b"0x") {
        let (parsed, used) = parse_u64_prefix(&arg[pos..], 16);
        if parsed == 0 || used == 0 {
            port = DEFAULT_SERIAL_PORT;
        } else {
            port = parsed as u16;
            pos += used;
        }
    } else if starts_with_at(arg, pos, b"ttyS") {
        let bases = [0x3f8u16, 0x2f8u16];
        let mut idx = 0usize;
        pos += 4;
        if arg.get(pos) == Some(&b'1') {
            idx = 1;
        }
        pos += 1;
        port = bases[idx];
    }

    if arg.get(pos) == Some(&b',') {
        pos += 1;
    }

    let (parsed_baud, used) = parse_u64_prefix(&arg[pos..], 0);
    if parsed_baud != 0 && used != 0 {
        baud = parsed_baud as u32;
    }

    (port != 0).then_some(EarlySerialConfig { port, baud })
}

pub fn parse_console_uart8250<U: Uart>(cmdline: &[u8], uart: &mut U) -> Option<EarlySerialConfig> {
    let optstr = cmdline_find_option(cmdline, "console")?;
    parse_console_uart8250_arg(optstr, uart)
}

pub fn parse_console_uart8250_arg<U: Uart>(
    optstr: &[u8],
    uart: &mut U,
) -> Option<EarlySerialConfig> {
    let mut pos;
    let port;

    if starts_with_at(optstr, 0, b"uart8250,io,") {
        pos = 12;
        let (parsed, used) = parse_u64_prefix(&optstr[pos..], 0);
        if parsed == 0 || used == 0 {
            return None;
        }
        port = parsed as u16;
        pos += used;
    } else if starts_with_at(optstr, 0, b"uart,io,") {
        pos = 8;
        let (parsed, used) = parse_u64_prefix(&optstr[pos..], 0);
        if parsed == 0 || used == 0 {
            return None;
        }
        port = parsed as u16;
        pos += used;
    } else {
        return None;
    }

    let baud = if optstr.get(pos) == Some(&b',') {
        let (parsed, used) = parse_u64_prefix(&optstr[pos + 1..], 0);
        if parsed != 0 && used != 0 {
            parsed as u32
        } else {
            DEFAULT_BAUD
        }
    } else {
        probe_baud(uart, port)
    };

    Some(EarlySerialConfig { port, baud })
}

pub fn console_init<U: Uart>(cmdline: &[u8], uart: &mut U) -> Option<EarlySerialConfig> {
    if let Some(config) = parse_earlyprintk(cmdline) {
        return Some(early_serial_init(uart, config.port, config.baud));
    }
    parse_console_uart8250(cmdline, uart)
        .map(|config| early_serial_init(uart, config.port, config.baud))
}

pub fn probe_baud<U: Uart>(uart: &mut U, port: u16) -> u32 {
    let lcr = uart.inb(port + LCR);
    uart.outb(port + LCR, lcr | DLAB);
    let dll = uart.inb(port + DLL);
    let dlh = uart.inb(port + DLH);
    uart.outb(port + LCR, lcr);

    let quot = ((dlh as u32) << 8) | dll as u32;
    if quot == 0 { 0 } else { BASE_BAUD / quot }
}

fn starts_with_at(buf: &[u8], pos: usize, needle: &[u8]) -> bool {
    buf.get(pos..)
        .is_some_and(|tail| tail.len() >= needle.len() && tail[..needle.len()] == *needle)
}

fn parse_u64_prefix(buf: &[u8], radix: u32) -> (u64, usize) {
    let mut pos = 0usize;
    let base = if radix == 0 {
        if starts_with_at(buf, 0, b"0x") || starts_with_at(buf, 0, b"0X") {
            pos = 2;
            16
        } else {
            10
        }
    } else {
        radix
    };

    if radix == 16 && (starts_with_at(buf, 0, b"0x") || starts_with_at(buf, 0, b"0X")) {
        pos = 2;
    }

    let digit_start = pos;
    let mut value = 0u64;
    while let Some(digit) = buf.get(pos).and_then(|b| digit_value(*b)) {
        if digit >= base {
            break;
        }
        value = value
            .saturating_mul(base as u64)
            .saturating_add(digit as u64);
        pos += 1;
    }

    if pos == digit_start {
        (0, 0)
    } else {
        (value, pos)
    }
}

fn digit_value(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a') as u32 + 10),
        b'A'..=b'F' => Some((b - b'A') as u32 + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[derive(Default)]
    struct StubUart {
        writes: RefCell<alloc::vec::Vec<(u16, u8)>>,
        lcr: u8,
        dll: u8,
        dlh: u8,
    }

    impl Uart for StubUart {
        fn inb(&self, port: u16) -> u8 {
            match port {
                p if p == DEFAULT_SERIAL_PORT + LCR => self.lcr,
                p if p == DEFAULT_SERIAL_PORT + DLL => self.dll,
                p if p == DEFAULT_SERIAL_PORT + DLH => self.dlh,
                _ => 0,
            }
        }

        fn outb(&mut self, port: u16, val: u8) {
            self.writes.borrow_mut().push((port, val));
        }
    }

    #[test]
    fn early_serial_init_programs_linux_8250_sequence() {
        let mut uart = StubUart {
            lcr: 0x03,
            ..Default::default()
        };

        let config = early_serial_init(&mut uart, DEFAULT_SERIAL_PORT, 115200);

        assert_eq!(
            config,
            EarlySerialConfig {
                port: DEFAULT_SERIAL_PORT,
                baud: 115200
            }
        );
        let writes = uart.writes.borrow();
        assert_eq!(writes[0], (DEFAULT_SERIAL_PORT + LCR, 0x03));
        assert_eq!(writes[1], (DEFAULT_SERIAL_PORT + IER, 0));
        assert_eq!(writes[2], (DEFAULT_SERIAL_PORT + FCR, 0));
        assert_eq!(writes[3], (DEFAULT_SERIAL_PORT + MCR, 0x03));
        assert_eq!(writes[4], (DEFAULT_SERIAL_PORT + LCR, 0x83));
        assert_eq!(writes[5], (DEFAULT_SERIAL_PORT + DLL, 1));
        assert_eq!(writes[6], (DEFAULT_SERIAL_PORT + DLH, 0));
        assert_eq!(writes[7], (DEFAULT_SERIAL_PORT + LCR, 0x03));
    }

    #[test]
    fn earlyprintk_parser_accepts_linux_serial_forms() {
        assert_eq!(
            parse_earlyprintk_arg(b"serial,0x2f8,115200"),
            Some(EarlySerialConfig {
                port: 0x2f8,
                baud: 115200
            })
        );
        assert_eq!(
            parse_earlyprintk_arg(b"serial,ttyS1,57600"),
            Some(EarlySerialConfig {
                port: 0x2f8,
                baud: 57600
            })
        );
        assert_eq!(
            parse_earlyprintk_arg(b"ttyS0,38400n8"),
            Some(EarlySerialConfig {
                port: 0x3f8,
                baud: 38400
            })
        );
    }

    #[test]
    fn earlyprintk_bare_serial_uses_linux_defaults() {
        assert_eq!(
            parse_earlyprintk_arg(b"serial"),
            Some(EarlySerialConfig {
                port: DEFAULT_SERIAL_PORT,
                baud: DEFAULT_BAUD
            })
        );
    }

    #[test]
    fn console_uart8250_parser_reads_explicit_baud() {
        let mut uart = StubUart::default();

        assert_eq!(
            parse_console_uart8250_arg(b"uart8250,io,0x3f8,115200n8", &mut uart),
            Some(EarlySerialConfig {
                port: DEFAULT_SERIAL_PORT,
                baud: 115200
            })
        );
    }

    #[test]
    fn console_uart8250_parser_probes_existing_divisor_without_baud() {
        let mut uart = StubUart {
            lcr: 0x03,
            dll: 12,
            ..Default::default()
        };

        assert_eq!(
            parse_console_uart8250_arg(b"uart,io,0x3f8", &mut uart),
            Some(EarlySerialConfig {
                port: DEFAULT_SERIAL_PORT,
                baud: 9600
            })
        );
        let writes = uart.writes.borrow();
        assert_eq!(writes[0], (DEFAULT_SERIAL_PORT + LCR, 0x83));
        assert_eq!(writes[1], (DEFAULT_SERIAL_PORT + LCR, 0x03));
    }

    #[test]
    fn console_init_prefers_earlyprintk_over_console() {
        let mut uart = StubUart {
            lcr: 0x03,
            ..Default::default()
        };
        let cmdline = b"console=uart8250,io,0x2f8,57600 earlyprintk=serial,0x3f8,115200\0";

        let config = console_init(cmdline, &mut uart);

        assert_eq!(
            config,
            Some(EarlySerialConfig {
                port: DEFAULT_SERIAL_PORT,
                baud: 115200
            })
        );
        assert_eq!(uart.writes.borrow()[5], (DEFAULT_SERIAL_PORT + DLL, 1));
    }
}
