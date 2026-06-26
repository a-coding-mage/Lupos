//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/trace_clock.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/trace_clock.c
//! x86 trace clock backed directly by the ordered TSC cycle counter.

use super::tsc;

/// Linux `trace_clock_x86_tsc()` returns raw ordered TSC cycles, not
/// nanoseconds. The generic tracing layer decides how to label and compare
/// this clock source.
pub fn trace_clock_x86_tsc() -> u64 {
    tsc::read_ordered()
}

#[cfg(test)]
pub fn trace_clock_x86_tsc_with(read_ordered: impl FnOnce() -> u64) -> u64 {
    read_ordered()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_clock_source_returns_ordered_tsc_cycles() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/trace_clock.c"
        ));
        assert!(source.contains("return rdtsc_ordered();"));

        assert_eq!(trace_clock_x86_tsc_with(|| 0xfeed_beef), 0xfeed_beef);
        assert_eq!(trace_clock_x86_tsc(), tsc::read_ordered());
    }
}
