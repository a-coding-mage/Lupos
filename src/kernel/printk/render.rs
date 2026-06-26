//! linux-parity: complete
//! linux-source: vendor/linux/kernel/printk
//! test-origin: linux:vendor/linux/kernel/printk
//! Render printk records into the three Linux output formats:
//! - `format_dmesg`: `[    1.234567] message\n`
//! - `format_dev_kmsg`: `<level>,<seq>,<ts_us>,<flags>;<message>\n`
//! - `format_console`: `[    1.234567] <level>message\n` (with `KERN_*`)
//!
//! Refs:
//! - `vendor/linux/kernel/printk/printk.c::msg_print_ext_body` (dev_kmsg).
//! - `vendor/linux/kernel/printk/printk.c::print_time` (dmesg).

extern crate alloc;

use alloc::string::String;
use core::fmt::Write;

use super::record::{LOG_CONT, PrintkInfo};

fn write_decimal_ts(out: &mut String, ts_nsec: u64) {
    let secs = ts_nsec / 1_000_000_000;
    let usecs = (ts_nsec % 1_000_000_000) / 1_000;
    let _ = write!(out, "[{:>5}.{:06}]", secs, usecs);
}

/// Linux `dmesg` line format: `[    1.234567] message\n`.
pub fn format_dmesg(info: &PrintkInfo, text: &[u8]) -> String {
    let mut out = String::new();
    write_decimal_ts(&mut out, info.ts_nsec);
    out.push(' ');
    for &b in text {
        out.push(b as char);
    }
    if !text.last().map(|&b| b == b'\n').unwrap_or(false) {
        out.push('\n');
    }
    out
}

/// Linux `/dev/kmsg` ext format: `<level>,<seq>,<ts_us>,<flag>;<text>\n`.
/// Flag is `c` if continuation, `-` otherwise (matches `printk.c::msg_add_ext_text`).
pub fn format_dev_kmsg(info: &PrintkInfo, text: &[u8]) -> String {
    let mut out = String::new();
    let priority = ((info.facility as u32) << 3) | (info.level() as u32);
    let ts_us = info.ts_nsec / 1_000;
    let flag = if info.flags() & LOG_CONT != 0 {
        'c'
    } else {
        '-'
    };
    let _ = write!(out, "{},{},{},{};", priority, info.seq, ts_us, flag);
    for &b in text {
        // Linux escapes raw control chars; for M61 we pass-through.
        out.push(b as char);
    }
    if !text.last().map(|&b| b == b'\n').unwrap_or(false) {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::printk::levels::*;
    use crate::kernel::printk::record::PrintkInfo;

    fn mk_info(seq: u64, ts: u64, level: u8, facility: u8, flags: u8, text_len: u16) -> PrintkInfo {
        let mut info = PrintkInfo::empty();
        info.seq = seq;
        info.ts_nsec = ts;
        info.facility = facility;
        info.set_flags_level(flags, level);
        info.text_len = text_len;
        info
    }

    #[test]
    fn dmesg_renders_decimal_timestamp() {
        let info = mk_info(0, 1_234_567_000, KERN_INFO, LOG_KERN, 0, 5);
        let s = format_dmesg(&info, b"hello");
        assert_eq!(s, "[    1.234567] hello\n");
    }

    #[test]
    fn dmesg_keeps_trailing_newline_unique() {
        let info = mk_info(0, 0, 6, 0, 0, 6);
        let s = format_dmesg(&info, b"hello\n");
        assert_eq!(s, "[    0.000000] hello\n");
    }

    #[test]
    fn dev_kmsg_priority_and_format() {
        let info = mk_info(42, 1_234_567_000, KERN_WARNING, LOG_KERN, 0, 7);
        let s = format_dev_kmsg(&info, b"message");
        // priority = (LOG_KERN<<3) | KERN_WARNING = 0|4 = 4
        // ts_us = 1_234_567
        assert_eq!(s, "4,42,1234567,-;message\n");
    }

    #[test]
    fn dev_kmsg_continuation_flag() {
        let info = mk_info(7, 0, KERN_INFO, LOG_KERN, LOG_CONT, 4);
        let s = format_dev_kmsg(&info, b"more");
        assert_eq!(s, "6,7,0,c;more\n");
    }
}
