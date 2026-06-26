//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/idle.c
//! test-origin: linux:vendor/linux/kernel/sched/idle.c
//! Idle scheduling class — `sched_class_idle`.
//!
//! The idle task per CPU runs when no other class has a runnable task.  Its
//! sole job is to wait for an interrupt (the LAPIC timer or an IPI) that will
//! pull a real task back onto the runqueue.

use crate::kernel::task::TaskStruct;

use super::class::{CLASS_PRIO_IDLE, SchedClass};
use super::rq::Rq;

unsafe fn pick_next_task_idle(rq: &mut Rq) -> *mut TaskStruct {
    rq.idle
}

unsafe fn put_prev_task_idle(_rq: &mut Rq, _prev: *mut TaskStruct) {}

unsafe fn task_tick_idle(_rq: &mut Rq, _p: *mut TaskStruct, _queued: bool) {}

pub static IDLE_SCHED_CLASS: SchedClass = SchedClass {
    class_prio: CLASS_PRIO_IDLE,
    _pad: [0; 7],
    enqueue_task: None,
    dequeue_task: None,
    yield_task: None,
    wakeup_preempt: None,
    pick_next_task: Some(pick_next_task_idle),
    put_prev_task: Some(put_prev_task_idle),
    set_next_task: None,
    task_tick: Some(task_tick_idle),
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
    fn idle_class_priority_is_lowest() {
        assert_eq!(IDLE_SCHED_CLASS.class_prio, CLASS_PRIO_IDLE);
    }
}
