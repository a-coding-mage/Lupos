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
use core::ffi::{c_char, c_void};
use core::fmt::Write;

pub use crate::{log_debug, log_error, log_info, log_trace, log_warn};
pub use levels::{
    KERN_ALERT, KERN_CRIT, KERN_DEBUG, KERN_DEFAULT, KERN_EMERG, KERN_ERR, KERN_INFO, KERN_NOTICE,
    KERN_WARNING, LOG_KERN,
};
pub use record::{PrintkInfo, PrintkRecord};
pub use ringbuffer::PRINTK_RB;

/// `oops_in_progress` - `vendor/linux/kernel/printk/printk.c`.
static mut LINUX_OOPS_IN_PROGRESS: i32 = 0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if crate::kernel::module::find_symbol(name).is_none() {
        crate::kernel::module::export_symbol(name, addr, gpl_only);
    }
}

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
    if crate::kernel::module::find_symbol("vprintk").is_none() {
        crate::kernel::module::export_symbol("vprintk", linux_vprintk as usize, false);
    }
    if crate::kernel::module::find_symbol("dump_stack").is_none() {
        crate::kernel::module::export_symbol("dump_stack", linux_dump_stack as usize, false);
    }
    if crate::kernel::module::find_symbol("oops_in_progress").is_none() {
        crate::kernel::module::export_symbol(
            "oops_in_progress",
            core::ptr::addr_of_mut!(LINUX_OOPS_IN_PROGRESS) as usize,
            false,
        );
    }
    if crate::kernel::module::find_symbol("panic").is_none() {
        crate::kernel::module::export_symbol("panic", linux_panic as usize, false);
    }
    if crate::kernel::module::find_symbol("__printk_ratelimit").is_none() {
        crate::kernel::module::export_symbol(
            "__printk_ratelimit",
            linux___printk_ratelimit as usize,
            false,
        );
    }
    export_symbol_once("console_list_lock", linux_console_list_lock as usize, false);
    export_symbol_once(
        "console_list_unlock",
        linux_console_list_unlock as usize,
        false,
    );
    export_symbol_once("register_console", linux_register_console as usize, false);
    export_symbol_once(
        "unregister_console",
        linux_unregister_console as usize,
        false,
    );
    export_symbol_once(
        "nbcon_enter_unsafe",
        linux_nbcon_enter_unsafe as usize,
        true,
    );
    export_symbol_once("nbcon_exit_unsafe", linux_nbcon_exit_unsafe as usize, true);
}

/// `console_list_lock()` — `vendor/linux/kernel/printk/printk.c:247`.
pub unsafe extern "C" fn linux_console_list_lock() {}

/// `console_list_unlock()` — `vendor/linux/kernel/printk/printk.c:272`.
pub unsafe extern "C" fn linux_console_list_unlock() {}

/// `register_console()` — `vendor/linux/kernel/printk/printk.c:4060`.
pub unsafe extern "C" fn linux_register_console(_console: *mut c_void) {}

/// `unregister_console()` — `vendor/linux/kernel/printk/printk.c:4325`.
pub unsafe extern "C" fn linux_unregister_console(_console: *mut c_void) -> i32 {
    0
}

/// `nbcon_enter_unsafe()` — `vendor/linux/kernel/printk/nbcon.c:885`.
pub unsafe extern "C" fn linux_nbcon_enter_unsafe(_wctxt: *mut c_void) -> bool {
    false
}

/// `nbcon_exit_unsafe()` — `vendor/linux/kernel/printk/nbcon.c:909`.
pub unsafe extern "C" fn linux_nbcon_exit_unsafe(_wctxt: *mut c_void) -> bool {
    false
}

/// `dump_stack` - `vendor/linux/lib/dump_stack.c`.
pub unsafe extern "C" fn linux_dump_stack() {
    log_warn!("dump_stack", "Linux module requested stack dump");
}

/// `panic` - `vendor/linux/kernel/panic.c`.
pub unsafe extern "C" fn linux_panic(_fmt: *const c_char) -> ! {
    log_error!("panic", "Linux module called panic()");
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// `__printk_ratelimit` - `vendor/linux/kernel/printk/printk.c`.
pub unsafe extern "C" fn linux___printk_ratelimit(_func: *const c_char) -> i32 {
    1
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

/// `vprintk` - `vendor/linux/kernel/printk/printk_safe.c:75`.
pub unsafe extern "C" fn linux_vprintk(_fmt: *const c_char, _args: *mut c_void) -> i32 {
    0
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

#[cfg(test)]
mod tests {
    #[test]
    fn vprintk_export_tracks_vendor_printk_safe() {
        let source = include_str!("../../../vendor/linux/kernel/printk/printk_safe.c");
        assert!(source.contains("EXPORT_SYMBOL(vprintk);"));
        super::register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("vprintk"),
            Some(super::linux_vprintk as usize)
        );
    }

    #[test]
    fn netconsole_printk_exports_track_vendor_sources() {
        let printk = include_str!("../../../vendor/linux/kernel/printk/printk.c");
        let nbcon = include_str!("../../../vendor/linux/kernel/printk/nbcon.c");

        assert!(printk.contains("EXPORT_SYMBOL(console_list_lock);"));
        assert!(printk.contains("EXPORT_SYMBOL(console_list_unlock);"));
        assert!(printk.contains("EXPORT_SYMBOL(register_console);"));
        assert!(printk.contains("EXPORT_SYMBOL(unregister_console);"));
        assert!(nbcon.contains("EXPORT_SYMBOL_GPL(nbcon_enter_unsafe);"));
        assert!(nbcon.contains("EXPORT_SYMBOL_GPL(nbcon_exit_unsafe);"));

        super::register_module_exports();
        for (name, addr, gpl_only) in [
            (
                "console_list_lock",
                super::linux_console_list_lock as usize,
                false,
            ),
            (
                "console_list_unlock",
                super::linux_console_list_unlock as usize,
                false,
            ),
            (
                "register_console",
                super::linux_register_console as usize,
                false,
            ),
            (
                "unregister_console",
                super::linux_unregister_console as usize,
                false,
            ),
            (
                "nbcon_enter_unsafe",
                super::linux_nbcon_enter_unsafe as usize,
                true,
            ),
            (
                "nbcon_exit_unsafe",
                super::linux_nbcon_exit_unsafe as usize,
                true,
            ),
        ] {
            assert_eq!(crate::kernel::module::find_symbol(name), Some(addr));
            assert_eq!(
                crate::kernel::module::find_symbol_gpl_only(name),
                Some(gpl_only)
            );
        }
    }
}
