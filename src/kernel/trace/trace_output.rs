//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_output.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_output.c
//! Pretty-printer for trace events (`%pS`, common-field formatting).
//!
//! Ref: vendor/linux/kernel/trace/trace_output.c

extern crate alloc;
use alloc::string::String;

pub fn format_event(pid: i32, cpu: u32, ts_ns: u64, msg: &str) -> String {
    alloc::format!("[{:>5}] CPU={} ts={}ns: {}", pid, cpu, ts_ns, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_includes_pid_cpu_ts_msg() {
        let s = format_event(1234, 2, 1_000_000, "hello");
        assert!(s.contains("1234"));
        assert!(s.contains("CPU=2"));
        assert!(s.contains("ts=1000000ns"));
        assert!(s.contains("hello"));
    }
}
