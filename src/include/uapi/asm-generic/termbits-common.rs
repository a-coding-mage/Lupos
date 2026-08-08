// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/termbits-common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016028

pub type cc_t = u8;
pub type speed_t = u32;

/* c_iflag bits */
pub const IGNBRK: i32 = 0x001;
pub const BRKINT: i32 = 0x002;
pub const IGNPAR: i32 = 0x004;
pub const PARMRK: i32 = 0x008;
pub const INPCK: i32 = 0x010;
pub const ISTRIP: i32 = 0x020;
pub const INLCR: i32 = 0x040;
pub const IGNCR: i32 = 0x080;
pub const ICRNL: i32 = 0x100;
pub const IXANY: i32 = 0x800;

/* c_oflag bits */
pub const OPOST: i32 = 0x01;
pub const OCRNL: i32 = 0x08;
pub const ONOCR: i32 = 0x10;
pub const ONLRET: i32 = 0x20;
pub const OFILL: i32 = 0x40;
pub const OFDEL: i32 = 0x80;

/* c_cflag bit meaning */
/* Common CBAUD rates */
pub const B0: i32 = 0x00000000;
pub const B50: i32 = 0x00000001;
pub const B75: i32 = 0x00000002;
pub const B110: i32 = 0x00000003;
pub const B134: i32 = 0x00000004;
pub const B150: i32 = 0x00000005;
pub const B200: i32 = 0x00000006;
pub const B300: i32 = 0x00000007;
pub const B600: i32 = 0x00000008;
pub const B1200: i32 = 0x00000009;
pub const B1800: i32 = 0x0000000a;
pub const B2400: i32 = 0x0000000b;
pub const B4800: i32 = 0x0000000c;
pub const B9600: i32 = 0x0000000d;
pub const B19200: i32 = 0x0000000e;
pub const B38400: i32 = 0x0000000f;
pub const EXTA: i32 = B19200;
pub const EXTB: i32 = B38400;

pub const ADDRB: i32 = 0x20000000;
pub const CMSPAR: i32 = 0x40000000;
pub const CRTSCTS: u32 = 0x80000000;

pub const IBSHIFT: i32 = 16;

/* tcflow() ACTION argument and TCXONC use these */
pub const TCOOFF: i32 = 0;
pub const TCOON: i32 = 1;
pub const TCIOFF: i32 = 2;
pub const TCION: i32 = 3;

/* tcflush() QUEUE_SELECTOR argument and TCFLSH use these */
pub const TCIFLUSH: i32 = 0;
pub const TCOFLUSH: i32 = 1;
pub const TCIOFLUSH: i32 = 2;
