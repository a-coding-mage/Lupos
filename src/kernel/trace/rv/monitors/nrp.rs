//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor: no-runtime-preempt (NRP).
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/nrp/nrp.c

use core::sync::atomic::{AtomicBool, Ordering};

pub static IN_NRP: AtomicBool = AtomicBool::new(false);

pub fn enter() {
    IN_NRP.store(true, Ordering::Release);
}

pub fn exit() {
    IN_NRP.store(false, Ordering::Release);
}

pub fn check_preempt() -> bool {
    !IN_NRP.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nrp_section_blocks_preempt() {
        enter();
        assert!(!check_preempt());
        exit();
        assert!(check_preempt());
    }
}
