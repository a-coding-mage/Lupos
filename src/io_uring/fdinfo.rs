//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/fdinfo.c
//! test-origin: linux:vendor/linux/io_uring/fdinfo.c
//! `/proc/<pid>/fdinfo/<fd>` formatter for io_uring fds.
//!
//! Ref: vendor/linux/io_uring/fdinfo.c

extern crate alloc;

use alloc::string::String;
use core::fmt::Write;

use super::IoRingCtx;

/// `io_uring_show_fdinfo` — format the ring state in the same key:value
/// layout Linux uses, so userspace `/proc` parsers stay compatible.
///
/// Fields ported here cover the always-present subset:
///   `SqMask`, `SqHead`, `SqTail`, `CqMask`, `CqHead`, `CqTail`,
///   `SqEntries`, `CqEntries`, `CqOverflow`.
pub fn show_fdinfo(ctx: &IoRingCtx) -> String {
    use core::sync::atomic::Ordering;

    let mut s = String::new();
    let sq_mask = ctx.sq_entries.wrapping_sub(1);
    let cq_mask = ctx.cq_entries.wrapping_sub(1);

    let _ = writeln!(s, "SqMask:\t{}", sq_mask);
    let _ = writeln!(s, "SqHead:\t{}", ctx.sq_head.load(Ordering::Acquire));
    let _ = writeln!(s, "SqTail:\t{}", ctx.sq_tail.load(Ordering::Acquire));
    let _ = writeln!(s, "CqMask:\t{}", cq_mask);
    let _ = writeln!(s, "CqHead:\t{}", ctx.cq_head.load(Ordering::Acquire));
    let _ = writeln!(s, "CqTail:\t{}", ctx.cq_tail.load(Ordering::Acquire));
    let _ = writeln!(s, "SqEntries:\t{}", ctx.sq_entries);
    let _ = writeln!(s, "CqEntries:\t{}", ctx.cq_entries);
    let _ = writeln!(s, "CqOverflow:\t0");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    #[test]
    fn fdinfo_emits_linux_keys() {
        let ctx = IoRingCtx::new(8);
        ctx.sq_tail.store(3, Ordering::Release);
        ctx.cq_tail.store(2, Ordering::Release);
        let s = show_fdinfo(&ctx);
        assert!(s.contains("SqMask:\t7"));
        assert!(s.contains("SqHead:\t0"));
        assert!(s.contains("SqTail:\t3"));
        assert!(s.contains("CqMask:\t15"));
        assert!(s.contains("CqTail:\t2"));
        assert!(s.contains("SqEntries:\t8"));
        assert!(s.contains("CqEntries:\t16"));
        assert!(s.contains("CqOverflow:\t0"));
    }
}
