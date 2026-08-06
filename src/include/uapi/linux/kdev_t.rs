// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/kdev_t.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016214

/*
 * Some programs want their definitions of MAJOR and MINOR and MKDEV from the
 * kernel sources. These must be the externally visible ones.
 *
 * These remain expression macros: their operands determine the integer width
 * and signedness, as in the UAPI definitions. No Rust device-number wrapper
 * or narrowing conversion is introduced here.
 */
macro_rules! MAJOR {
    ($dev:expr) => {
        (($dev) >> 8)
    };
}
pub(crate) use MAJOR;

macro_rules! MINOR {
    ($dev:expr) => {
        (($dev) & 0xff)
    };
}
pub(crate) use MINOR;

macro_rules! MKDEV {
    ($ma:expr, $mi:expr) => {
        (($ma) << 8 | ($mi))
    };
}
pub(crate) use MKDEV;
