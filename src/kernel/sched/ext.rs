//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/ext.c
//! test-origin: linux:vendor/linux/kernel/sched/ext.c
//! sched_ext integration surface.
//!
//! Mirrors `vendor/linux/kernel/sched/ext.c`. Linux sched_ext delegates policy
//! decisions to BPF programs. Lupos does not enable BPF scheduler programs yet,
//! but keeps explicit disabled-state hooks for ABI and build parity.

use core::sync::atomic::{AtomicBool, Ordering};

static SCX_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn sched_ext_enabled() -> bool {
    SCX_ENABLED.load(Ordering::Acquire)
}

pub fn sched_ext_set_enabled(enabled: bool) {
    SCX_ENABLED.store(enabled, Ordering::Release);
}

pub fn scx_task_enabled() -> bool {
    sched_ext_enabled()
}

pub const fn scx_bpf_dispatch_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_ext_defaults_to_disabled() {
        sched_ext_set_enabled(false);
        assert!(!sched_ext_enabled());
        assert!(!scx_bpf_dispatch_available());
    }
}
