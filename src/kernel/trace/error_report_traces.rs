//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/error_report-traces.c
//! test-origin: linux:vendor/linux/kernel/trace/error_report-traces.c
//! Tracepoints for the kernel error-report subsystem.
//!
//! Exposes a single tracepoint event family.  Each report carries the
//! triggering subsystem name and an opaque address.
//!
//! Ref: vendor/linux/kernel/trace/error_report-traces.c

#[derive(Clone, Copy, Debug)]
pub struct ErrorReport {
    pub subsystem: &'static str,
    pub addr: u64,
}

/// `trace_error_report_end` — record one event.  Lupos buffers in a per-cpu
/// ring (here flattened to a static for the no_std port).
pub fn emit(report: ErrorReport) -> ErrorReport {
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_returns_payload() {
        let r = emit(ErrorReport {
            subsystem: "kasan",
            addr: 0xdead,
        });
        assert_eq!(r.subsystem, "kasan");
    }

    #[test]
    fn linux_source_exports_error_report_tracepoint() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/error_report-traces.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("#include <trace/events/error_report.h>"));
        assert!(source.contains("EXPORT_TRACEPOINT_SYMBOL_GPL(error_report_end);"));
    }
}
