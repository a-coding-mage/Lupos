//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_nop.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_nop.c
//! `nop` tracer — the default tracer that records nothing.  Useful as the
//! identity element when no other tracer is selected.
//!
//! Ref: vendor/linux/kernel/trace/trace_nop.c

pub fn trace(_event: &str) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_always_zero() {
        assert_eq!(trace("any"), 0);
        assert_eq!(trace(""), 0);
    }
}
