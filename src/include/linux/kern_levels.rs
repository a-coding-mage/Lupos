// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/kern_levels.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014172

/*
 * These declarative macros retain the token-producing character of the Linux
 * definitions.  In particular, the severity prefixes remain string literals
 * suitable for direct concatenation with a printk format string.
 */
macro_rules! KERN_SOH {
    () => { "\u{0001}" };
}

pub const KERN_SOH_ASCII: u8 = 1;

macro_rules! KERN_EMERG {
    () => { concat!(KERN_SOH!(), "0") };
}
macro_rules! KERN_ALERT {
    () => { concat!(KERN_SOH!(), "1") };
}
macro_rules! KERN_CRIT {
    () => { concat!(KERN_SOH!(), "2") };
}
macro_rules! KERN_ERR {
    () => { concat!(KERN_SOH!(), "3") };
}
macro_rules! KERN_WARNING {
    () => { concat!(KERN_SOH!(), "4") };
}
macro_rules! KERN_NOTICE {
    () => { concat!(KERN_SOH!(), "5") };
}
macro_rules! KERN_INFO {
    () => { concat!(KERN_SOH!(), "6") };
}
macro_rules! KERN_DEBUG {
    () => { concat!(KERN_SOH!(), "7") };
}

macro_rules! KERN_DEFAULT {
    () => { "" };
}

macro_rules! KERN_CONT {
    () => { concat!(KERN_SOH!(), "c") };
}

pub const LOGLEVEL_SCHED: i32 = -2;
pub const LOGLEVEL_DEFAULT: i32 = -1;
pub const LOGLEVEL_EMERG: i32 = 0;
pub const LOGLEVEL_ALERT: i32 = 1;
pub const LOGLEVEL_CRIT: i32 = 2;
pub const LOGLEVEL_ERR: i32 = 3;
pub const LOGLEVEL_WARNING: i32 = 4;
pub const LOGLEVEL_NOTICE: i32 = 5;
pub const LOGLEVEL_INFO: i32 = 6;
pub const LOGLEVEL_DEBUG: i32 = 7;
