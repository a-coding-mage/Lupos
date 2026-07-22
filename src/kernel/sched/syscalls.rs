//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched/syscalls.c
//! test-origin: linux:vendor/linux/kernel/sched/syscalls.c
//! Scheduler syscalls — `sched_setattr`, `sched_setscheduler`, `sched_yield`,
//! `sched_get_priority_{min,max}`, `sched_rr_get_interval` (M30).
//!
//! UAPI structures and `errno` returns are byte-for-byte parity with
//! `vendor/linux/include/uapi/linux/sched/types.h` and the corresponding
//! `kernel/sched/syscalls.c` paths.
//!
//! On-rq policy changes use the same dequeue/change/enqueue transaction as
//! Linux `sched_change_begin()` / `sched_change_end()`. Fair-policy changes
//! still lack Linux's full `reweight_entity()` load-accounting update.

use crate::kernel::task::TaskStruct;

use super::class::SchedClass;
use super::deadline::DL_SCHED_CLASS;
use super::fair::FAIR_SCHED_CLASS;
use super::prio::{
    DEFAULT_PRIO, MAX_NICE, MAX_RT_PRIO, MIN_NICE, SCHED_BATCH, SCHED_DEADLINE, SCHED_FIFO,
    SCHED_IDLE, SCHED_NORMAL, SCHED_RESET_ON_FORK, SCHED_RR,
};
use super::rt::RT_SCHED_CLASS;

// ── UAPI: struct sched_attr (vendor/linux/include/uapi/linux/sched/types.h) ──

/// `SCHED_ATTR_SIZE_VER0` — original 48-byte layout.
pub const SCHED_ATTR_SIZE_VER0: u32 = 48;
/// `SCHED_ATTR_SIZE_VER1` — adds `util_min` / `util_max` (56 bytes).
pub const SCHED_ATTR_SIZE_VER1: u32 = 56;

/// Linux `struct sched_attr` — UAPI for `sched_setattr` / `sched_getattr`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SchedAttr {
    pub size: u32,
    pub sched_policy: u32,
    pub sched_flags: u64,
    pub sched_nice: i32,
    pub sched_priority: u32,
    pub sched_runtime: u64,
    pub sched_deadline: u64,
    pub sched_period: u64,
    pub sched_util_min: u32,
    pub sched_util_max: u32,
}

const _: () = assert!(core::mem::size_of::<SchedAttr>() == SCHED_ATTR_SIZE_VER1 as usize);

// ── errno values referenced ──────────────────────────────────────────────────

pub const EINVAL: i32 = 22;
pub const EPERM: i32 = 1;
pub const ESRCH: i32 = 3;
pub const E2BIG: i32 = 7;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Map a `SCHED_*` policy to its `sched_class` vtable.
pub fn class_for_policy(policy: u32) -> Option<&'static SchedClass> {
    match policy & !SCHED_RESET_ON_FORK {
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => Some(&FAIR_SCHED_CLASS),
        SCHED_FIFO | SCHED_RR => Some(&RT_SCHED_CLASS),
        SCHED_DEADLINE => Some(&DL_SCHED_CLASS),
        _ => None,
    }
}

/// Convert (`policy`, `rt_priority`, `nice`) into the effective Linux
/// `prio` value (`MAX_RT_PRIO - 1 - rt_priority` for RT, `DEFAULT_PRIO + nice`
/// for normal classes, `0` for DL).
pub fn effective_prio(policy: u32, rt_priority: u32, nice: i32) -> i32 {
    match policy & !SCHED_RESET_ON_FORK {
        SCHED_FIFO | SCHED_RR => MAX_RT_PRIO - 1 - rt_priority as i32,
        SCHED_DEADLINE => -1,
        _ => DEFAULT_PRIO + nice,
    }
}

// ── sys_sched_setattr ────────────────────────────────────────────────────────

unsafe fn apply_sched_attr_fields(
    p: *mut TaskStruct,
    attr: &SchedAttr,
    policy: u32,
    next_class: *const SchedClass,
    new_prio: i32,
) {
    unsafe {
        (*p).m29.policy = policy;
        (*p).m29.rt_priority = attr.sched_priority;
        (*p).m29.static_prio = DEFAULT_PRIO + attr.sched_nice;
        (*p).m29.normal_prio = new_prio;
        (*p).m29.prio = new_prio;
        (*p).m29.dl.dl_runtime = attr.sched_runtime;
        (*p).m29.dl.dl_deadline = if attr.sched_deadline != 0 {
            attr.sched_deadline
        } else {
            attr.sched_period
        };
        (*p).m29.dl.dl_period = attr.sched_period;
        (*p).m29.sched_class = next_class;
    }
}

/// Apply a `sched_attr` to a task.  Returns 0 on success, negative `errno` on
/// failure.
pub unsafe fn sys_sched_setattr(p: *mut TaskStruct, attr: &SchedAttr) -> i32 {
    if p.is_null() {
        return -ESRCH;
    }
    if attr.size < SCHED_ATTR_SIZE_VER0 || attr.size > SCHED_ATTR_SIZE_VER1 {
        return -E2BIG;
    }
    let policy = attr.sched_policy;
    if class_for_policy(policy).is_none() {
        return -EINVAL;
    }
    match policy & !SCHED_RESET_ON_FORK {
        SCHED_FIFO | SCHED_RR => {
            if attr.sched_priority < 1 || attr.sched_priority >= MAX_RT_PRIO as u32 {
                return -EINVAL;
            }
        }
        SCHED_DEADLINE => {
            if attr.sched_runtime == 0 || attr.sched_period == 0 {
                return -EINVAL;
            }
            if attr.sched_runtime > attr.sched_deadline.max(attr.sched_period) {
                return -EINVAL;
            }
        }
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => {
            if attr.sched_nice < MIN_NICE || attr.sched_nice > MAX_NICE {
                return -EINVAL;
            }
        }
        _ => return -EINVAL,
    }

    let next_class = class_for_policy(policy).unwrap() as *const SchedClass;
    let new_prio = effective_prio(policy, attr.sched_priority, attr.sched_nice);
    unsafe {
        super::change_task_scheduler(p, next_class, new_prio, |task| {
            apply_sched_attr_fields(task, attr, policy, next_class, new_prio);
        });
    }
    0
}

/// Read the current sched_attr of a task into `out`.
pub unsafe fn sys_sched_getattr(p: *mut TaskStruct, out: &mut SchedAttr) -> i32 {
    if p.is_null() {
        return -ESRCH;
    }
    out.size = SCHED_ATTR_SIZE_VER1;
    unsafe {
        out.sched_policy = (*p).m29.policy;
        out.sched_flags = 0;
        out.sched_nice = (*p).m29.static_prio - DEFAULT_PRIO;
        out.sched_priority = (*p).m29.rt_priority;
        out.sched_runtime = (*p).m29.dl.dl_runtime;
        out.sched_deadline = (*p).m29.dl.dl_deadline;
        out.sched_period = (*p).m29.dl.dl_period;
        out.sched_util_min = 0;
        out.sched_util_max = 1024;
    }
    0
}

/// Linux `sched_setscheduler(p, policy, sched_param)`.  Returns 0 / -errno.
pub unsafe fn sys_sched_setscheduler(p: *mut TaskStruct, policy: u32, priority: u32) -> i32 {
    let attr = SchedAttr {
        size: SCHED_ATTR_SIZE_VER1,
        sched_policy: policy,
        sched_priority: priority,
        ..SchedAttr::default()
    };
    unsafe { sys_sched_setattr(p, &attr) }
}

/// Linux `sched_getscheduler(pid)` — return policy or -errno.
pub unsafe fn sys_sched_getscheduler(p: *mut TaskStruct) -> i32 {
    if p.is_null() {
        return -ESRCH;
    }
    unsafe { (*p).m29.policy as i32 }
}

/// Linux `sched_get_priority_max(policy)`.
pub fn sys_sched_get_priority_max(policy: u32) -> i32 {
    match policy & !SCHED_RESET_ON_FORK {
        SCHED_FIFO | SCHED_RR => 99,
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => 0,
        _ => -EINVAL,
    }
}

/// Linux `sched_get_priority_min(policy)`.
pub fn sys_sched_get_priority_min(policy: u32) -> i32 {
    match policy & !SCHED_RESET_ON_FORK {
        SCHED_FIFO | SCHED_RR => 1,
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => 0,
        _ => -EINVAL,
    }
}

/// Linux `sched_rr_get_interval(pid, &tv)` — only meaningful for SCHED_RR.
/// Returns the time-slice in nanoseconds, or 0 for non-RR policies.
pub unsafe fn sys_sched_rr_get_interval(p: *mut TaskStruct) -> u64 {
    if p.is_null() {
        return 0;
    }
    unsafe {
        if (*p).m29.policy == SCHED_RR {
            super::rt::RR_TIMESLICE_NS
        } else {
            0
        }
    }
}

/// Linux `sched_yield()` — request voluntary CPU release.
pub unsafe fn sys_sched_yield() -> i32 {
    #[cfg(test)]
    {
        return 0;
    }
    #[cfg(not(test))]
    unsafe {
        super::schedule_with_irqs_enabled();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn sched_attr_is_56_bytes() {
        assert_eq!(core::mem::size_of::<SchedAttr>(), 56);
    }

    #[test]
    fn priority_max_for_rt_is_99() {
        assert_eq!(sys_sched_get_priority_max(SCHED_FIFO), 99);
        assert_eq!(sys_sched_get_priority_max(SCHED_RR), 99);
    }

    #[test]
    fn priority_min_for_rt_is_1() {
        assert_eq!(sys_sched_get_priority_min(SCHED_FIFO), 1);
        assert_eq!(sys_sched_get_priority_min(SCHED_RR), 1);
    }

    #[test]
    fn priority_max_for_normal_is_0() {
        assert_eq!(sys_sched_get_priority_max(SCHED_NORMAL), 0);
    }

    #[test]
    fn unknown_policy_returns_einval() {
        assert_eq!(sys_sched_get_priority_max(42), -EINVAL);
    }

    #[test]
    fn class_lookup_maps_policies() {
        assert!(core::ptr::eq(
            class_for_policy(SCHED_NORMAL).unwrap(),
            &super::super::fair::FAIR_SCHED_CLASS,
        ));
        assert!(core::ptr::eq(
            class_for_policy(SCHED_FIFO).unwrap(),
            &super::super::rt::RT_SCHED_CLASS,
        ));
        assert!(core::ptr::eq(
            class_for_policy(SCHED_DEADLINE).unwrap(),
            &super::super::deadline::DL_SCHED_CLASS,
        ));
    }

    #[test]
    fn effective_prio_rt_is_max_rt_minus_priority() {
        // SCHED_FIFO with rt_priority 50 → prio = 100 - 1 - 50 = 49
        assert_eq!(effective_prio(SCHED_FIFO, 50, 0), 49);
    }

    #[test]
    fn effective_prio_normal_is_default_plus_nice() {
        assert_eq!(effective_prio(SCHED_NORMAL, 0, 5), DEFAULT_PRIO + 5);
        assert_eq!(effective_prio(SCHED_NORMAL, 0, -10), DEFAULT_PRIO - 10);
    }

    // test-origin: linux:vendor/linux/kernel/sched/syscalls.c:__sched_setscheduler
    // Linux has no userspace selftest for the internal dequeue/change/enqueue
    // transaction. This checks the queue-membership invariant exposed by a
    // runnable PipeWire thread changing from CFS to SCHED_FIFO.
    #[test]
    fn runnable_policy_change_moves_task_between_class_runqueues() {
        const TEST_CPU: u32 = (super::super::MAX_CPUS - 1) as u32;

        struct ResetRunqueue(u32);

        impl Drop for ResetRunqueue {
            fn drop(&mut self) {
                let _ = super::super::rq::with_rq(self.0, |rq| {
                    *rq = super::super::rq::Rq::new(self.0);
                });
            }
        }

        super::super::rq::init_rqs();
        super::super::rq::with_rq(TEST_CPU, |rq| {
            *rq = super::super::rq::Rq::new(TEST_CPU);
        })
        .expect("test runqueue exists");
        let _reset_runqueue = ResetRunqueue(TEST_CPU);

        let mut task = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let task_ptr = &mut *task as *mut TaskStruct;
        task.m29 = crate::kernel::task::M29SchedFields::zeroed();
        task.m29.sched_class = &super::super::fair::FAIR_SCHED_CLASS;
        task.m29.policy = SCHED_NORMAL;
        task.m29.cpus_mask = super::super::entity::CpuMask::one(TEST_CPU);
        task.m29.cpus_ptr = &task.m29.cpus_mask;
        task.m29.nr_cpus_allowed = 1;
        task.thread_info.cpu = TEST_CPU;

        unsafe {
            super::super::enqueue_on_rq(TEST_CPU, task_ptr, super::super::class::ENQUEUE_INITIAL);
        }
        assert_eq!(task.m29.se.on_rq, 1);

        let attr = SchedAttr {
            size: SCHED_ATTR_SIZE_VER1,
            sched_policy: SCHED_FIFO,
            sched_priority: 88,
            ..SchedAttr::default()
        };
        assert_eq!(unsafe { sys_sched_setattr(task_ptr, &attr) }, 0);

        assert_eq!(task.m29.on_rq, 1);
        assert_eq!(task.m29.se.on_rq, 0);
        assert_eq!(task.m29.rt.on_rq, 1);
        super::super::rq::with_rq(TEST_CPU, |rq| {
            assert_eq!(rq.cfs.nr_running, 0);
            assert_eq!(rq.rt.nr_running, 1);
            assert_eq!(rq.nr_running, 1);
        })
        .expect("test runqueue exists");
    }
}
