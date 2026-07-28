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

use super::class::{
    CLASS_PRIO_RT, DEQUEUE_MIGRATING, DEQUEUE_SLEEP, ENQUEUE_HEAD, SchedClass,
    TASK_ON_RQ_MIGRATING, TASK_ON_RQ_QUEUED,
};
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
        (*p).m29.on_rq = TASK_ON_RQ_QUEUED;
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
            (*p).m29.on_rq = if flags & DEQUEUE_MIGRATING != 0 {
                TASK_ON_RQ_MIGRATING
            } else {
                0
            };
            (*p).m29.rt.on_rq = 0;
        }
        rq.nr_running = rq.nr_running.saturating_sub(1);
    }
    let _ = flags & DEQUEUE_SLEEP;
    removed
}

unsafe fn pick_next_task_rt(rq: &mut Rq) -> *mut TaskStruct {
    // Linux `pick_next_rt_entity()` first uses the active bitmap to select
    // the highest-priority bucket, then examines that bucket only. Falling
    // through to a lower priority when the highest entity is temporarily not
    // switchable changes the RT class ordering and turns every scheduler pass
    // into a scan of all 100 FIFO queues.
    let Some(prio) = rq.rt.highest_prio() else {
        return core::ptr::null_mut();
    };
    rq.rt.queues[prio as usize]
        .iter()
        .copied()
        .find(|&task| unsafe { super::task_can_switch_to_on_rq(task, rq.current) })
        .unwrap_or(core::ptr::null_mut())
}

unsafe fn put_prev_task_rt(_rq: &mut Rq, _prev: *mut TaskStruct) {}

/// Linux `set_next_task_rt()` publishes the RT class's current entity only
/// after the generic scheduler has selected the task and completed the
/// put-prev/set-next ordering.  The picker must not change `rq.current`.
unsafe fn set_next_task_rt(rq: &mut Rq, next: *mut TaskStruct, _first: bool) {
    if !next.is_null() {
        rq.rt.current = next;
    }
}

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
    set_next_task: Some(set_next_task_rt),
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
    use alloc::boxed::Box;

    #[test]
    fn rr_timeslice_is_100ms() {
        assert_eq!(RR_TIMESLICE_NS, 100_000_000);
        assert_eq!(RR_TIMESLICE_TICKS, 25);
    }

    #[test]
    fn rt_class_above_fair() {
        assert!(CLASS_PRIO_RT < super::super::class::CLASS_PRIO_FAIR);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/rt.c:pick_task_rt
    /// test-origin: linux:vendor/linux/kernel/sched/core.c:prepare_task
    /// test-origin: linux:vendor/linux/kernel/sched/rt.c:set_next_task_rt
    ///
    /// Linux's RT picker returns a candidate without publishing `rq->curr`;
    /// the generic scheduler performs that publication only after the
    /// on_cpu handoff.  Keeping the local rq current task unchanged here is
    /// also what makes the remote-on_cpu rejection effective.
    #[test]
    fn rt_picker_rejects_remote_active_task_without_mutating_rq_current() {
        let mut rq = Rq::new(1);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;
        let task_ptr = &mut *task as *mut TaskStruct;
        let stack_top = (super::super::KTHREAD_STACK_SIZE * 2) as u64;

        rq.current = current_ptr;
        task.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        task.stack = stack_top as *mut core::ffi::c_void;
        task.thread.sp = stack_top - super::super::SWITCH_FRAME_BYTES as u64;
        task.m29.sched_class = &RT_SCHED_CLASS as *const SchedClass;
        task.m29.prio = 10;
        task.m29.policy = SCHED_FIFO;
        task.m29.on_cpu.store(1, Ordering::Release);

        unsafe { enqueue_task_rt(&mut rq, task_ptr, 0) };
        assert!(unsafe { pick_next_task_rt(&mut rq) }.is_null());
        assert_eq!(rq.current, current_ptr);
        assert!(rq.rt.current.is_null());

        rq.current = task_ptr;
        assert_eq!(unsafe { pick_next_task_rt(&mut rq) }, task_ptr);
        assert_eq!(rq.current, task_ptr);
        unsafe { set_next_task_rt(&mut rq, task_ptr, true) };
        assert_eq!(rq.rt.current, task_ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/rt.c:pick_next_rt_entity
    ///
    /// Linux selects from the highest active RT priority only. A task that is
    /// temporarily owned by another CPU must not make the picker fall through
    /// to a lower-priority FIFO task and violate RT class ordering.
    #[test]
    fn rt_picker_does_not_fall_through_highest_active_priority() {
        let mut rq = Rq::new(1);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut high = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut low = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;
        let high_ptr = &mut *high as *mut TaskStruct;
        let low_ptr = &mut *low as *mut TaskStruct;

        rq.current = current_ptr;
        for (task, task_ptr, prio, on_cpu) in
            [(&mut *high, high_ptr, 10, 1), (&mut *low, low_ptr, 20, 0)]
        {
            task.__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
            task.stack = (super::super::KTHREAD_STACK_SIZE * 2) as *mut core::ffi::c_void;
            task.thread.sp = task.stack as u64 - super::super::SWITCH_FRAME_BYTES as u64;
            task.m29.sched_class = &RT_SCHED_CLASS as *const SchedClass;
            task.m29.prio = prio;
            task.m29.policy = SCHED_FIFO;
            task.m29.on_cpu.store(on_cpu, Ordering::Release);
            unsafe { enqueue_task_rt(&mut rq, task_ptr, 0) };
        }

        assert!(unsafe { pick_next_task_rt(&mut rq) }.is_null());
        assert_eq!(rq.rt.highest_prio(), Some(10));
    }
}
