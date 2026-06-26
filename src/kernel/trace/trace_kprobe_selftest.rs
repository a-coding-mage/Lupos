//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_kprobe_selftest.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_kprobe_selftest.c
//! Kprobe trace selftest target function.

pub const LINUX_SOURCE: &str = "vendor/linux/kernel/trace/trace_kprobe_selftest.c";

pub const fn kprobe_trace_selftest_target(
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    a6: i32,
) -> i32 {
    a1 + a2 + a3 + a4 + a5 + a6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kprobe_selftest_target_matches_linux_sum() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/trace_kprobe_selftest.c"
        ));
        assert!(source.contains("#include \"trace_kprobe_selftest.h\""));
        assert!(source.contains("int kprobe_trace_selftest_target"));
        assert!(source.contains("return a1 + a2 + a3 + a4 + a5 + a6;"));
        assert_eq!(kprobe_trace_selftest_target(1, 2, 3, 4, 5, 6), 21);
        assert_eq!(kprobe_trace_selftest_target(-1, 2, -3, 4, -5, 6), 3);
    }
}
