// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/pinctrl/pinctrl-state.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014648

use core::ffi::c_char;

/*
 * Each upstream macro expands to a C string-literal array, rather than naming
 * a pointer object.  Keep the Rust equivalents as NUL-terminated value arrays
 * so their extent and aggregate-initializer/indexing semantics remain visible.
 * A translated C use that had adjacent literals lowers the complete expanded
 * literal into one value array at that use; it does not pass a state pointer.
 */
pub const PINCTRL_STATE_DEFAULT: [c_char; 8] = [
    b'd' as c_char, b'e' as c_char, b'f' as c_char, b'a' as c_char,
    b'u' as c_char, b'l' as c_char, b't' as c_char, 0,
];

pub const PINCTRL_STATE_INIT: [c_char; 5] = [
    b'i' as c_char, b'n' as c_char, b'i' as c_char, b't' as c_char, 0,
];

pub const PINCTRL_STATE_IDLE: [c_char; 5] = [
    b'i' as c_char, b'd' as c_char, b'l' as c_char, b'e' as c_char, 0,
];

pub const PINCTRL_STATE_SLEEP: [c_char; 6] = [
    b's' as c_char, b'l' as c_char, b'e' as c_char, b'e' as c_char,
    b'p' as c_char, 0,
];
