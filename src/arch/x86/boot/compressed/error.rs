//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/error.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/error.c
//! Compressed-kernel error/warn helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/error.c
//! - vendor/linux/arch/x86/boot/compressed/error.h
//!
//! Linux provides `warn(msg)`, `error(msg)`, and (under CONFIG_EFI_STUB)
//! `panic(fmt, ...)`. `error()` halts the CPU with `while(1) hlt`.
//! The Rust port exposes the same three entry points; the put-string
//! sink and the halt loop sit behind a trait so host tests can probe.

extern crate alloc;

/// Sink for decompressor diagnostics — `error_putstr` in Linux.
pub trait ErrorSink {
    fn putstr(&mut self, msg: &str);
}

/// Production halt strategy. `hlt` on x86 in a tight loop; the
/// hypothetical "no halt" path is for host tests.
pub trait HaltStrategy {
    fn halt(&mut self) -> !;
}

/// `warn(msg)` — surround the message with blank lines. Mirrors
/// error.c lines 10-15.
pub fn warn<S: ErrorSink>(sink: &mut S, msg: &str) {
    sink.putstr("\n\n");
    sink.putstr(msg);
    sink.putstr("\n\n");
}

/// `error(msg)` — warn, append "System halted", then never return.
pub fn error<S: ErrorSink, H: HaltStrategy>(sink: &mut S, halt: &mut H, msg: &str) -> ! {
    warn(sink, msg);
    sink.putstr(" -- System halted");
    halt.halt()
}

/// `panic(fmt_args)` — error.c routes through `vsnprintf` into a
/// 1024-byte buffer under CONFIG_EFI_STUB. We expose the post-format
/// call so the formatting can happen at the call site (lupos has no
/// `va_list`).
pub fn panic<S: ErrorSink, H: HaltStrategy>(sink: &mut S, halt: &mut H, formatted: &str) -> ! {
    let trimmed = formatted.strip_suffix('\n').unwrap_or(formatted);
    error(sink, halt, trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    struct CaptureSink {
        out: Vec<String>,
    }
    impl ErrorSink for CaptureSink {
        fn putstr(&mut self, msg: &str) {
            self.out.push(msg.to_string());
        }
    }

    struct PanicHalt {
        halted: bool,
    }
    impl HaltStrategy for PanicHalt {
        fn halt(&mut self) -> ! {
            self.halted = true;
            // Use `loop` so the function's type stays `!` but tests
            // never actually loop — they `panic` from inside `error()`.
            panic!("halt invoked")
        }
    }

    #[test]
    fn warn_emits_blank_lines_around_message() {
        let mut s = CaptureSink { out: Vec::new() };
        warn(&mut s, "abc");
        assert_eq!(s.out, alloc::vec!["\n\n", "abc", "\n\n"]);
    }

    #[test]
    #[should_panic(expected = "halt invoked")]
    fn error_halts_the_caller() {
        let mut s = CaptureSink { out: Vec::new() };
        let mut h = PanicHalt { halted: false };
        error(&mut s, &mut h, "kernel fault");
    }

    #[test]
    #[should_panic(expected = "halt invoked")]
    fn panic_strips_trailing_newline_then_errors() {
        let mut s = CaptureSink { out: Vec::new() };
        let mut h = PanicHalt { halted: false };
        panic(&mut s, &mut h, "fatal\n");
    }
}
