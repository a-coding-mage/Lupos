//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_remote.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_remote.c
//! Remote (non-cpu) trace-buffer integration — used by hypervisor / vCPU
//! trace tap.
//!
//! Ref: vendor/linux/kernel/trace/trace_remote.c

use crate::kernel::trace::simple_ring_buffer::SimpleRing;

pub static REMOTE_TRACE: spin::Lazy<SimpleRing<u64>> = spin::Lazy::new(|| SimpleRing::new(1024));

pub fn push(ts: u64) -> bool {
    REMOTE_TRACE.push(ts)
}

pub fn drain_one() -> Option<u64> {
    REMOTE_TRACE.pop()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_drain() {
        push(0x1234);
        let v = drain_one();
        assert_eq!(v, Some(0x1234));
    }
}
