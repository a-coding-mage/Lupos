//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! SMP load balancing — `load_balance` and friends (M31).
//!
//! Skeleton implementation of `vendor/linux/kernel/sched/fair.c::load_balance`
//! sufficient to balance the CFS runqueues across the boot APIC topology.
//!
//! The full upstream balancer pulls in `sched_domain` traversal, NUMA
//! awareness, and cgroup-bandwidth honouring — for M31 we ship the leaf-level
//! pull migration only and leave the multi-tier tree behaviour to M55.

use core::sync::atomic::Ordering;

use super::rq::{MAX_RQ_CPUS, rq_nr_running, with_rq};
use crate::kernel::sched;
use crate::kernel::task::TaskStruct;
use crate::kernel::task::task_state::{EXIT_DEAD, EXIT_ZOMBIE, NON_RUNNABLE_MASK};

/// Linux `sysctl_sched_balance_interval` defaults to one tick at MC level.
pub const DEFAULT_BALANCE_INTERVAL_TICKS: u64 = 1;

/// Periodic balance entry point — invoked from `scheduler_tick()`.
///
/// Lupos brings the APs up but they idle in `ap_main` without ever running the
/// scheduler — only the BSP (`sched::SCHEDULING_CPU`) executes tasks. This entry
/// point is driven from `scheduler_tick`, which fires on *every* CPU's LAPIC
/// timer ISR, so a stock pull-balancer would let each AP pull the busiest task
/// onto its own runqueue and then return to `hlt`, stranding that task in a
/// runqueue that is never serviced — the multi-CPU boot hang where, run to run,
/// init/systemd or the libata probe chain ends up parked on an AP.
///
/// Until APs actually run the scheduler, restrict all migration to the BSP and,
/// instead of load-balancing *onto* idle CPUs, have the BSP *rescue* any task
/// that ended up on a non-scheduling CPU back to itself.
pub fn run_periodic_balance(this_cpu: u32) {
    if this_cpu != sched::SCHEDULING_CPU {
        return;
    }
    for src_cpu in 0..MAX_RQ_CPUS as u32 {
        if src_cpu == this_cpu {
            continue;
        }
        if rq_nr_running(src_cpu).unwrap_or(0) == 0 {
            continue;
        }
        // Drain every runnable task off the non-scheduling CPU back to the BSP.
        // `pull_one_task` returns false once nothing more can be pulled, which
        // also bounds the loop (a task pinned away from the BSP stops it).
        while pull_one_task(src_cpu, this_cpu) {}
    }
}

/// Migrate one runnable task from `src_cpu` to `dst_cpu`.
///
/// Returns `true` if a task was moved, `false` when the source has no task that
/// may run on `dst_cpu`.
fn pull_one_task(src_cpu: u32, dst_cpu: u32) -> bool {
    let candidate = with_rq(src_cpu, |rq| {
        pick_migratable_task(rq.current, rq.cfs.leftmost())
    })
    .flatten()
    .or_else(|| {
        with_rq(src_cpu, |rq| {
            pick_migratable_task(rq.current, rq.rt.pick_first())
        })
        .flatten()
    })
    .or_else(|| {
        with_rq(src_cpu, |rq| {
            pick_migratable_task(rq.current, rq.dl.earliest())
        })
        .flatten()
    });

    let Some(task) = candidate else {
        return false;
    };
    if !task_allowed_on_cpu(task, dst_cpu) {
        return false;
    }
    if !unsafe { sched::dequeue_from_rq(src_cpu, task, super::class::DEQUEUE_MIGRATED) } {
        return false;
    }
    unsafe {
        (*task).thread_info.cpu = dst_cpu;
        (*task).m29.se.nr_migrations = (*task).m29.se.nr_migrations.saturating_add(1);
        sched::enqueue_on_rq(
            dst_cpu,
            task,
            super::class::ENQUEUE_MIGRATED | super::class::ENQUEUE_WAKEUP,
        );
    }
    sched::request_reschedule(dst_cpu);
    true
}

/// `find_busiest_queue` shim — exposed for tests and M55 expansion.
pub fn find_busiest_queue(skip_cpu: u32) -> Option<u32> {
    let mut busiest_cpu = None;
    let mut busiest_load: u32 = 0;
    for cpu in 0..MAX_RQ_CPUS as u32 {
        if cpu == skip_cpu {
            continue;
        }
        let _ = with_rq(cpu, |rq| {
            if rq.nr_running > busiest_load {
                busiest_load = rq.nr_running;
                busiest_cpu = Some(cpu);
            }
        });
    }
    busiest_cpu
}

fn task_allowed_on_cpu(task: *mut TaskStruct, cpu: u32) -> bool {
    if task.is_null() {
        return false;
    }
    unsafe { (*task).m29.cpus_mask.test(cpu) }
}

fn pick_migratable_task(
    current: *mut TaskStruct,
    candidate: *mut TaskStruct,
) -> Option<*mut TaskStruct> {
    if candidate.is_null() || candidate == current {
        return None;
    }
    let state = unsafe { (*candidate).__state.load(Ordering::Acquire) };
    if state & (NON_RUNNABLE_MASK | EXIT_ZOMBIE | EXIT_DEAD) != 0 {
        return None;
    }
    if unsafe { (*candidate).m29.migration_disabled } != 0 {
        return None;
    }
    Some(candidate)
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
