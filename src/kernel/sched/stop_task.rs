//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/stop_task.c
//! test-origin: linux:vendor/linux/kernel/sched/stop_task.c
//! Stop scheduling class wrapper.
//!
//! Mirrors `vendor/linux/kernel/sched/stop_task.c`. The concrete stop class
//! implementation lives in `sched::stop`; this module keeps the Linux file
//! surface and helper names.

pub use super::stop::STOP_SCHED_CLASS;

pub fn stop_task_class_prio() -> u8 {
    STOP_SCHED_CLASS.class_prio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sched::class::CLASS_PRIO_STOP;

    #[test]
    fn stop_task_has_highest_class_priority() {
        assert_eq!(stop_task_class_prio(), CLASS_PRIO_STOP);
    }
}
