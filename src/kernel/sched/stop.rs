//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! Stop scheduling class — `sched_class_stop`.
//!
//! Used by per-CPU migration kthreads (Linux `migration/N`).  Always picked
//! before any other class when its task is runnable.  M31 wires the migration
//! kthread; until then this class is registered but never enqueued.

use crate::kernel::task::TaskStruct;

use super::class::{CLASS_PRIO_STOP, SchedClass};
use super::rq::Rq;

unsafe fn pick_next_task_stop(_rq: &mut Rq) -> *mut TaskStruct {
    core::ptr::null_mut()
}

pub static STOP_SCHED_CLASS: SchedClass = SchedClass {
    class_prio: CLASS_PRIO_STOP,
    _pad: [0; 7],
    enqueue_task: None,
    dequeue_task: None,
    yield_task: None,
    wakeup_preempt: None,
    pick_next_task: Some(pick_next_task_stop),
    put_prev_task: None,
    set_next_task: None,
    task_tick: None,
    task_fork: None,
    task_dead: None,
    switched_to: None,
    prio_changed: None,
    get_rr_interval: None,
    update_curr: None,
    select_task_rq: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_class_priority_is_highest() {
        assert_eq!(STOP_SCHED_CLASS.class_prio, CLASS_PRIO_STOP);
    }
}
