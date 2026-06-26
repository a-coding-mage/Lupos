//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_stack.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_stack.c
//! Per-cpu max-stack-depth tracer.
//!
//! Ref: vendor/linux/kernel/trace/trace_stack.c

use core::sync::atomic::{AtomicUsize, Ordering};

pub static MAX_STACK_BYTES: AtomicUsize = AtomicUsize::new(0);

pub fn observe(depth: usize) {
    let cur = MAX_STACK_BYTES.load(Ordering::Acquire);
    if depth > cur {
        MAX_STACK_BYTES.store(depth, Ordering::Release);
    }
}

pub fn reset() {
    MAX_STACK_BYTES.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_tracks_maximum() {
        reset();
        observe(2048);
        observe(8192);
        observe(4096);
        assert_eq!(MAX_STACK_BYTES.load(Ordering::Acquire), 8192);
    }
}
