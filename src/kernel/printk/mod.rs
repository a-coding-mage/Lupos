//! linux-parity: partial
//! linux-source: vendor/linux/kernel/printk
//! Linux-shaped printk surface (M61).
//!
//! Wraps the existing `src/log.rs` ring buffer with a Linux-format adapter:
//! - `record::PrintkInfo` and friends mirror `printk_ringbuffer.h`.
//! - `ringbuffer::PrintkRingbuffer` stores committed records in a global ring.
//! - `levels` parses `<n>` / `<facility.level>` prefixes.
//! - `render` produces dmesg / `/dev/kmsg` formats.
//!
//! `printk!()` is the public macro: it accepts a `KERN_*` level constant and
//! a format string, builds the text, then calls `printk_emit`.

extern crate alloc;

pub mod braille;
pub mod index;
pub mod levels;
pub mod log;
pub mod nbcon;
pub mod printk_safe;
pub mod record;
pub mod render;
pub mod ringbuffer;
pub mod sysctl;

use alloc::string::String;
use core::fmt::Write;

pub use crate::{log_debug, log_error, log_info, log_trace, log_warn};
pub use levels::{
    KERN_ALERT, KERN_CRIT, KERN_DEBUG, KERN_DEFAULT, KERN_EMERG, KERN_ERR, KERN_INFO, KERN_NOTICE,
    KERN_WARNING, LOG_KERN,
};
pub use record::{PrintkInfo, PrintkRecord};
pub use ringbuffer::PRINTK_RB;

/// Initialize the printk subsystem.  Idempotent.
pub fn init() {
    // Currently nothing to do — `PRINTK_RB` is a `static` initialised at compile time.
    // Future: console handover, deferred-print thread, etc.
}

/// Emit a record into the printk ring.  Parses any leading `<n>` prefix.
pub fn printk_emit(level: u8, facility: u8, fmt_args: core::fmt::Arguments<'_>) {
    let mut s = String::new();
    let _ = s.write_fmt(fmt_args);
    let bytes = s.as_bytes();
    let parsed = levels::parse_prefix(bytes);
    let (text, lvl, fac) = if parsed.consumed > 0 {
        (&bytes[parsed.consumed..], parsed.level, parsed.facility)
    } else {
        (bytes, level, facility)
    };
    let ts = current_ts_nsec();
    let _ = ringbuffer::PRINTK_RB.emit(ts, fac, lvl, 0, caller_id(), text);
}

#[inline]
fn current_ts_nsec() -> u64 {
    // TSC cycles since reset, treated as nanoseconds under the 1 GHz nominal
    // assumption (tsc_clocksource mult=1, shift=0).  Accurate calibration
    // arrives in M37; this is already sub-tick resolution and never stuck at 0.
    // Falls back to jiffies in test / non-x86 builds where RDTSC is unavailable.
    let tsc = crate::kernel::time::clocksource::read_tsc();
    if tsc != 0 {
        tsc
    } else {
        crate::kernel::time::jiffies::jiffies() as u64 * 1_000_000
    }
}

#[inline]
fn caller_id() -> u32 {
    // Linux high bit means "processor id".  We use 0 (BSP) until per-CPU is wired.
    0x8000_0000
}

/// `printk!(level, "fmt", args...)` — Linux-style emit.
#[macro_export]
macro_rules! printk {
    ($lvl:expr, $($arg:tt)*) => {
        $crate::kernel::printk::printk_emit(
            $lvl,
            $crate::kernel::printk::LOG_KERN,
            ::core::format_args!($($arg)*),
        )
    };
}
