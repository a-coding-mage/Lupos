// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/termbits-common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016028

pub type cc_t = u8;
pub type speed_t = u32;

// c_iflag bits
pub const IGNBRK: i32 = 0x001; // Ignore break condition
pub const BRKINT: i32 = 0x002; // Signal interrupt on break
pub const IGNPAR: i32 = 0x004; // Ignore characters with parity errors
pub const PARMRK: i32 = 0x008; // Mark parity and framing errors
pub const INPCK: i32 = 0x010; // Enable input parity check
pub const ISTRIP: i32 = 0x020; // Strip 8th bit off characters
pub const INLCR: i32 = 0x040; // Map NL to CR on input
pub const IGNCR: i32 = 0x080; // Ignore CR
pub const ICRNL: i32 = 0x100; // Map CR to NL on input
pub const IXANY: i32 = 0x800; // Any character will restart after stop

// c_oflag bits
pub const OPOST: i32 = 0x01; // Perform output processing
pub const OCRNL: i32 = 0x08;
pub const ONOCR: i32 = 0x10;
pub const ONLRET: i32 = 0x20;
pub const OFILL: i32 = 0x40;
pub const OFDEL: i32 = 0x80;

// c_cflag bit meaning
// Common CBAUD rates
pub const B0: i32 = 0x0000_0000; // hang up
pub const B50: i32 = 0x0000_0001;
pub const B75: i32 = 0x0000_0002;
pub const B110: i32 = 0x0000_0003;
pub const B134: i32 = 0x0000_0004;
pub const B150: i32 = 0x0000_0005;
pub const B200: i32 = 0x0000_0006;
pub const B300: i32 = 0x0000_0007;
pub const B600: i32 = 0x0000_0008;
pub const B1200: i32 = 0x0000_0009;
pub const B1800: i32 = 0x0000_000a;
pub const B2400: i32 = 0x0000_000b;
pub const B4800: i32 = 0x0000_000c;
pub const B9600: i32 = 0x0000_000d;
pub const B19200: i32 = 0x0000_000e;
pub const B38400: i32 = 0x0000_000f;
pub const EXTA: i32 = B19200;
pub const EXTB: i32 = B38400;

pub const ADDRB: i32 = 0x2000_0000; // address bit
pub const CMSPAR: i32 = 0x4000_0000; // mark or space (stick) parity
pub const CRTSCTS: u32 = 0x8000_0000; // flow control

pub const IBSHIFT: i32 = 16; // Shift from CBAUD to CIBAUD

// tcflow() ACTION argument and TCXONC use these
pub const TCOOFF: i32 = 0; // Suspend output
pub const TCOON: i32 = 1; // Restart suspended output
pub const TCIOFF: i32 = 2; // Send a STOP character
pub const TCION: i32 = 3; // Send a START character

// tcflush() QUEUE_SELECTOR argument and TCFLSH use these
pub const TCIFLUSH: i32 = 0; // Discard data received but not yet read
pub const TCOFLUSH: i32 = 1; // Discard data written but not yet sent
pub const TCIOFLUSH: i32 = 2; // Discard all pending data
