//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/build_policy.c
//! test-origin: linux:vendor/linux/kernel/sched/build_policy.c
//! Scheduler policy build aggregation.
//!
//! Mirrors `vendor/linux/kernel/sched/build_policy.c`. Upstream uses this file
//! to assemble policy classes into the final scheduler build. Lupos exposes the
//! same policy ordering explicitly so tests can lock class precedence.

use super::class::{
    CLASS_PRIO_DL, CLASS_PRIO_FAIR, CLASS_PRIO_IDLE, CLASS_PRIO_RT, CLASS_PRIO_STOP,
};

pub const POLICY_BUILD_INPUTS: &[&str] = &[
    "vendor/linux/kernel/sched/idle.c",
    "vendor/linux/kernel/sched/fair.c",
    "vendor/linux/kernel/sched/rt.c",
    "vendor/linux/kernel/sched/deadline.c",
    "vendor/linux/kernel/sched/stop_task.c",
];

pub const fn sched_class_priority_order() -> [u8; 5] {
    [
        CLASS_PRIO_STOP,
        CLASS_PRIO_DL,
        CLASS_PRIO_RT,
        CLASS_PRIO_FAIR,
        CLASS_PRIO_IDLE,
    ]
}

pub fn policy_source_is_built(linux_path: &str) -> bool {
    POLICY_BUILD_INPUTS.contains(&linux_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_order_keeps_stop_above_deadline_above_rt() {
        assert_eq!(
            sched_class_priority_order(),
            [
                CLASS_PRIO_STOP,
                CLASS_PRIO_DL,
                CLASS_PRIO_RT,
                CLASS_PRIO_FAIR,
                CLASS_PRIO_IDLE
            ]
        );
    }

    #[test]
    fn policy_inputs_include_linux_stop_task() {
        assert!(policy_source_is_built(
            "vendor/linux/kernel/sched/stop_task.c"
        ));
    }
}
