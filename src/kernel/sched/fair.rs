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
    CLASS_PRIO_FAIR, DEQUEUE_MIGRATING, DEQUEUE_SLEEP, ENQUEUE_HEAD, ENQUEUE_INITIAL,
    ENQUEUE_MIGRATED, ENQUEUE_WAKEUP, SchedClass, TASK_ON_RQ_MIGRATING, TASK_ON_RQ_QUEUED,
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
/// `sysctl_sched_base_slice` default in the vendored Linux EEVDF scheduler.
pub const SYSCTL_SCHED_BASE_SLICE_NS: u64 = 700_000;

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
        // Linux `activate_task()` keeps p->on_rq at
        // TASK_ON_RQ_MIGRATING while the class enqueues a detached task.
        // Accept that one state for ENQUEUE_MIGRATED; every other nonzero
        // value still means this entity is already queued.
        let migrating = (*m).on_rq == TASK_ON_RQ_MIGRATING && flags & ENQUEUE_MIGRATED != 0;
        if (*se).on_rq != 0 || ((*m).on_rq != 0 && !migrating) {
            debug_assert_eq!((*se).on_rq != 0, (*m).on_rq != 0);
            return;
        }
        if rq.cfs.entity_node_linked(p) {
            return;
        }

        // Linux set_load_weight(): refresh policy-aware load before enqueue.
        super::set_load_weight(p);

        if flags & ENQUEUE_INITIAL != 0 {
            place_entity(rq, se, true);
        } else if flags & ENQUEUE_WAKEUP != 0 {
            place_entity(rq, se, false);
        }
        if !rq.cfs.insert(p, (*se).vruntime) {
            return;
        }
        (*se).on_rq = 1;
        (*m).on_rq = TASK_ON_RQ_QUEUED;
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
        let removed = if rq.cfs.current == p {
            // Linux dequeue_entities() succeeds for the currently running
            // entity, which is intentionally not present in the rb-tree.
            true
        } else {
            // `__dequeue_entity()` operates on the embedded run_node and can
            // fail here when the caller's rq is not its owner.  Preserve the
            // Linux failure path: do not clear on_rq or release the task while
            // another cfs_rq can still reference its intrusive node.
            rq.cfs.remove(p, (*se).vruntime)
        };
        if !removed {
            return false;
        }
        (*se).on_rq = 0;
        (*m).on_rq = if flags & DEQUEUE_MIGRATING != 0 {
            TASK_ON_RQ_MIGRATING
        } else {
            0
        };
        rq.cfs.load_weight = rq.cfs.load_weight.saturating_sub((*se).load.weight);
    }
    rq.cfs.nr_running = rq.cfs.nr_running.saturating_sub(1);
    rq.nr_running = rq.nr_running.saturating_sub(1);
    let _ = flags & DEQUEUE_SLEEP;
    true
}

unsafe fn pick_next_task_fair(rq: &mut Rq) -> *mut TaskStruct {
    // Linux pick_task_fair() only selects an entity.  The scheduler core's
    // put_prev_set_next_task() performs the ordered handoff below: first
    // put_prev_entity() requeues the old current task, then set_next_entity()
    // removes the selected task and records cfs_rq->curr.
    //
    // Linux's pick_eevdf() has a one-entity fast path: when the current fair
    // entity is the only runnable entity, it is not in the rb-tree, so the
    // picker must return cfs_rq->curr.  Without this, a reschedule request
    // with no peer selects the idle class, requeues the still-runnable current
    // task, and leaves the CPU asleep with work on its CFS tree.
    if rq.cfs.nr_running == 1 {
        let current = rq.cfs.current;
        if !current.is_null()
            && unsafe { (*current).m29.se.on_rq != 0 }
            && unsafe { super::task_can_switch_to(current) }
        {
            return current;
        }
    }
    rq.cfs
        .tasks_timeline
        .iter()
        .find_map(|task| {
            if unsafe { super::task_can_switch_to(task) } {
                Some(task)
            } else {
                None
            }
        })
        .unwrap_or(core::ptr::null_mut())
}

unsafe fn put_prev_task_fair(rq: &mut Rq, prev: *mut TaskStruct) {
    if prev.is_null() {
        return;
    }
    let was_current = rq.cfs.current == prev;
    let was_linked = rq.cfs.contains_task(prev);
    if was_current {
        unsafe {
            update_curr(rq);
        }
    }
    unsafe {
        let se = task_se(prev);
        if (*se).on_rq != 0 && (was_current || !was_linked) {
            let _ = rq.cfs.insert(prev, (*se).vruntime);
        }
    }
    rq.cfs.current = core::ptr::null_mut();
}

/// Linux `set_next_task_fair()` / `set_next_entity()`.
///
/// `pick_next_task_fair()` handles the usual tree pick, but the scheduler core
/// can continue with `prev` when no class pick is available. Linux still calls
/// this hook for that selected fair task; without it a stale `cfs.current`
/// makes the next timer tick skip accounting forever.
unsafe fn set_next_task_fair(rq: &mut Rq, next: *mut TaskStruct, _first: bool) {
    if next.is_null() || rq.cfs.current == next {
        return;
    }

    let se = task_se(next);
    if unsafe { (*se).on_rq != 0 } {
        // Linux set_next_entity() removes the selected on-rq entity after
        // put_prev_entity() has completed.  Refuse the handoff if the
        // embedded node is not owned by this cfs_rq; silently marking it
        // current would leave another runqueue with a live intrusive node.
        let removed = unsafe { rq.cfs.remove(next, (*se).vruntime) };
        debug_assert!(
            removed,
            "selected fair entity must be removable by run_node"
        );
        if !removed {
            return;
        }
    }

    rq.cfs.current = next;
    let now = sched_clock_ns();
    unsafe {
        (*se).exec_start = now;
        (*se).prev_sum_exec_runtime = (*se).sum_exec_runtime;
    }
}

unsafe fn task_tick_fair(rq: &mut Rq, p: *mut TaskStruct, _queued: bool) {
    if p.is_null() {
        return;
    }
    if rq.cfs.current != p {
        return;
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
    set_next_task: Some(set_next_task_fair),
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

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:enqueue_task_fair
    #[test]
    fn enqueue_task_fair_does_not_reenqueue_entity_already_on_rq() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        task.m29.static_prio = DEFAULT_PRIO;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        task.m29.se.load.weight = NICE_0_LOAD;

        unsafe {
            enqueue_task_fair(&mut rq, ptr, ENQUEUE_INITIAL);
        }
        assert_eq!(task.m29.on_rq, 1);
        assert_eq!(task.m29.se.on_rq, 1);
        assert_eq!(rq.nr_running, 1);
        assert_eq!(rq.cfs.nr_running, 1);
        assert_eq!(rq.cfs.load_weight, NICE_0_LOAD);

        unsafe {
            enqueue_task_fair(&mut rq, ptr, ENQUEUE_WAKEUP);
        }

        let mut queued = rq.cfs.tasks_timeline.iter();
        assert_eq!(
            queued.next(),
            Some(ptr),
            "Linux breaks out of enqueue_task_fair() when se->on_rq is already set"
        );
        assert_eq!(queued.next(), None);
        assert_eq!(rq.nr_running, 1);
        assert_eq!(rq.cfs.nr_running, 1);
        assert_eq!(rq.cfs.load_weight, NICE_0_LOAD);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/core.c:deactivate_task
    /// and linux:vendor/linux/kernel/sched/core.c:activate_task
    ///
    /// Linux publishes TASK_ON_RQ_MIGRATING before class dequeue and does not
    /// clear it until after destination enqueue.  The nonzero state is the
    /// task-struct lifetime boundary for an embedded CFS run_node.
    #[test]
    fn fair_migration_keeps_on_rq_nonzero_across_detach_and_attach() {
        let mut source = Rq::new(0);
        let mut destination = Rq::new(1);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        task.m29.static_prio = DEFAULT_PRIO;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        task.m29.se.load.weight = NICE_0_LOAD;

        unsafe {
            enqueue_task_fair(&mut source, ptr, ENQUEUE_INITIAL);
            // This is Linux deactivate_task()'s store, performed before the
            // class hook clears se.on_rq and removes the rb node.
            (*ptr).m29.on_rq = TASK_ON_RQ_MIGRATING;
            assert!(dequeue_task_fair(&mut source, ptr, DEQUEUE_MIGRATING));
        }
        assert_eq!(task.m29.se.on_rq, 0);
        assert_eq!(task.m29.on_rq, TASK_ON_RQ_MIGRATING);
        assert!(source.cfs.tasks_timeline.is_empty());

        unsafe {
            enqueue_task_fair(&mut destination, ptr, ENQUEUE_MIGRATED);
        }
        assert_eq!(task.m29.se.on_rq, 1);
        assert_eq!(task.m29.on_rq, TASK_ON_RQ_QUEUED);
        assert_eq!(destination.cfs.tasks_timeline.first(), ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:dequeue_task_fair
    ///
    /// Linux returns failure from `dequeue_task_fair()` when the entity could
    /// not be removed from its cfs_rq.  This Lupos-specific ownership setup
    /// exercises the equivalent failure: calling the hook with a different
    /// runqueue must not clear `on_rq`, because task release uses that state
    /// to keep the still-linked intrusive node alive.
    #[test]
    fn dequeue_task_fair_preserves_state_when_node_belongs_to_other_rq() {
        let mut owner = Rq::new(0);
        let mut wrong_rq = Rq::new(1);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        task.m29.static_prio = DEFAULT_PRIO;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        task.m29.se.load.weight = NICE_0_LOAD;
        task.m29.se.vruntime = 10;
        task.m29.se.on_rq = 1;
        task.m29.on_rq = 1;

        assert!(owner.cfs.insert(ptr, task.m29.se.vruntime));
        owner.cfs.nr_running = 1;
        owner.cfs.load_weight = NICE_0_LOAD;
        owner.nr_running = 1;

        let dequeued = unsafe { dequeue_task_fair(&mut wrong_rq, ptr, DEQUEUE_SLEEP) };

        assert!(!dequeued, "Linux reports a failed dequeue to its caller");
        assert_eq!(task.m29.se.on_rq, 1);
        assert_eq!(task.m29.on_rq, 1);
        assert_eq!(owner.cfs.nr_running, 1);
        assert_eq!(owner.nr_running, 1);
        assert_eq!(owner.cfs.tasks_timeline.first(), ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:task_tick_fair
    #[test]
    fn task_tick_fair_does_not_make_a_queued_entity_current() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        unsafe {
            (*ptr).m29.static_prio = DEFAULT_PRIO;
            (*ptr).m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            (*ptr).m29.se.load.weight = NICE_0_LOAD;
            (*ptr).m29.se.vruntime = 42;
            (*ptr).m29.se.on_rq = 1;
            (*ptr).m29.on_rq = 1;
        }
        rq.cfs.insert(ptr, task.m29.se.vruntime);
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = NICE_0_LOAD;
        rq.nr_running = 1;

        // Linux task_tick_fair() updates cfs_rq->curr via entity_tick(); it
        // does not assign cfs_rq->curr from the task argument. A queued entity
        // must therefore stay in the rb-tree if cfs.current is stale.
        crate::kernel::sched::entity::SCHED_CLOCK_NS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |clock| {
                Some(clock.max(1))
            })
            .unwrap();
        unsafe {
            task_tick_fair(&mut rq, ptr, true);
        }

        assert!(rq.cfs.current.is_null());
        assert_eq!(task.m29.se.sum_exec_runtime, 0);
        assert_eq!(task.m29.se.vruntime, 42);
        let mut queued = rq.cfs.tasks_timeline.iter();
        assert_eq!(queued.next(), Some(ptr));
        assert_eq!(queued.next(), None);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:set_next_task_fair
    #[test]
    fn set_next_task_fair_repairs_current_handoff_and_removes_tree_node() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        unsafe {
            (*ptr).m29.se.on_rq = 1;
            (*ptr).m29.on_rq = 1;
            (*ptr).m29.se.vruntime = 17;
        }
        rq.cfs.insert(ptr, task.m29.se.vruntime);
        rq.cfs.nr_running = 1;

        unsafe {
            set_next_task_fair(&mut rq, ptr, false);
        }

        assert_eq!(rq.cfs.current, ptr);
        assert!(rq.cfs.tasks_timeline.first().is_null());
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:put_prev_entity
    #[test]
    fn put_prev_task_fair_clears_stale_current_after_mismatch() {
        let mut rq = Rq::new(0);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut queued = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;
        let queued_ptr = &mut *queued as *mut TaskStruct;

        for (task, vruntime) in [(&mut current, 10), (&mut queued, 20)] {
            task.m29.static_prio = DEFAULT_PRIO;
            task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            task.m29.se.load.weight = NICE_0_LOAD;
            task.m29.se.vruntime = vruntime;
            task.m29.se.on_rq = 1;
            task.m29.on_rq = 1;
        }
        rq.cfs.current = current_ptr;
        rq.current = current_ptr;
        rq.cfs.insert(queued_ptr, queued.m29.se.vruntime);
        rq.cfs.nr_running = 2;
        rq.cfs.load_weight = NICE_0_LOAD * 2;
        rq.nr_running = 2;

        unsafe {
            put_prev_task_fair(&mut rq, queued_ptr);
        }

        assert!(
            rq.cfs.current.is_null(),
            "Linux put_prev_entity() warns on cfs_rq->curr mismatch but still clears cfs_rq->curr"
        );
        let mut queued_iter = rq.cfs.tasks_timeline.iter();
        assert_eq!(queued_iter.next(), Some(queued_ptr));
        assert_eq!(queued_iter.next(), None);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:put_prev_entity
    #[test]
    fn put_prev_task_fair_requeues_runnable_prev_when_current_tracking_is_stale() {
        let mut rq = Rq::new(0);
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let ptr = &mut *task as *mut TaskStruct;
        task.m29.static_prio = DEFAULT_PRIO;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        task.m29.se.load.weight = NICE_0_LOAD;
        task.m29.se.vruntime = 33;
        task.m29.se.on_rq = 1;
        task.m29.on_rq = 1;
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = NICE_0_LOAD;
        rq.nr_running = 1;

        unsafe {
            put_prev_task_fair(&mut rq, ptr);
        }

        assert!(rq.cfs.current.is_null());
        let mut queued = rq.cfs.tasks_timeline.iter();
        assert_eq!(
            queued.next(),
            Some(ptr),
            "a runnable previous fair task must not be left outside both cfs.current and the rb-tree"
        );
        assert_eq!(queued.next(), None);
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
        sleeper.m29.se.on_rq = 1;
        sleeper.m29.on_rq = 1;

        runnable.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        let runnable_stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * 3;
        runnable.stack = runnable_stack_top as *mut core::ffi::c_void;
        runnable.thread.sp = runnable_stack_top as u64 - 64;
        runnable.m29.se.vruntime = 2;
        runnable.m29.se.on_rq = 1;
        runnable.m29.on_rq = 1;

        rq.cfs.insert(sleeper_ptr, sleeper.m29.se.vruntime);
        rq.cfs.insert(runnable_ptr, runnable.m29.se.vruntime);

        let picked = unsafe { pick_next_task_fair(&mut rq) };

        assert_eq!(picked, runnable_ptr);
        assert!(rq.cfs.current.is_null());
        assert_eq!(rq.cfs.tasks_timeline.first(), sleeper_ptr);
        unsafe {
            set_next_task_fair(&mut rq, picked, true);
        }
        assert_eq!(rq.cfs.current, runnable_ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/core.c:pick_next_task
    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:pick_task_fair
    ///
    /// Linux's picker only chooses an entity.  The scheduler core then calls
    /// `put_prev_set_next_task()`, where `put_prev_entity()` requeues the old
    /// current task and `set_next_entity()` removes the selected task.  A
    /// picker that mutates either side early changes which task is selected
    /// and leaves the handoff order unlike Linux.
    #[test]
    fn pick_next_task_fair_defers_tree_handoff_to_set_next_entity() {
        let mut rq = Rq::new(0);
        let mut prev = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut next = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let prev_ptr = &mut *prev as *mut TaskStruct;
        let next_ptr = &mut *next as *mut TaskStruct;

        for (task, vruntime) in [(&mut prev, 1), (&mut next, 2)] {
            task.__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
            let stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * (2 + vruntime as usize);
            task.stack = stack_top as *mut core::ffi::c_void;
            task.thread.sp = stack_top as u64 - 64;
            task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
            task.m29.se.load.weight = NICE_0_LOAD;
            task.m29.se.vruntime = vruntime;
            task.m29.se.on_rq = 1;
            task.m29.on_rq = 1;
        }
        rq.current = prev_ptr;
        rq.cfs.current = prev_ptr;
        rq.cfs.nr_running = 2;
        rq.cfs.load_weight = NICE_0_LOAD * 2;
        rq.nr_running = 2;
        assert!(rq.cfs.insert(next_ptr, next.m29.se.vruntime));

        let picked = unsafe { pick_next_task_fair(&mut rq) };

        assert_eq!(picked, next_ptr);
        assert_eq!(rq.cfs.current, prev_ptr);
        assert_eq!(rq.current, prev_ptr);
        assert_eq!(rq.cfs.tasks_timeline.first(), next_ptr);

        unsafe {
            put_prev_task_fair(&mut rq, prev_ptr);
            set_next_task_fair(&mut rq, picked, true);
        }
        assert_eq!(rq.cfs.current, next_ptr);
        assert_eq!(rq.cfs.tasks_timeline.first(), prev_ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:pick_task_fair
    ///
    /// Linux relies on rq ownership to keep a queued entity from being
    /// selected twice; its picker does not add an independent `on_cpu` test.
    /// This Lupos-specific fixture preserves a runnable queued entity with a
    /// stale handoff bit, the state that made the former invented filter turn
    /// a non-empty CFS runqueue into an idle pick on SMP.
    #[test]
    fn pick_next_task_fair_does_not_filter_runnable_entity_by_on_cpu() {
        let mut rq = Rq::new(1);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;
        let task_ptr = &mut *task as *mut TaskStruct;

        rq.current = current_ptr;
        task.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        let stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * 2;
        task.stack = stack_top as *mut core::ffi::c_void;
        task.thread.sp = stack_top as u64 - 64;
        task.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        task.m29.se.on_rq = 1;
        task.m29.on_rq = 1;
        task.m29.se.vruntime = 1;
        task.m29.on_cpu.store(1, Ordering::Release);

        rq.cfs.insert(task_ptr, task.m29.se.vruntime);
        rq.cfs.nr_running = 1;
        rq.nr_running = 1;

        let picked = unsafe { pick_next_task_fair(&mut rq) };

        assert_eq!(picked, task_ptr);
        assert_eq!(rq.cfs.tasks_timeline.first(), task_ptr);
        unsafe {
            set_next_task_fair(&mut rq, picked, true);
        }
        assert_eq!(rq.cfs.current, task_ptr);
    }

    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:pick_eevdf
    /// test-origin: linux:vendor/linux/kernel/sched/fair.c:pick_task_fair
    ///
    /// Linux's EEVDF picker returns cfs_rq->curr when it is the only
    /// runnable entity (`nr_queued == 1`).  The current entity is not in the
    /// rb-tree while executing, so a tree-only picker incorrectly falls back
    /// to the idle class and strands the sole runnable task after a tick or a
    /// user-fault reschedule.
    #[test]
    fn pick_next_task_fair_keeps_the_only_current_task_running() {
        let mut rq = Rq::new(0);
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let current_ptr = &mut *current as *mut TaskStruct;

        current.__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            Ordering::Release,
        );
        let stack_top = crate::kernel::sched::KTHREAD_STACK_SIZE * 2;
        current.stack = stack_top as *mut core::ffi::c_void;
        current.thread.sp = stack_top as u64 - 64;
        current.m29.sched_class = &FAIR_SCHED_CLASS as *const SchedClass;
        current.m29.se.load.weight = NICE_0_LOAD;
        current.m29.se.on_rq = 1;
        current.m29.on_rq = 1;

        rq.current = current_ptr;
        rq.cfs.current = current_ptr;
        rq.cfs.nr_running = 1;
        rq.cfs.load_weight = NICE_0_LOAD;
        rq.nr_running = 1;

        assert_eq!(unsafe { pick_next_task_fair(&mut rq) }, current_ptr);
    }
}
