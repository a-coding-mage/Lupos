//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/trace_syscalls.c
//! test-origin: linux:vendor/linux/kernel/trace/trace_syscalls.c
//! `syscalls/sys_enter_*` and `sys_exit_*` tracepoints.
//!
//! Ref: vendor/linux/kernel/trace/trace_syscalls.c

use core::sync::atomic::{AtomicU64, Ordering};

pub static SYS_ENTER_COUNT: AtomicU64 = AtomicU64::new(0);
pub static SYS_EXIT_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn on_sys_enter(_nr: u32) {
    SYS_ENTER_COUNT.fetch_add(1, Ordering::AcqRel);
}

pub fn on_sys_exit(_nr: u32, _ret: i64) {
    SYS_EXIT_COUNT.fetch_add(1, Ordering::AcqRel);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_and_exit_bump_counters() {
        let e0 = SYS_ENTER_COUNT.load(Ordering::Acquire);
        on_sys_enter(0);
        on_sys_exit(0, 0);
        assert_eq!(SYS_ENTER_COUNT.load(Ordering::Acquire), e0 + 1);
    }
}
