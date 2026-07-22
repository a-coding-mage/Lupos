//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/rt.c
//! test-origin: linux:vendor/linux/kernel/sched/rt.c
//! Realtime scheduling class — `SCHED_FIFO` and `SCHED_RR` (M30).
//!
//! Mirrors `vendor/linux/kernel/sched/rt.c`.  Linux uses one FIFO list per RT
//! priority bucket (0..99); we mirror that in `RtRq`.  Within a bucket FIFO
//! order is preserved; RR rotates the head every `RR_TIMESLICE_TICKS`.
//!
//! Class priority order: stop > dl > **rt** > fair > idle.

use core::sync::atomic::Ordering;

use crate::kernel::task::TaskStruct;

use super::class::{CLASS_PRIO_RT, DEQUEUE_SLEEP, ENQUEUE_HEAD, SchedClass};
use super::prio::{MAX_RT_PRIO, SCHED_FIFO, SCHED_RR};
use super::rq::Rq;

/// Linux `RR_TIMESLICE_NS` — default round-robin slice (100 ms).
///
/// `sysctl_sched_rr_timeslice` in `vendor/linux/kernel/sched/rt.c` defaults to
/// `RR_TIMESLICE = 100 * HZ / 1000` jiffies.
pub const RR_TIMESLICE_NS: u64 = 100_000_000;

/// Linux `RR_TIMESLICE = 100 * HZ / 1000` scheduler ticks.
pub const RR_TIMESLICE_TICKS: u32 = (100 * crate::kernel::time::jiffies::HZ / 1000) as u32;

unsafe fn wakeup_preempt_rt(rq: &mut Rq, p: *mut TaskStruct, _flags: u32) {
    let current = rq.current;
    if current.is_null() || p.is_null() || current == p {
        return;
    }
    unsafe {
        if (*p).m29.prio < (*current).m29.prio {
            super::set_task_need_resched(current);
        }
    }
}

unsafe fn enqueue_task_rt(rq: &mut Rq, p: *mut TaskStruct, flags: u32) {
    if p.is_null() {
        return;
    }
    let prio = unsafe { (*p).m29.prio };
    rq.rt.enqueue(p, prio, flags & ENQUEUE_HEAD != 0);
    unsafe {
        (*p).m29.on_rq = 1;
        (*p).m29.rt.on_rq = 1;
        if (*p).m29.rt.time_slice == 0 {
            (*p).m29.rt.time_slice = RR_TIMESLICE_TICKS;
        }
    }
    rq.nr_running = rq.nr_running.saturating_add(1);
}

unsafe fn dequeue_task_rt(rq: &mut Rq, p: *mut TaskStruct, flags: u32) -> bool {
    if p.is_null() {
        return false;
    }
    let prio = unsafe { (*p).m29.prio };
    let removed = rq.rt.dequeue(p, prio);
    if removed {
        unsafe {
            (*p).m29.on_rq = 0;
            (*p).m29.rt.on_rq = 0;
        }
        rq.nr_running = rq.nr_running.saturating_sub(1);
    }
    let _ = flags & DEQUEUE_SLEEP;
    removed
}

unsafe fn pick_next_task_rt(rq: &mut Rq) -> *mut TaskStruct {
    let p = rq.rt.pick_first();
    if !p.is_null() {
        rq.rt.current = p;
        rq.current = p;
    }
    p
}

unsafe fn put_prev_task_rt(_rq: &mut Rq, _prev: *mut TaskStruct) {}

unsafe fn task_tick_rt(rq: &mut Rq, p: *mut TaskStruct, _queued: bool) {
    if p.is_null() {
        return;
    }
    unsafe {
        let policy = (*p).m29.policy;
        if policy != SCHED_RR {
            return; // SCHED_FIFO never preempts on tick.
        }
        if (*p).m29.rt.time_slice > 0 {
            (*p).m29.rt.time_slice -= 1;
        }
        if (*p).m29.rt.time_slice == 0 {
            (*p).m29.rt.time_slice = RR_TIMESLICE_TICKS;
            // Rotate this priority's FIFO so the next pick takes the sibling.
            rq.rt.requeue_tail((*p).m29.prio);
            (*p).thread_info
                .flags
                .fetch_or(crate::kernel::task::TIF_NEED_RESCHED, Ordering::Release);
        }
    }
}

unsafe fn task_fork_rt(p: *mut TaskStruct) {
    if p.is_null() {
        return;
    }
    unsafe {
        (*p).m29.rt.time_slice = RR_TIMESLICE_TICKS;
    }
}

unsafe fn switched_to_rt(rq: &mut Rq, p: *mut TaskStruct) {
    if p.is_null() || rq.current == p || unsafe { (*p).m29.on_rq == 0 } {
        return;
    }
    let current = rq.current;
    if current.is_null() {
        return;
    }
    if current == rq.idle || unsafe { (*p).m29.prio < (*current).m29.prio } {
        super::set_task_need_resched(current);
    }
}

unsafe fn get_rr_interval_rt(_rq: &mut Rq, p: *mut TaskStruct) -> u64 {
    if p.is_null() {
        return 0;
    }
    unsafe {
        if (*p).m29.policy == SCHED_RR {
            RR_TIMESLICE_NS
        } else {
            0 // SCHED_FIFO has no slice.
        }
    }
}

unsafe fn select_task_rq_rt(_p: *mut TaskStruct, prev_cpu: u32, _flags: u32) -> u32 {
    prev_cpu
}

pub static RT_SCHED_CLASS: SchedClass = SchedClass {
    class_prio: CLASS_PRIO_RT,
    _pad: [0; 7],
    enqueue_task: Some(enqueue_task_rt),
    dequeue_task: Some(dequeue_task_rt),
    yield_task: None,
    wakeup_preempt: Some(wakeup_preempt_rt),
    pick_next_task: Some(pick_next_task_rt),
    put_prev_task: Some(put_prev_task_rt),
    set_next_task: None,
    task_tick: Some(task_tick_rt),
    task_fork: Some(task_fork_rt),
    task_dead: None,
    switched_to: Some(switched_to_rt),
    prio_changed: None,
    get_rr_interval: Some(get_rr_interval_rt),
    update_curr: None,
    select_task_rq: Some(select_task_rq_rt),
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rr_timeslice_is_100ms() {
        assert_eq!(RR_TIMESLICE_NS, 100_000_000);
        assert_eq!(RR_TIMESLICE_TICKS, 25);
    }
    #[test]
    fn rt_class_above_fair() {
        assert!(CLASS_PRIO_RT < super::super::class::CLASS_PRIO_FAIR);
    }
}
