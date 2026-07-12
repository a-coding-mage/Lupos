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
use core::ffi::c_char;
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

/// Register printk entry points used by vendor-built modules.
pub fn register_module_exports() {
    if crate::kernel::module::find_symbol("_printk").is_none() {
        crate::kernel::module::export_symbol("_printk", linux_printk as usize, false);
    }
}

/// x86-64 C-variadic trampoline for Linux `_printk(const char *fmt, ...)`.
#[unsafe(naked)]
#[unsafe(export_name = "_printk")]
pub unsafe extern "C" fn linux_printk() {
    core::arch::naked_asm!(
        "sub rsp, 40",
        "mov qword ptr [rsp], rsi",
        "mov qword ptr [rsp + 8], rdx",
        "mov qword ptr [rsp + 16], rcx",
        "mov qword ptr [rsp + 24], r8",
        "mov qword ptr [rsp + 32], r9",
        "lea rsi, [rsp]",
        "lea rdx, [rsp + 48]",
        "call {helper}",
        "add rsp, 40",
        "ret",
        helper = sym linux_printk_helper,
    );
}

#[inline(never)]
unsafe extern "C" fn linux_printk_helper(
    fmt: *const c_char,
    register_args: *const usize,
    stack_args: *const usize,
) -> i32 {
    let mut message_buf = [0u8; log::MSG_CAP];
    let message_len = unsafe {
        crate::linux_driver_abi::base::printf::vscnprintf_n(
            message_buf.as_mut_ptr(),
            message_buf.len(),
            fmt,
            register_args,
            5,
            stack_args,
        )
    };
    let parsed = levels::parse_prefix(&message_buf[..message_len]);
    let message = core::str::from_utf8(&message_buf[parsed.consumed..message_len]).unwrap_or("");
    let message = message.strip_suffix('\n').unwrap_or(message);
    let level = match parsed.level {
        0..=3 => log::Level::Error,
        4 => log::Level::Warn,
        _ => log::Level::Info,
    };
    log::_log(level, "", format_args!("{message}"));
    message_len.min(i32::MAX as usize) as i32
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
