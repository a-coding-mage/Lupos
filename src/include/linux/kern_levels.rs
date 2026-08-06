// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/kern_levels.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014172

use core::ffi::{c_char, c_int};

/*
 * Each source macro below expands to a C string literal.  Preserve the
 * literal's static NUL-terminated character-array storage here; a translated
 * C-string use takes the corresponding thin `_PTR` view.  A source use of
 * `KERN_<LEVEL> "literal"` is translated with the matching
 * `kern_<level>_cstr!("literal")` macro below, which forms one static
 * NUL-terminated literal before producing its C-character pointer.
 */
pub static KERN_SOH: [c_char; 2] = [1, 0];
pub const KERN_SOH_PTR: *const c_char = KERN_SOH.as_ptr();
pub const KERN_SOH_ASCII: c_int = 1;

pub static KERN_EMERG: [c_char; 3] = [1, b'0' as c_char, 0];
pub const KERN_EMERG_PTR: *const c_char = KERN_EMERG.as_ptr();
pub static KERN_ALERT: [c_char; 3] = [1, b'1' as c_char, 0];
pub const KERN_ALERT_PTR: *const c_char = KERN_ALERT.as_ptr();
pub static KERN_CRIT: [c_char; 3] = [1, b'2' as c_char, 0];
pub const KERN_CRIT_PTR: *const c_char = KERN_CRIT.as_ptr();
pub static KERN_ERR: [c_char; 3] = [1, b'3' as c_char, 0];
pub const KERN_ERR_PTR: *const c_char = KERN_ERR.as_ptr();
pub static KERN_WARNING: [c_char; 3] = [1, b'4' as c_char, 0];
pub const KERN_WARNING_PTR: *const c_char = KERN_WARNING.as_ptr();
pub static KERN_NOTICE: [c_char; 3] = [1, b'5' as c_char, 0];
pub const KERN_NOTICE_PTR: *const c_char = KERN_NOTICE.as_ptr();
pub static KERN_INFO: [c_char; 3] = [1, b'6' as c_char, 0];
pub const KERN_INFO_PTR: *const c_char = KERN_INFO.as_ptr();
pub static KERN_DEBUG: [c_char; 3] = [1, b'7' as c_char, 0];
pub const KERN_DEBUG_PTR: *const c_char = KERN_DEBUG.as_ptr();

pub static KERN_DEFAULT: [c_char; 1] = [0];
pub const KERN_DEFAULT_PTR: *const c_char = KERN_DEFAULT.as_ptr();

pub static KERN_CONT: [c_char; 3] = [1, b'c' as c_char, 0];
pub const KERN_CONT_PTR: *const c_char = KERN_CONT.as_ptr();

/*
 * These macros replace only C adjacent-string-literal uses at translated call
 * sites.  `concat!` receives literal tokens, gives the joined storage static
 * duration, and appends the one terminator C contributes after concatenation.
 */
#[macro_export]
macro_rules! kern_soh_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_emerg_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "0", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_alert_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "1", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_crit_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "2", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_err_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "3", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_warning_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "4", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_notice_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "5", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_info_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "6", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_debug_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "7", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_default_cstr {
    ($($literal:tt)+) => {{ concat!($($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

#[macro_export]
macro_rules! kern_cont_cstr {
    ($($literal:tt)+) => {{ concat!("\x01", "c", $($literal)+, "\0").as_ptr().cast::<core::ffi::c_char>() }};
}

/* Integer equivalents of KERN_<LEVEL>. */
pub const LOGLEVEL_SCHED: c_int = -2;
pub const LOGLEVEL_DEFAULT: c_int = -1;
pub const LOGLEVEL_EMERG: c_int = 0;
pub const LOGLEVEL_ALERT: c_int = 1;
pub const LOGLEVEL_CRIT: c_int = 2;
pub const LOGLEVEL_ERR: c_int = 3;
pub const LOGLEVEL_WARNING: c_int = 4;
pub const LOGLEVEL_NOTICE: c_int = 5;
pub const LOGLEVEL_INFO: c_int = 6;
pub const LOGLEVEL_DEBUG: c_int = 7;
