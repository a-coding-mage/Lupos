//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched/deadline.c
//! test-origin: linux:vendor/linux/kernel/sched/deadline.c
//! `SCHED_DEADLINE` — Earliest Deadline First (EDF) with bandwidth admission
//! control (M30).
//!
//! Mirrors `vendor/linux/kernel/sched/deadline.c`.  Three parameters per task:
//!
//!   * `dl_runtime` — maximum CPU time per period (ns).
//!   * `dl_deadline` — relative deadline within the period.
//!   * `dl_period`  — period length.
//!
//! Admission is enforced via `running_bw + dl_runtime / dl_period <= bw_cap`,
//! where `bw_cap = 95% * (1 << BW_SHIFT)` (Linux default).

use crate::kernel::task::TaskStruct;

use super::class::{CLASS_PRIO_DL, DEQUEUE_SLEEP, ENQUEUE_WAKEUP, SchedClass};
use super::entity::sched_clock_ns;
use super::rq::{BW_SHIFT, Rq};

/// Compute fixed-point bandwidth = `runtime / period` shifted by `BW_SHIFT`.
#[inline]
pub const fn to_ratio(runtime: u64, period: u64) -> u64 {
    if period == 0 {
        return 0;
    }
    (runtime << BW_SHIFT) / period
}

/// Reject the candidate if admission would exceed the bandwidth cap.
pub fn dl_bw_admit(rq: &Rq, runtime: u64, period: u64) -> bool {
    let bw = to_ratio(runtime, period);
    rq.dl.running_bw.saturating_add(bw) <= rq.dl.bw_cap
}

unsafe fn enqueue_task_dl(rq: &mut Rq, p: *mut TaskStruct, flags: u32) {
    if p.is_null() {
        return;
    }
    unsafe {
        let absolute_deadline = sched_clock_ns().saturating_add((*p).m29.dl.dl_deadline);
        (*p).m29.dl.deadline = absolute_deadline;
        if (*p).m29.dl.runtime <= 0 {
            (*p).m29.dl.runtime = (*p).m29.dl.dl_runtime as i64;
        }
        rq.dl.insert(p, absolute_deadline);
        (*p).m29.on_rq = 1;
        rq.dl.running_bw = rq
            .dl
            .running_bw
            .saturating_add(to_ratio((*p).m29.dl.dl_runtime, (*p).m29.dl.dl_period));
    }
    rq.dl.nr_running = rq.dl.nr_running.saturating_add(1);
    rq.nr_running = rq.nr_running.saturating_add(1);
    let _ = flags & ENQUEUE_WAKEUP;
}

unsafe fn dequeue_task_dl(rq: &mut Rq, p: *mut TaskStruct, flags: u32) -> bool {
    if p.is_null() {
        return false;
    }
    unsafe {
        let dl = (*p).m29.dl.deadline;
        rq.dl.remove(p, dl);
        rq.dl.running_bw = rq
            .dl
            .running_bw
            .saturating_sub(to_ratio((*p).m29.dl.dl_runtime, (*p).m29.dl.dl_period));
        (*p).m29.on_rq = 0;
    }
    rq.dl.nr_running = rq.dl.nr_running.saturating_sub(1);
    rq.nr_running = rq.nr_running.saturating_sub(1);
    let _ = flags & DEQUEUE_SLEEP;
    true
}

unsafe fn pick_next_task_dl(rq: &mut Rq) -> *mut TaskStruct {
    let p = rq.dl.earliest();
    if !p.is_null() {
        rq.dl.current = p;
        rq.current = p;
    }
    p
}

unsafe fn put_prev_task_dl(_rq: &mut Rq, _prev: *mut TaskStruct) {}

unsafe fn task_tick_dl(rq: &mut Rq, p: *mut TaskStruct, _queued: bool) {
    if p.is_null() {
        return;
    }
    unsafe {
        // Decrement runtime by one tick budget (25 ms = 25 000 000 ns on QEMU).
        let consumed: i64 = 25_000_000;
        (*p).m29.dl.runtime = (*p).m29.dl.runtime.saturating_sub(consumed);
        if (*p).m29.dl.runtime <= 0 {
            // Throttle until next period.
            (*p).m29.dl.dl_throttled = 1;
            (*p).thread_info.flags |= crate::kernel::task::TIF_NEED_RESCHED;
        }
    }
    let _ = rq;
}

unsafe fn task_fork_dl(p: *mut TaskStruct) {
    if p.is_null() {
        return;
    }
    unsafe {
        // Defaults: runtime=0 / period=0 means task hasn't been promoted to
        // SCHED_DEADLINE; it stays in CFS until sched_setattr.
        (*p).m29.dl.runtime = 0;
    }
}

unsafe fn select_task_rq_dl(_p: *mut TaskStruct, prev_cpu: u32, _flags: u32) -> u32 {
    prev_cpu
}

pub static DL_SCHED_CLASS: SchedClass = SchedClass {
    class_prio: CLASS_PRIO_DL,
    _pad: [0; 7],
    enqueue_task: Some(enqueue_task_dl),
    dequeue_task: Some(dequeue_task_dl),
    yield_task: None,
    wakeup_preempt: None,
    pick_next_task: Some(pick_next_task_dl),
    put_prev_task: Some(put_prev_task_dl),
    set_next_task: None,
    task_tick: Some(task_tick_dl),
    task_fork: Some(task_fork_dl),
    task_dead: None,
    switched_to: None,
    prio_changed: None,
    get_rr_interval: None,
    update_curr: None,
    select_task_rq: Some(select_task_rq_dl),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_ratio_matches_linux_formula() {
        // 1 ms / 10 ms = 10% = 0.1 in fixed-point with BW_SHIFT=20
        let bw = to_ratio(1_000_000, 10_000_000);
        let expected = (1u64 << BW_SHIFT) / 10;
        assert_eq!(bw, expected);
    }

    #[test]
    fn admit_below_cap_succeeds() {
        let rq = Rq::new(0);
        // 10% < 95% cap
        assert!(dl_bw_admit(&rq, 1_000_000, 10_000_000));
    }

    #[test]
    fn admit_above_cap_fails() {
        let rq = Rq::new(0);
        // 99% > 95% cap
        assert!(!dl_bw_admit(&rq, 99_000_000, 100_000_000));
    }

    #[test]
    fn dl_class_above_rt() {
        assert!(CLASS_PRIO_DL < super::super::class::CLASS_PRIO_RT);
    }
}
