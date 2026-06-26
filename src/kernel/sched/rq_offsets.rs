//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/rq-offsets.c
//! test-origin: linux:vendor/linux/kernel/sched/rq-offsets.c
//! Runqueue offset constants.
//!
//! Mirrors `vendor/linux/kernel/sched/rq-offsets.c`. Upstream emits offsets
//! for assembly and BPF consumers; Lupos exposes compile-time offsets for the
//! Rust `Rq` layout.

use core::mem::{offset_of, size_of};

use super::rq::Rq;

pub const RQ_CPU_OFFSET: usize = offset_of!(Rq, cpu);
pub const RQ_NR_RUNNING_OFFSET: usize = offset_of!(Rq, nr_running);
pub const RQ_CURRENT_OFFSET: usize = offset_of!(Rq, current);
pub const RQ_CLOCK_OFFSET: usize = offset_of!(Rq, clock);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rq_offsets_match_rq_layout() {
        assert_eq!(RQ_CPU_OFFSET, offset_of!(Rq, cpu));
        assert_eq!(RQ_NR_RUNNING_OFFSET, offset_of!(Rq, nr_running));
        assert_eq!(RQ_CURRENT_OFFSET, offset_of!(Rq, current));
        assert_eq!(RQ_CLOCK_OFFSET, offset_of!(Rq, clock));
        assert!(RQ_CPU_OFFSET < size_of::<Rq>());
        assert!(RQ_NR_RUNNING_OFFSET < size_of::<Rq>());
        assert!(RQ_CURRENT_OFFSET < size_of::<Rq>());
        assert!(RQ_CLOCK_OFFSET < size_of::<Rq>());
    }
}
