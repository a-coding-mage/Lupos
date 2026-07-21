//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/fair.c
//! test-origin: linux:vendor/linux/kernel/sched/fair.c
//! CFS — Completely Fair Scheduler (M29).
//!
//! Implementation of `sched_class_fair` matching `vendor/linux/kernel/sched/fair.c`.
//!
//! The hot path:
//! ```text
//!  scheduler_tick()
//!    └→ update_curr(rq)            // accumulate vruntime
//!         └→ resched_curr(rq) when slice expired
//!            └→ TIF_NEED_RESCHED set
//!  schedule()
//!    └→ pick_next_task_fair(rq)
//!         └→ pick_next_entity()    // leftmost in rb-tree
//! ```

use super::class::{
    CLASS_PRIO_FAIR, DEQUEUE_SLEEP, ENQUEUE_HEAD, ENQUEUE_INITIAL, ENQUEUE_MIGRATED,
    ENQUEUE_WAKEUP, SchedClass,
};
use super::entity::{SchedEntity, sched_clock_ns};
use super::prio::calc_delta_fair;
use super::rq::Rq;
use crate::kernel::task::{M29SchedFields, TaskStruct};

// ── CFS tunables (Linux defaults) ────────────────────────────────────────────

/// Targeted preemption latency for CPU-bound tasks (Linux 6.M default 6 ms).
pub const SYSCTL_SCHED_LATENCY_NS: u64 = 6_000_000;
/// Minimum preemption granularity for CPU-bound tasks (Linux default 0.75 ms).
pub const SYSCTL_SCHED_MIN_GRANULARITY_NS: u64 = 750_000;
/// Wakeup-preemption granularity (Linux default 1 ms).
pub const SYSCTL_SCHED_WAKEUP_GRANULARITY_NS: u64 = 1_000_000;

// ── Helpers ──────────────────────────────────────────────────────────────────

#[inline]
fn task_se(p: *mut TaskStruct) -> *mut SchedEntity {
    if p.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { &mut (*p).m29.se as *mut SchedEntity }
}

#[inline]
fn task_m29(p: *mut TaskStruct) -> *mut M29SchedFields {
    if p.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { &mut (*p).m29 as *mut M29SchedFields }
}

/// Compute the CFS time slice for the current task on `rq`.
///
/// Equivalent to Linux `sched_slice(cfs_rq, se)`:
///   slice = sched_period(nr_running) * (weight / total_weight)
fn sched_slice(rq: &Rq, se_weight: u64) -> u64 {
    let nr = rq.cfs.nr_running.max(1) as u64;
    // sched_period: max(SCHED_LATENCY_NS, nr * SCHED_MIN_GRANULARITY_NS)
    let period = SYSCTL_SCHED_LATENCY_NS.max(nr.saturating_mul(SYSCTL_SCHED_MIN_GRANULARITY_NS));
    let total_weight = rq.cfs.load_weight.max(1);
    period.saturating_mul(se_weight) / total_weight
}

/// Linux `update_curr(cfs_rq)` — bring `current.vruntime` up to date.
pub unsafe fn update_curr(rq: &mut Rq) {
    let curr = rq.cfs.current;
    if curr.is_null() {
        return;
    }
    let se = task_se(curr);
    let now = sched_clock_ns();
    let last = unsafe { (*se).exec_start };
    if last == 0 {
        unsafe {
            (*se).exec_start = now;
        }
        return;
    }
    let delta_exec = now.saturating_sub(last);
    if delta_exec == 0 {
        return;
    }
    unsafe {
        (*se).exec_start = now;
        (*se).sum_exec_runtime = (*se).sum_exec_runtime.saturating_add(delta_exec);
        let weight = (*se).load.weight;
        (*se).vruntime = (*se)
            .vruntime
            .saturating_add(calc_delta_fair(delta_exec, weight));
        rq.cfs.last_update_ns = now;

        // Check slice expiry — if exceeded, request a reschedule.
        let slice = sched_slice(rq, weight).max(SYSCTL_SCHED_MIN_GRANULARITY_NS);
        let ran = (*se)
            .sum_exec_runtime
            .saturating_sub((*se).prev_sum_exec_runtime);
        if ran >= slice {
            // Set TIF_NEED_RESCHED — picked up by schedule() at the next yield
            // point; under the cooperative scheduler this becomes effective on
            // the next explicit `schedule()` call.
            super::set_task_need_resched(curr);
        }
    }
    rq.cfs.update_min_vruntime();
}

/// Linux `place_entity(cfs_rq, se, initial)` — set the starting vruntime for
/// an entity that's about to be enqueued.
pub unsafe fn place_entity(rq: &Rq, se: *mut SchedEntity, initial: bool) {
    let mut vrt = unsafe { (*se).vruntime.max(rq.cfs.min_vruntime) };
    if initial {
        // Linux gives a small head-start advantage based on `START_DEBIT`,
        // proportional to `sched_vslice(cfs_rq, se)`; we approximate with one
        // minimum granularity tick scaled by weight.
        let weight = unsafe { (*se).load.weight };
        vrt = vrt.saturating_add(calc_delta_fair(SYSCTL_SCHED_MIN_GRANULARITY_NS, weight));
    }
    unsafe {
        (*se).vruntime = vrt;
    }
}

// ── sched_class hooks ────────────────────────────────────────────────────────

unsafe fn wakeup_preempt_fair(rq: &mut Rq, p: *mut TaskStruct, _flags: u32) {
    let current = rq.current;
    if current.is_null() || p.is_null() || current == p {
        return;
    }

    unsafe {
        update_curr(rq);
        let current_se = task_se(current);
        let waking_se = task_se(p);
        let granularity = calc_delta_fair(
            SYSCTL_SCHED_WAKEUP_GRANULARITY_NS,
            (*current_se).load.weight,
        );
        if (*current_se).vruntime.saturating_sub((*waking_se).vruntime) > granularity {
            super::set_task_need_resched(current);
        }
    }
}

unsafe fn enqueue_task_fair(rq: &mut Rq, p: *mut TaskStruct, flags: u32) {
    if p.is_null() {
        return;
    }
    let se = task_se(p);
    let m = task_m29(p);
    unsafe {
        // Linux set_load_weight(): refresh policy-aware load before enqueue.
        super::set_load_weight(p);

        if flags & ENQUEUE_INITIAL != 0 {
            place_entity(rq, se, true);
        } else if flags & ENQUEUE_WAKEUP != 0 {
            place_entity(rq, se, false);
        }
        rq.cfs.insert(p, (*se).vruntime);
        (*se).on_rq = 1;
        (*m).on_rq = 1;
    }
    rq.cfs.nr_running += 1;
    rq.cfs.load_weight = rq
        .cfs
        .load_weight
        .saturating_add(unsafe { (*se).load.weight });
    rq.nr_running = rq.nr_running.saturating_add(1);
    let _ = flags & ENQUEUE_HEAD;
    let _ = flags & ENQUEUE_MIGRATED;
}

unsafe fn dequeue_task_fair(rq: &mut Rq, p: *mut TaskStruct, flags: u32) -> bool {
    if p.is_null() {
        return false;
    }
    let se = task_se(p);
    let m = task_m29(p);
    unsafe {
        rq.cfs.remove(p, (*se).vruntime);
        (*se).on_rq = 0;
        (*m).on_rq = 0;
        rq.cfs.load_weight = rq.cfs.load_weight.saturating_sub((*se).load.weight);
    }
    rq.cfs.nr_running = rq.cfs.nr_running.saturating_sub(1);
    rq.nr_running = rq.nr_running.saturating_sub(1);
    let _ = flags & DEQUEUE_SLEEP;
    true
}

unsafe fn pick_next_task_fair(rq: &mut Rq) -> *mut TaskStruct {
    let p = rq
        .cfs
        .tasks_timeline
        .iter()
        .find_map(|task| {
            if unsafe { super::task_can_switch_to(task) } {
                Some(task)
            } else {
                None
            }
        })
        .unwrap_or(core::ptr::null_mut());
    if !p.is_null() {
        // Linux set_next_entity(): the running entity remains accounted as
        // on_rq but is not kept in the ordered tree.
        unsafe {
            rq.cfs.remove(p, (*task_se(p)).vruntime);
        }
        rq.cfs.current = p;
        rq.current = p;
        // Refresh exec_start so update_curr can compute delta_exec next tick.
        let se = task_se(p);
        unsafe {
            (*se).exec_start = sched_clock_ns();
            (*se).prev_sum_exec_runtime = (*se).sum_exec_runtime;
        }
    }
    p
}

unsafe fn put_prev_task_fair(rq: &mut Rq, prev: *mut TaskStruct) {
    if prev.is_null() {
        return;
    }
    unsafe {
        update_curr(rq);
        let se = task_se(prev);
        if (*se).on_rq != 0 {
            rq.cfs.insert(prev, (*se).vruntime);
        }
    }
    rq.cfs.current = core::ptr::null_mut();
}

unsafe fn task_tick_fair(rq: &mut Rq, p: *mut TaskStruct, _queued: bool) {
    if p.is_null() {
        return;
    }
    if rq.cfs.current != p {
        rq.cfs.current = p;
        let se = task_se(p);
        let now = sched_clock_ns();
        unsafe {
            (*se).exec_start = now;
            (*se).prev_sum_exec_runtime = (*se).sum_exec_runtime;
        }
    }
    unsafe {
        update_curr(rq);
    }
}

unsafe fn task_fork_fair(p: *mut TaskStruct) {
    if p.is_null() {
        return;
    }
    let m = task_m29(p);
    let se = task_se(p);
    unsafe {
        super::set_load_weight(p);
        (*se).vruntime = 0;
        (*se).sum_exec_runtime = 0;
        (*se).prev_sum_exec_runtime = 0;
        (*m).sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
    }
}

unsafe fn task_dead_fair(_p: *mut TaskStruct) {
    // Nothing to do — runqueue dequeue already happened in do_exit.
}

unsafe fn yield_task_fair(rq: &mut Rq) {
    let curr = rq.cfs.current;
    if curr.is_null() {
        return;
    }
    let se = task_se(curr);
    unsafe {
        // Push our vruntime to the rightmost entity so the leftmost picks
        // someone else.  Mirrors Linux `yield_task_fair` heuristic.
        let max_vruntime = rq.cfs.tasks_timeline.last_vruntime();
        if let Some(max_vrt) = max_vruntime {
            let bump = max_vrt.saturating_add(1);
            (*se).vruntime = bump;
        }
    }
}

unsafe fn update_curr_fair(rq: &mut Rq) {
    unsafe { update_curr(rq) };
}

unsafe fn get_rr_interval_fair(_rq: &mut Rq, _p: *mut TaskStruct) -> u64 {
    SYSCTL_SCHED_LATENCY_NS
}

unsafe fn select_task_rq_fair(p: *mut TaskStruct, prev_cpu: u32, flags: u32) -> u32 {
    super::select_idlest_active_cpu(p, prev_cpu, flags)
}

// ── FAIR_SCHED_CLASS singleton ───────────────────────────────────────────────

pub static FAIR_SCHED_CLASS: SchedClass = SchedClass {
    class_prio: CLASS_PRIO_FAIR,
    _pad: [0; 7],
    enqueue_task: Some(enqueue_task_fair),
    dequeue_task: Some(dequeue_task_fair),
    yield_task: Some(yield_task_fair),
    wakeup_preempt: Some(wakeup_preempt_fair),
    pick_next_task: Some(pick_next_task_fair),
    put_prev_task: Some(put_prev_task_fair),
    set_next_task: None,
    task_tick: Some(task_tick_fair),
    task_fork: Some(task_fork_fair),
    task_dead: Some(task_dead_fair),
    switched_to: None,
    prio_changed: None,
    get_rr_interval: Some(get_rr_interval_fair),
    update_curr: Some(update_curr_fair),
    select_task_rq: Some(select_task_rq_fair),
};

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::prio::{DEFAULT_PRIO, NICE_0_LOAD, nice_to_weight};
    use super::*;
    use alloc::boxed::Box;
    use core::sync::atomic::Ordering;

    #[test]
    fn sched_slice_proportional_to_weight() {
        let mut rq = Rq::new(0);
        rq.cfs.nr_running = 3;
        rq.cfs.load_weight = NICE_0_LOAD * 3;

        let nice0_slice = sched_slice(&rq, NICE_0_LOAD);
        let nice19_slice = sched_slice(&rq, nice_to_weight(19));

        // Lower weight gets a proportionally smaller slice.
        assert!(nice0_slice > nice19_slice * 50);
    }

    #[test]
    fn fair_class_dispatch_vector_is_populated() {
        let c = &FAIR_SCHED_CLASS;
        assert!(c.enqueue_task.is_some());
        assert!(c.dequeue_task.is_some());
        assert!(c.pick_next_task.is_some());
        assert!(c.task_tick.is_some());
        assert_eq!(c.class_prio, CLASS_PRIO_FAIR);
    }

    #[test]
    fn min_granularity_lower_than_latency() {
        assert!(SYSCTL_SCHED_MIN_GRANULARITY_NS < SYSCTL_SCHED_LATENCY_NS);
    }

    #[test]
    fn update_curr_keeps_running_entity_out_of_cfs_tree() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        unsafe {
            (*ptr).m29.static_prio = DEFAULT_PRIO;
            (*ptr).m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            (*ptr).m29.se.load.weight = NICE_0_LOAD;
            (*ptr).m29.se.on_rq = 1;
            (*ptr).m29.se.vruntime = 0;
            (*ptr).m29.se.exec_start = 1;
        }
        rq.cfs.current = ptr;
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = NICE_0_LOAD;
        crate::arch::x86::kernel::apic_timer::TIMER_TICKS.store(1, Ordering::Release);
        unsafe {
            update_curr(&mut rq);
        }

        let new_vruntime = unsafe { (*ptr).m29.se.vruntime };
        assert!(new_vruntime > 0);
        assert!(!rq.cfs.tasks_timeline.contains_key(&(0, ptr as usize)));
        assert!(
            !rq.cfs
                .tasks_timeline
                .contains_key(&(new_vruntime, ptr as usize)),
            "Linux keeps cfs_rq->curr outside the rb-tree"
        );
        crate::arch::x86::kernel::apic_timer::TIMER_TICKS.store(0, Ordering::Release);
    }

    #[test]
    fn update_curr_uses_positive_nice_entity_weight() {
        // Linux fair.c::calc_delta_fair() passes curr->load unchanged to
        // __calc_delta(); a positive nice value must therefore advance
        // vruntime faster than nice 0.
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        let weight = nice_to_weight(1);
        let start = sched_clock_ns().max(1);
        let _ = crate::kernel::sched::entity::SCHED_CLOCK_NS.fetch_max(start, Ordering::AcqRel);
        unsafe {
            (*ptr).m29.static_prio = DEFAULT_PRIO + 1;
            (*ptr).m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            (*ptr).m29.se.load.weight = weight;
            (*ptr).m29.se.on_rq = 1;
            (*ptr).m29.se.exec_start = start;
        }
        rq.cfs.current = ptr;
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = weight;
        let _ = crate::kernel::sched::entity::SCHED_CLOCK_NS.fetch_add(1_000_000, Ordering::AcqRel);

        unsafe {
            update_curr(&mut rq);
        }

        let delta_exec = task.m29.se.sum_exec_runtime;
        assert!(delta_exec > 0);
        assert_eq!(
            task.m29.se.vruntime,
            calc_delta_fair(delta_exec, weight),
            "Linux accounts a positive-nice entity using its actual load weight"
        );
    }

    #[test]
    fn enqueue_sched_idle_uses_linux_idle_weight() {
        // Linux core.c::set_load_weight() assigns
        // scale_load(WEIGHT_IDLEPRIO) to a task with SCHED_IDLE policy. On
        // CONFIG_64BIT x86_64 this is 3 << 10 == 3072.
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        task.m29.policy = super::super::prio::SCHED_IDLE;
        task.m29.static_prio = DEFAULT_PRIO;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;

        unsafe {
            enqueue_task_fair(&mut rq, ptr, 0);
        }

        assert_eq!(
            task.m29.se.load.weight,
            3_u64 << super::super::prio::SCHED_FIXEDPOINT_SHIFT,
            "Linux uses scale_load(WEIGHT_IDLEPRIO) for SCHED_IDLE"
        );
        assert_eq!(
            task.m29.se.load.inv_weight,
            super::super::prio::WMULT_IDLEPRIO,
            "Linux uses WMULT_IDLEPRIO for SCHED_IDLE"
        );
    }

    #[test]
    fn task_tick_fair_accounts_passed_task_when_cfs_current_is_stale() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        unsafe {
            (*ptr).m29.static_prio = DEFAULT_PRIO;
            (*ptr).m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            (*ptr).m29.se.load.weight = NICE_0_LOAD;
            (*ptr).m29.se.on_rq = 1;
            (*ptr).m29.on_rq = 1;
        }
        // Linux keeps the current runnable entity in cfs_rq->load and
        // nr_running even though it is outside the timeline tree.
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = NICE_0_LOAD;
        rq.nr_running = 1;

        crate::kernel::sched::entity::SCHED_CLOCK_NS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |clock| {
                Some(clock.max(1))
            })
            .unwrap();
        unsafe {
            task_tick_fair(&mut rq, ptr, true);
        }
        assert_eq!(rq.cfs.current, ptr);
        let first_runtime = task.m29.se.sum_exec_runtime;

        // Advance the shared monotonic scheduler clock by 8 ms. Using an
        // absolute LAPIC tick value made this fixture depend on tests that had
        // already advanced the process-wide scheduler clock.
        crate::kernel::sched::entity::SCHED_CLOCK_NS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |clock| {
                Some(clock.saturating_add(8_000_000))
            })
            .unwrap();
        unsafe {
            task_tick_fair(&mut rq, ptr, true);
        }

        assert!(task.m29.se.sum_exec_runtime > first_runtime);
        assert_ne!(
            task.thread_info.flags.load(Ordering::Acquire) & crate::kernel::task::TIF_NEED_RESCHED,
            0,
            "timer tick must request a cooperative reschedule for legacy fair tasks"
        );
    }

    #[test]
    fn pick_next_task_fair_skips_non_switchable_leftmost_entity() {
        let mut rq = Rq::new(0);
        let mut sleeper = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut runnable = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let sleeper_ptr = &mut *sleeper as *mut TaskStruct;
        let runnable_ptr = &mut *runnable as *mut TaskStruct;

        sleeper.__state.store(
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            Ordering::Release,
        );
        let sleeper_stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * 2;
        sleeper.stack = sleeper_stack_top as *mut core::ffi::c_void;
        sleeper.thread.sp = sleeper_stack_top as u64 - 64;
        sleeper.m29.se.vruntime = 1;

        runnable.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        let runnable_stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * 3;
        runnable.stack = runnable_stack_top as *mut core::ffi::c_void;
        runnable.thread.sp = runnable_stack_top as u64 - 64;
        runnable.m29.se.vruntime = 2;

        rq.cfs.insert(sleeper_ptr, sleeper.m29.se.vruntime);
        rq.cfs.insert(runnable_ptr, runnable.m29.se.vruntime);

        let picked = unsafe { pick_next_task_fair(&mut rq) };

        assert_eq!(picked, runnable_ptr);
        assert_eq!(rq.cfs.current, runnable_ptr);
    }
}
