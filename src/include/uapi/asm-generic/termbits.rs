// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/asm-generic/termbits.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016029

pub use super::termbits_common::{cc_t, speed_t};

#[allow(non_camel_case_types)]
pub type tcflag_t = u32;

pub const NCCS: i32 = 19;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct termios {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_line: cc_t,
    pub c_cc: [cc_t; NCCS as usize],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct termios2 {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_line: cc_t,
    pub c_cc: [cc_t; NCCS as usize],
    pub c_ispeed: speed_t,
    pub c_ospeed: speed_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ktermios {
    pub c_iflag: tcflag_t,
    pub c_oflag: tcflag_t,
    pub c_cflag: tcflag_t,
    pub c_lflag: tcflag_t,
    pub c_line: cc_t,
    pub c_cc: [cc_t; NCCS as usize],
    pub c_ispeed: speed_t,
    pub c_ospeed: speed_t,
}

// c_cc characters
pub const VINTR: i32 = 0;
pub const VQUIT: i32 = 1;
pub const VERASE: i32 = 2;
pub const VKILL: i32 = 3;
pub const VEOF: i32 = 4;
pub const VTIME: i32 = 5;
pub const VMIN: i32 = 6;
pub const VSWTC: i32 = 7;
pub const VSTART: i32 = 8;
pub const VSTOP: i32 = 9;
pub const VSUSP: i32 = 10;
pub const VEOL: i32 = 11;
pub const VREPRINT: i32 = 12;
pub const VDISCARD: i32 = 13;
pub const VWERASE: i32 = 14;
pub const VLNEXT: i32 = 15;
pub const VEOL2: i32 = 16;

// c_iflag bits
pub const IUCLC: i32 = 0x0200;
pub const IXON: i32 = 0x0400;
pub const IXOFF: i32 = 0x1000;
pub const IMAXBEL: i32 = 0x2000;
pub const IUTF8: i32 = 0x4000;

// c_oflag bits
pub const OLCUC: i32 = 0x00002;
pub const ONLCR: i32 = 0x00004;
pub const NLDLY: i32 = 0x00100;
pub const NL0: i32 = 0x00000;
pub const NL1: i32 = 0x00100;
pub const CRDLY: i32 = 0x00600;
pub const CR0: i32 = 0x00000;
pub const CR1: i32 = 0x00200;
pub const CR2: i32 = 0x00400;
pub const CR3: i32 = 0x00600;
pub const TABDLY: i32 = 0x01800;
pub const TAB0: i32 = 0x00000;
pub const TAB1: i32 = 0x00800;
pub const TAB2: i32 = 0x01000;
pub const TAB3: i32 = 0x01800;
pub const XTABS: i32 = 0x01800;
pub const BSDLY: i32 = 0x02000;
pub const BS0: i32 = 0x00000;
pub const BS1: i32 = 0x02000;
pub const VTDLY: i32 = 0x04000;
pub const VT0: i32 = 0x00000;
pub const VT1: i32 = 0x04000;
pub const FFDLY: i32 = 0x08000;
pub const FF0: i32 = 0x00000;
pub const FF1: i32 = 0x08000;

// c_cflag bit meaning
pub const CBAUD: i32 = 0x0000_100f;
pub const CSIZE: i32 = 0x0000_0030;
pub const CS5: i32 = 0x0000_0000;
pub const CS6: i32 = 0x0000_0010;
pub const CS7: i32 = 0x0000_0020;
pub const CS8: i32 = 0x0000_0030;
pub const CSTOPB: i32 = 0x0000_0040;
pub const CREAD: i32 = 0x0000_0080;
pub const PARENB: i32 = 0x0000_0100;
pub const PARODD: i32 = 0x0000_0200;
pub const HUPCL: i32 = 0x0000_0400;
pub const CLOCAL: i32 = 0x0000_0800;
pub const CBAUDEX: i32 = 0x0000_1000;
pub const BOTHER: i32 = 0x0000_1000;
pub const B57600: i32 = 0x0000_1001;
pub const B115200: i32 = 0x0000_1002;
pub const B230400: i32 = 0x0000_1003;
pub const B460800: i32 = 0x0000_1004;
pub const B500000: i32 = 0x0000_1005;
pub const B576000: i32 = 0x0000_1006;
pub const B921600: i32 = 0x0000_1007;
pub const B1000000: i32 = 0x0000_1008;
pub const B1152000: i32 = 0x0000_1009;
pub const B1500000: i32 = 0x0000_100a;
pub const B2000000: i32 = 0x0000_100b;
pub const B2500000: i32 = 0x0000_100c;
pub const B3000000: i32 = 0x0000_100d;
pub const B3500000: i32 = 0x0000_100e;
pub const B4000000: i32 = 0x0000_100f;
pub const CIBAUD: i32 = 0x100f_0000;

// c_lflag bits
pub const ISIG: i32 = 0x00001;
pub const ICANON: i32 = 0x00002;
pub const XCASE: i32 = 0x00004;
pub const ECHO: i32 = 0x00008;
pub const ECHOE: i32 = 0x00010;
pub const ECHOK: i32 = 0x00020;
pub const ECHONL: i32 = 0x00040;
pub const NOFLSH: i32 = 0x00080;
pub const TOSTOP: i32 = 0x00100;
pub const ECHOCTL: i32 = 0x00200;
pub const ECHOPRT: i32 = 0x00400;
pub const ECHOKE: i32 = 0x00800;
pub const FLUSHO: i32 = 0x01000;
pub const PENDIN: i32 = 0x04000;
pub const IEXTEN: i32 = 0x08000;
pub const EXTPROC: i32 = 0x10000;

// tcsetattr uses these
pub const TCSANOW: i32 = 0;
pub const TCSADRAIN: i32 = 1;
pub const TCSAFLUSH: i32 = 2;
