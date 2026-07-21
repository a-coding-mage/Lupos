//! linux-parity: partial
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

/// One pass through Linux's generic `do_idle()` loop.
///
/// The final reschedule/softirq checks run with local IRQs disabled and are
/// immediately followed by the contiguous `sti; hlt` pair. A wakeup that races
/// with the check therefore either prevents the halt or leaves a pending IPI
/// that wakes it without a lost-wakeup window.
fn do_idle(cpu: u32) {
    super::nohz::tick_nohz_idle_enter(cpu);
    loop {
        crate::kernel::watchdog::touch_softlockup_watchdog_sched();
        crate::kernel::rcu::tasks_rcu_qs();
        crate::kernel::rcu::rcu_qs();

        if super::current_needs_resched() {
            break;
        }
        if crate::kernel::softirq::local_softirq_pending() != 0 {
            super::nohz::tick_nohz_idle_exit(cpu);
            crate::kernel::softirq::do_softirq();
            super::nohz::tick_nohz_idle_enter(cpu);
            continue;
        }

        crate::kernel::locking::local_irq_disable();
        if super::current_needs_resched() || crate::kernel::softirq::local_softirq_pending() != 0 {
            crate::kernel::locking::local_irq_enable();
            continue;
        }

        #[cfg(not(test))]
        unsafe {
            core::arch::asm!("sti; hlt", options(nostack));
        }
        #[cfg(test)]
        {
            crate::kernel::locking::local_irq_enable();
            break;
        }
    }
    super::nohz::tick_nohz_idle_exit(cpu);
    unsafe {
        super::schedule_idle();
    }
}

/// Enter the per-CPU idle thread after architecture bring-up is complete.
///
/// The architecture must initialize the exact idle stack, current pointer,
/// TSS/GS/FPU state, local APIC, and clockevent, then call
/// `sched_activate_cpu()` before publishing its ready state or enabling local
/// interrupts. This entry verifies that publication before entering idle.
pub fn cpu_startup_entry() -> ! {
    let cpu = super::current_cpu();
    assert!(
        super::cpu_active_mask().test(cpu),
        "CPU entered idle before scheduler activation"
    );
    crate::kernel::locking::local_irq_enable();
    loop {
        do_idle(cpu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_class_priority_is_lowest() {
        assert_eq!(IDLE_SCHED_CLASS.class_prio, CLASS_PRIO_IDLE);
    }
}
