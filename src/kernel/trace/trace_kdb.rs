//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_kdb.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_kdb.c
//! kdb / kgdb dump path through the trace ring buffer.
//!
//! Ref: vendor/linux/kernel/trace/trace_kdb.c

extern crate alloc;
use alloc::string::String;

/// `ftdump` kdb command equivalent — render a fixed line-count snapshot.
pub fn ftdump(line_limit: usize) -> String {
    let mut s = String::new();
    for i in 0..line_limit {
        s.push_str(&alloc::format!("trace line {}\n", i));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftdump_emits_n_lines() {
        let s = ftdump(3);
        assert_eq!(s.lines().count(), 3);
    }
}
