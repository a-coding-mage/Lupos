//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sched
//! test-origin: linux:vendor/linux/kernel/sched
//! `struct sched_class` — function-pointer dispatch table.
//!
//! Mirrors `vendor/linux/kernel/sched/sched.h::struct sched_class`.  Each
//! scheduling policy (`fair`, `rt`, `dl`, `idle`, `stop`) provides a static
//! `SchedClass` instance that the core scheduler dispatches through.
//!
//! Class priority order (highest to lowest), used by `pick_next_task`:
//!
//!   stop  >  dl  >  rt  >  fair  >  idle
//!
//! References:
//!   * `vendor/linux/kernel/sched/sched.h:2519`
//!   * Linux 7.x sched_class linkage: section `__sched_class_highest`..lowest.

use crate::kernel::task::TaskStruct;

/// Flags accepted by `enqueue_task` / `dequeue_task` (subset of Linux `ENQUEUE_*`).
pub const ENQUEUE_WAKEUP: u32 = 0x01;
pub const ENQUEUE_RESTORE: u32 = 0x02;
pub const ENQUEUE_MOVE: u32 = 0x04;
pub const ENQUEUE_HEAD: u32 = 0x0001_0000;
pub const ENQUEUE_NOCLOCK: u32 = 0x08;
pub const ENQUEUE_CLASS: u32 = 0x40;
pub const ENQUEUE_MIGRATED: u32 = 0x0004_0000;
pub const ENQUEUE_INITIAL: u32 = 0x0008_0000;

pub const DEQUEUE_SLEEP: u32 = 0x01;
pub const DEQUEUE_SAVE: u32 = 0x02;
pub const DEQUEUE_MOVE: u32 = 0x04;
pub const DEQUEUE_NOCLOCK: u32 = 0x08;
pub const DEQUEUE_MIGRATING: u32 = 0x0010;
pub const DEQUEUE_CLASS: u32 = 0x40;

/// Fork placement flag passed to `select_task_rq()` / `wakeup_preempt()`.
/// This is a wake flag, not an `ENQUEUE_*` flag.
pub const WF_FORK: u32 = 0x04;

/// Forward declaration — concrete `Rq` lives in `rq.rs`.
pub type Rq = crate::kernel::sched::rq::Rq;

/// `struct sched_class` — function-pointer dispatch table.
///
/// All callbacks may be NULL; the core scheduler tests for NULL before invoking.
/// Layout intentionally mirrors `vendor/linux/kernel/sched/sched.h:2519`.
#[repr(C)]
pub struct SchedClass {
    /// Class priority — `0` = highest (stop), `4` = lowest (idle).
    /// Used by `pick_next_task` linear scan.
    pub class_prio: u8,
    pub _pad: [u8; 7],

    pub enqueue_task: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct, flags: u32)>,
    pub dequeue_task: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct, flags: u32) -> bool>,
    pub yield_task: Option<unsafe fn(rq: &mut Rq)>,
    pub wakeup_preempt: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct, flags: u32)>,
    pub pick_next_task: Option<unsafe fn(rq: &mut Rq) -> *mut TaskStruct>,
    pub put_prev_task: Option<unsafe fn(rq: &mut Rq, prev: *mut TaskStruct)>,
    pub set_next_task: Option<unsafe fn(rq: &mut Rq, next: *mut TaskStruct, first: bool)>,
    pub task_tick: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct, queued: bool)>,
    pub task_fork: Option<unsafe fn(p: *mut TaskStruct)>,
    pub task_dead: Option<unsafe fn(p: *mut TaskStruct)>,
    pub switched_to: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct)>,
    pub prio_changed: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct, old_prio: i32)>,
    pub get_rr_interval: Option<unsafe fn(rq: &mut Rq, p: *mut TaskStruct) -> u64>,
    pub update_curr: Option<unsafe fn(rq: &mut Rq)>,
    pub select_task_rq: Option<unsafe fn(p: *mut TaskStruct, prev_cpu: u32, flags: u32) -> u32>,
}

unsafe impl Send for SchedClass {}
unsafe impl Sync for SchedClass {}

impl SchedClass {
    /// All-NULL skeleton.  Used as a building block for concrete classes.
    pub const fn empty(class_prio: u8) -> Self {
        Self {
            class_prio,
            _pad: [0; 7],
            enqueue_task: None,
            dequeue_task: None,
            yield_task: None,
            wakeup_preempt: None,
            pick_next_task: None,
            put_prev_task: None,
            set_next_task: None,
            task_tick: None,
            task_fork: None,
            task_dead: None,
            switched_to: None,
            prio_changed: None,
            get_rr_interval: None,
            update_curr: None,
            select_task_rq: None,
        }
    }
}

/// Class priorities — lower number wins.  Mirrors Linux's
/// `__sched_class_highest..__sched_class_lowest` linker ordering.
pub const CLASS_PRIO_STOP: u8 = 0;
pub const CLASS_PRIO_DL: u8 = 1;
pub const CLASS_PRIO_RT: u8 = 2;
pub const CLASS_PRIO_FAIR: u8 = 3;
pub const CLASS_PRIO_IDLE: u8 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_prio_ordering_matches_linux() {
        assert!(CLASS_PRIO_STOP < CLASS_PRIO_DL);
        assert!(CLASS_PRIO_DL < CLASS_PRIO_RT);
        assert!(CLASS_PRIO_RT < CLASS_PRIO_FAIR);
        assert!(CLASS_PRIO_FAIR < CLASS_PRIO_IDLE);
    }

    #[test]
    fn empty_class_is_all_null() {
        let c = SchedClass::empty(CLASS_PRIO_FAIR);
        assert!(c.enqueue_task.is_none());
        assert!(c.pick_next_task.is_none());
    }

    #[test]
    fn new_task_flags_match_linux_sched_h() {
        assert_eq!(WF_FORK, 0x04);
        assert_eq!(ENQUEUE_NOCLOCK, 0x0008);
        assert_eq!(ENQUEUE_INITIAL, 0x0008_0000);
        assert_eq!(WF_FORK & ENQUEUE_INITIAL, 0);
    }

    #[test]
    fn queue_head_and_migration_flags_match_linux_sched_h() {
        assert_eq!(DEQUEUE_SAVE, 0x0002);
        assert_eq!(DEQUEUE_MOVE, 0x0004);
        assert_eq!(DEQUEUE_CLASS, 0x0040);
        assert_eq!(DEQUEUE_MIGRATING, 0x0010);
        assert_eq!(ENQUEUE_RESTORE, 0x0002);
        assert_eq!(ENQUEUE_MOVE, 0x0004);
        assert_eq!(ENQUEUE_CLASS, 0x0040);
        assert_eq!(ENQUEUE_HEAD, 0x0001_0000);
        assert_eq!(ENQUEUE_MIGRATED, 0x0004_0000);
    }
}
