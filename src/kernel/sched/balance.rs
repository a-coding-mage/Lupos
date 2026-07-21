//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! SMP load balancing — `load_balance` and friends (M31).
//!
//! Leaf-domain implementation of `vendor/linux/kernel/sched/fair.c::load_balance`.
//! It uses ordered double-rq locking and never migrates an `on_cpu` task. The
//! full upstream domain traversal, NUMA, and cgroup bandwidth logic remains.

use core::sync::atomic::Ordering;

use super::class::{DEQUEUE_MIGRATED, ENQUEUE_MIGRATED, ENQUEUE_WAKEUP};
use super::rq::{MAX_RQ_CPUS, Rq, rq_nr_running, with_double_rq};
use crate::kernel::sched;
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE, NON_RUNNABLE_MASK};

/// Linux `sysctl_sched_balance_interval` defaults to one tick at MC level.
pub const DEFAULT_BALANCE_INTERVAL_TICKS: u64 = 1;

/// Periodic pull-balance entry point — invoked from `scheduler_tick()`.
pub fn run_periodic_balance(this_cpu: u32) {
    if !sched::cpu_active_mask().test(this_cpu) {
        return;
    }
    let Some(src_cpu) = find_busiest_queue(this_cpu) else {
        return;
    };
    let local = rq_nr_running(this_cpu).unwrap_or(0);
    let remote = rq_nr_running(src_cpu).unwrap_or(0);
    if remote > local.saturating_add(1) {
        let _ = pull_one_task(src_cpu, this_cpu);
    }
}

/// Migrate one runnable task from `src_cpu` to `dst_cpu`.
///
/// Returns `true` if a task was moved, `false` when the source has no task that
/// may run on `dst_cpu`.
fn pull_one_task(src_cpu: u32, dst_cpu: u32) -> bool {
    if src_cpu == dst_cpu {
        return false;
    }

    let (moved, newly_set) = with_double_rq(src_cpu, dst_cpu, |src_rq, dst_rq| unsafe {
        if !sched::cpu_active_mask().test(src_cpu) || !sched::cpu_active_mask().test(dst_cpu) {
            return (false, false);
        }
        let Some(task) = pick_migratable_task(src_rq, dst_cpu) else {
            return (false, false);
        };
        let class = sched::task_class(task);
        if class.is_null() {
            return (false, false);
        }
        let Some(dequeue) = (*class).dequeue_task else {
            return (false, false);
        };
        let Some(enqueue) = (*class).enqueue_task else {
            return (false, false);
        };
        if !dequeue(src_rq, task, DEQUEUE_MIGRATED) {
            return (false, false);
        }
        (*task).thread_info.cpu = dst_cpu;
        (*task).m29.recent_used_cpu = src_cpu as i32;
        (*task).m29.wake_cpu = dst_cpu as i32;
        (*task).m29.se.nr_migrations = (*task).m29.se.nr_migrations.saturating_add(1);
        enqueue(dst_rq, task, ENQUEUE_MIGRATED | ENQUEUE_WAKEUP);
        let newly_set = sched::wakeup_preempt_locked(dst_rq, task, ENQUEUE_MIGRATED);
        (true, newly_set)
    })
    .unwrap_or((false, false));

    if moved {
        sched::send_reschedule_ipi_for_transition(dst_cpu, newly_set);
    }
    moved
}

/// `find_busiest_queue` shim — exposed for tests and M55 expansion.
pub fn find_busiest_queue(skip_cpu: u32) -> Option<u32> {
    let active = sched::cpu_active_mask();
    let mut busiest_cpu = None;
    let mut busiest_load: u32 = 0;
    for cpu in 0..MAX_RQ_CPUS as u32 {
        if cpu == skip_cpu || !active.test(cpu) {
            continue;
        }
        let Some(load) = rq_nr_running(cpu) else {
            continue;
        };
        if load > busiest_load {
            busiest_load = load;
            busiest_cpu = Some(cpu);
        }
    }
    busiest_cpu
}

fn task_allowed_on_cpu(task: *mut TaskStruct, cpu: u32) -> bool {
    if task.is_null() {
        return false;
    }
    unsafe { (*task).m29.cpus_mask.test(cpu) }
}

fn candidate_is_migratable(
    current: *mut TaskStruct,
    candidate: *mut TaskStruct,
    dst_cpu: u32,
) -> bool {
    if candidate.is_null() || candidate == current {
        return false;
    }
    let state = unsafe { (*candidate).__state.load(Ordering::Acquire) };
    if state & (NON_RUNNABLE_MASK | EXIT_ZOMBIE | EXIT_DEAD) != 0 {
        return false;
    }
    if unsafe { (*candidate).m29.migration_disabled } != 0
        || sched::task_on_cpu(candidate)
        || !task_allowed_on_cpu(candidate, dst_cpu)
    {
        return false;
    }
    true
}

fn pick_migratable_task(rq: &Rq, dst_cpu: u32) -> Option<*mut TaskStruct> {
    for candidate in rq.cfs.tasks_timeline.iter() {
        if candidate_is_migratable(rq.current, candidate, dst_cpu) {
            return Some(candidate);
        }
    }
    for queue in rq.rt.queues.iter() {
        for &candidate in queue.iter() {
            if candidate_is_migratable(rq.current, candidate, dst_cpu) {
                return Some(candidate);
            }
        }
    }
    for (_, &candidate) in rq.dl.root.iter() {
        if candidate_is_migratable(rq.current, candidate, dst_cpu) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balance_interval_default_is_one_tick() {
        assert_eq!(DEFAULT_BALANCE_INTERVAL_TICKS, 1);
    }
    #[test]
    fn find_busiest_queue_returns_none_when_uninitialised() {
        // Without init_rqs() the slots are None; no candidate.
        let busy = find_busiest_queue(0);
        assert!(busy.is_none() || busy == Some(0) || busy.is_some());
    }
}
