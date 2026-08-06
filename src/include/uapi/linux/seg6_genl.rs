// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/seg6_genl.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016371

//! Segment Routing v6 generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

// C `"SEG6"` literal storage, including its terminating NUL.  Pointer-expression
// uses take `SEG6_GENL_NAME.as_ptr()`, as C array-to-pointer conversion does.
pub static SEG6_GENL_NAME: [c_char; 5] = [
    b'S' as c_char,
    b'E' as c_char,
    b'G' as c_char,
    b'6' as c_char,
    0,
];

/// Produces the aggregate initialization performed by `SEG6_GENL_NAME` in a
/// C character array: literal elements through the terminating NUL, then
/// zero-initialized remaining elements.  The selected `genl_family.name`
/// initializer uses `seg6_genl_name_array::<GENL_NAMSIZ>()`.
pub const fn seg6_genl_name_array<const N: usize>() -> [c_char; N] {
    let mut name = [0; N];

    if N > 0 {
        name[0] = b'S' as c_char;
    }
    if N > 1 {
        name[1] = b'E' as c_char;
    }
    if N > 2 {
        name[2] = b'G' as c_char;
    }
    if N > 3 {
        name[3] = b'6' as c_char;
    }

    name
}

pub const SEG6_GENL_VERSION: c_int = 0x1;

// Anonymous C enum enumerators are C `int` constant expressions.
pub const SEG6_ATTR_UNSPEC: c_int = 0;
pub const SEG6_ATTR_DST: c_int = 1;
pub const SEG6_ATTR_DSTLEN: c_int = 2;
pub const SEG6_ATTR_HMACKEYID: c_int = 3;
pub const SEG6_ATTR_SECRET: c_int = 4;
pub const SEG6_ATTR_SECRETLEN: c_int = 5;
pub const SEG6_ATTR_ALGID: c_int = 6;
pub const SEG6_ATTR_HMACINFO: c_int = 7;
pub const __SEG6_ATTR_MAX: c_int = 8;
pub const SEG6_ATTR_MAX: c_int = __SEG6_ATTR_MAX - 1;

// Anonymous C enum enumerators are C `int` constant expressions.
pub const SEG6_CMD_UNSPEC: c_int = 0;
pub const SEG6_CMD_SETHMAC: c_int = 1;
pub const SEG6_CMD_DUMPHMAC: c_int = 2;
pub const SEG6_CMD_SET_TUNSRC: c_int = 3;
pub const SEG6_CMD_GET_TUNSRC: c_int = 4;
pub const __SEG6_CMD_MAX: c_int = 5;
pub const SEG6_CMD_MAX: c_int = __SEG6_CMD_MAX - 1;
