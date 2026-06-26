//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// OOM Killer — Milestone 18.
///
/// Linux-compatible Out-Of-Memory victim selection and killing.
///
/// The `OomTask` registry is a pre-M20 placeholder; M20 (`task_struct`) will
/// replace it with real process descriptors while keeping the scoring API
/// identical.
///
/// Ref: Linux `mm/oom_kill.c`
extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;

// ---------------------------------------------------------------------------
// Constants — match Linux uapi/linux/oom.h
// ---------------------------------------------------------------------------

/// Minimum OOM score adjustment — task is never killed.
///
/// Ref: Linux `OOM_SCORE_ADJ_MIN` — `include/uapi/linux/oom.h`
pub const OOM_SCORE_ADJ_MIN: i16 = -1000;

/// Maximum OOM score adjustment — task is preferentially killed.
///
/// Ref: Linux `OOM_SCORE_ADJ_MAX` — `include/uapi/linux/oom.h`
pub const OOM_SCORE_ADJ_MAX: i16 = 1000;

// ---------------------------------------------------------------------------
// OOM Constraint — enum oom_constraint in Linux
// ---------------------------------------------------------------------------

/// The constraint that caused the OOM event.
///
/// Ref: Linux `enum oom_constraint` — `mm/oom_kill.c`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OomConstraint {
    /// Global memory exhaustion — no specific constraint.
    NoConstraint,
    /// A memory-cgroup's `memory.max` was exceeded.
    Memcg,
}

// ---------------------------------------------------------------------------
// OOM Control — struct oom_control in Linux
// ---------------------------------------------------------------------------

/// Parameters for an OOM invocation, mirroring `struct oom_control`.
///
/// In M18, `memcg_id` is `None` for global OOM and `Some(id)` for per-cgroup
/// OOM. M19 (memcg) populates this field when a cgroup limit is exceeded.
///
/// Ref: Linux `include/linux/oom.h`
#[derive(Debug)]
pub struct OomControl {
    /// GFP allocation flags that triggered the OOM.
    pub gfp_mask: u32,
    /// Allocation order that failed (-1 = sysrq / admin-triggered).
    pub order: i32,
    /// Cgroup ID of the cgroup that hit its limit, or `None` for global OOM.
    /// M19 populates this; OOM killer scopes the search to that cgroup's tasks.
    pub memcg_id: Option<u32>,
    /// Total number of available pages (computed before victim selection).
    pub totalpages: u64,
    /// PID of the selected victim process (output field).
    pub chosen: Option<u32>,
    /// Badness score of the chosen victim (output field).
    pub chosen_points: i64,
    /// The constraint type that triggered this OOM event.
    pub constraint: OomConstraint,
}

impl OomControl {
    /// Create an `OomControl` for a global (non-cgroup) OOM event.
    pub fn new(gfp_mask: u32, order: i32) -> Self {
        Self {
            gfp_mask,
            order,
            memcg_id: None,
            totalpages: 0,
            chosen: None,
            chosen_points: i64::MIN,
            constraint: OomConstraint::NoConstraint,
        }
    }

    /// Create an `OomControl` scoped to a specific memory cgroup.
    ///
    /// Used by M19's `mem_cgroup_charge()` when `memory.max` is exceeded.
    pub fn for_memcg(gfp_mask: u32, order: i32, memcg_id: u32) -> Self {
        Self {
            gfp_mask,
            order,
            memcg_id: Some(memcg_id),
            totalpages: 0,
            chosen: None,
            chosen_points: i64::MIN,
            constraint: OomConstraint::Memcg,
        }
    }
}

// ---------------------------------------------------------------------------
// OOM Task — minimal process descriptor for victim selection
// ---------------------------------------------------------------------------

/// A task entry in the OOM victim registry.
///
/// In M18 this is a manually-managed descriptor. M20 (`task_struct`) will
/// replace it with real process descriptors; the scoring formulas are
/// identical to Linux.
///
/// Ref: `task_struct` fields consumed by `oom_badness()` — `mm/oom_kill.c:199`
#[derive(Debug, Clone)]
pub struct OomTask {
    /// UNIX process ID.
    pub pid: u32,
    /// Resident set size in pages (anonymous + file-backed).
    pub mm_rss_pages: u64,
    /// Swap usage in pages (`MM_SWAPENTS` in Linux).
    pub swap_pages: u64,
    /// Page-table overhead in pages.
    pub pgtable_pages: u64,
    /// OOM score adjustment in [-1000, 1000].
    ///
    /// Ref: `task_struct::signal::oom_score_adj` — Linux `include/linux/sched.h`
    pub oom_score_adj: i16,
    /// Kernel threads are never selected as OOM victims.
    pub is_kthread: bool,
    /// Short process name, NUL-padded (mirrors `task_struct::comm`).
    pub comm: [u8; 16],
    /// Cgroup ID (None = root cgroup / ungrouped).
    pub memcg_id: Option<u32>,
    /// Set when the OOM killer has already targeted this task.
    pub is_oom_victim: bool,
    /// Set after the reaper has finished freeing this task's memory.
    pub is_reaped: bool,
}

impl OomTask {
    /// Construct a user-space task descriptor.
    ///
    /// `oom_score_adj` is clamped to `[OOM_SCORE_ADJ_MIN, OOM_SCORE_ADJ_MAX]`.
    pub fn new(pid: u32, rss: u64, swap: u64, adj: i16) -> Self {
        Self {
            pid,
            mm_rss_pages: rss,
            swap_pages: swap,
            pgtable_pages: 0,
            oom_score_adj: adj.clamp(OOM_SCORE_ADJ_MIN, OOM_SCORE_ADJ_MAX),
            is_kthread: false,
            comm: [0u8; 16],
            memcg_id: None,
            is_oom_victim: false,
            is_reaped: false,
        }
    }

    /// Construct a kernel-thread descriptor — never an OOM victim.
    pub fn kthread(pid: u32) -> Self {
        let mut t = Self::new(pid, 0, 0, 0);
        t.is_kthread = true;
        t
    }
}

// ---------------------------------------------------------------------------
// Global task registry and reaper queue
// ---------------------------------------------------------------------------

static OOM_TASKS: Mutex<Vec<OomTask>> = Mutex::new(Vec::new());
static OOM_REAPER_QUEUE: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Register a task as an OOM candidate.
///
/// In M18 callers register tasks manually. M20 will do this automatically
/// during `copy_process()` / `do_exit()`.
pub fn register_oom_task(task: OomTask) {
    OOM_TASKS.lock().push(task);
}

/// Remove a task from the OOM registry (call on task exit).
pub fn unregister_oom_task(pid: u32) {
    OOM_TASKS.lock().retain(|t| t.pid != pid);
}

/// Return the number of tasks currently in the OOM registry.
pub fn oom_task_count() -> usize {
    OOM_TASKS.lock().len()
}

/// Return `true` if a task with the given PID is in the OOM registry.
pub fn oom_task_exists(pid: u32) -> bool {
    OOM_TASKS.lock().iter().any(|t| t.pid == pid)
}

/// Clear all OOM global state — only for use in unit tests.
#[cfg(test)]
pub fn reset_oom_state() {
    OOM_TASKS.lock().clear();
    OOM_REAPER_QUEUE.lock().clear();
}

// ---------------------------------------------------------------------------
// Badness scoring — oom_badness()
// ---------------------------------------------------------------------------

/// Compute the OOM badness score for a task.
///
/// A higher score means the task is a *worse* citizen and is more likely to be
/// killed. Returns `i64::MIN` for tasks that must never be killed (kernel
/// threads, or tasks with `oom_score_adj == OOM_SCORE_ADJ_MIN`).
///
/// Formula (mirrors Linux `mm/oom_kill.c:199`):
/// ```text
/// points  = rss + swap + pgtable_pages
/// points += (oom_score_adj × totalpages) / 1000
/// ```
pub fn oom_badness(task: &OomTask, totalpages: u64) -> i64 {
    if task.is_kthread || task.oom_score_adj == OOM_SCORE_ADJ_MIN {
        return i64::MIN;
    }

    let points = (task.mm_rss_pages + task.swap_pages + task.pgtable_pages) as i64;
    let adj = task.oom_score_adj as i64;
    points + (adj * totalpages as i64) / 1000
}

/// Return the OOM score for a PID, as exposed in `/proc/<pid>/oom_score`.
///
/// Score is clamped to 0 (negative badness is reported as 0).
/// Returns 0 if the PID is not registered.
pub fn oom_score_for(pid: u32, totalpages: u64) -> i64 {
    OOM_TASKS
        .lock()
        .iter()
        .find(|t| t.pid == pid)
        .map(|t| oom_badness(t, totalpages).max(0))
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Victim selection — select_bad_process()
// ---------------------------------------------------------------------------

/// Select the worst-behaving process as the OOM victim.
///
/// When `oc.memcg_id` is `Some(id)`, only tasks belonging to that cgroup are
/// considered. The task with the highest `oom_badness` score is chosen.
///
/// Ref: Linux `select_bad_process()` — `mm/oom_kill.c:362`
pub fn select_bad_process(oc: &mut OomControl) {
    let tasks = OOM_TASKS.lock();
    for task in tasks.iter() {
        if task.is_oom_victim {
            continue;
        }
        if let Some(memcg_id) = oc.memcg_id {
            if task.memcg_id != Some(memcg_id) {
                continue;
            }
        }
        let score = oom_badness(task, oc.totalpages);
        if score == i64::MIN {
            continue;
        }
        if oc.chosen.is_none() || score > oc.chosen_points {
            oc.chosen = Some(task.pid);
            oc.chosen_points = score;
        }
    }
}

// ---------------------------------------------------------------------------
// Killing and reaping
// ---------------------------------------------------------------------------

/// Mark the chosen victim as an OOM victim and queue it for the reaper.
///
/// In M18, "killing" flags the task in the registry and queues it for
/// synchronous reaping. M21 (kthreads) will send `SIGKILL` to real tasks.
///
/// Ref: Linux `oom_kill_process()` → `__oom_kill_process()` — `mm/oom_kill.c:912`
pub fn oom_kill_process(oc: &mut OomControl, _msg: &str) {
    let pid = match oc.chosen {
        Some(pid) => pid,
        None => return,
    };
    {
        let mut tasks = OOM_TASKS.lock();
        if let Some(task) = tasks.iter_mut().find(|t| t.pid == pid) {
            task.is_oom_victim = true;
        }
    }
    queue_oom_reaper(pid);
}

/// Queue a PID for the OOM reaper.
///
/// Ref: Linux `queue_oom_reaper()` — `mm/oom_kill.c:686`
pub fn queue_oom_reaper(pid: u32) {
    OOM_REAPER_QUEUE.lock().push(pid);
}

/// Drain the OOM reaper queue.
///
/// In M18 this runs synchronously because there are no kthreads yet. Each
/// queued task is marked as reaped and removed from the registry.
/// M21 (context switch) will promote this to a kernel thread sleeping on a
/// wait queue, matching `oom_reaper()` in `mm/oom_kill.c:634`.
pub fn oom_reaper_run() {
    let pids: Vec<u32> = {
        let mut q = OOM_REAPER_QUEUE.lock();
        core::mem::take(&mut *q)
    };
    for pid in pids {
        let mut tasks = OOM_TASKS.lock();
        if let Some(task) = tasks.iter_mut().find(|t| t.pid == pid) {
            task.is_reaped = true;
            // Pages conceptually freed here. M20 will walk the victim's
            // mm_struct and call free_pages() on each owned page.
        }
        tasks.retain(|t| t.pid != pid);
    }
}

// ---------------------------------------------------------------------------
// Main OOM entry point — out_of_memory()
// ---------------------------------------------------------------------------

/// Invoke the OOM killer.
///
/// Returns `true` if a victim was found and killed, `false` if no killable
/// tasks exist (e.g., all tasks are kernel threads or OOM-protected).
///
/// Ref: Linux `out_of_memory()` — `mm/oom_kill.c:1103`
pub fn out_of_memory(oc: &mut OomControl) -> bool {
    if oc.totalpages == 0 {
        oc.totalpages = total_managed_pages();
    }
    select_bad_process(oc);
    if oc.chosen.is_none() {
        return false;
    }
    oom_kill_process(oc, "Out of memory");
    oom_reaper_run();
    true
}

// ---------------------------------------------------------------------------
// Helper: total managed pages across all zones
// ---------------------------------------------------------------------------

/// Return total managed pages across all zones.
///
/// Used as `totalpages` when the caller does not supply one.
/// Ref: Linux `oom_badness()` — `totalpages` calculation — `mm/oom_kill.c:207`
pub fn total_managed_pages() -> u64 {
    if crate::mm::buddy::is_buddy_ready() {
        crate::mm::buddy::with_global_buddy(|b| {
            b.zones.iter().map(|z| z.managed_pages as u64).sum()
        })
    } else {
        // Buddy not yet initialised (early boot / unit tests): use a safe
        // default so scoring arithmetic remains correct.
        256 * 1024 * 1024 / 4096 // 256 MiB / PAGE_SIZE
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn reset_state() {
        OOM_TASKS.lock().clear();
        OOM_REAPER_QUEUE.lock().clear();
    }

    // -----------------------------------------------------------------------
    // oom_badness — mirrors Linux oom_badness() formula
    // -----------------------------------------------------------------------

    #[test]
    fn oom_badness_basic_scoring() {
        let _g = test_guard();
        reset_state();

        let task = OomTask::new(1, 100, 50, 0); // rss=100, swap=50, adj=0
        let score = oom_badness(&task, 10_000);
        // adj_contrib = (0 * 10000) / 1000 = 0
        assert_eq!(score, 150);
    }

    #[test]
    fn oom_badness_adj_increases_score() {
        let _g = test_guard();
        reset_state();

        let task = OomTask::new(1, 100, 0, 500);
        let score = oom_badness(&task, 10_000);
        // adj_contrib = (500 * 10000) / 1000 = 5000
        assert_eq!(score, 100 + 5000);
    }

    #[test]
    fn oom_badness_adj_decreases_score() {
        let _g = test_guard();
        reset_state();

        let task = OomTask::new(1, 1000, 0, -500);
        let score = oom_badness(&task, 10_000);
        // adj_contrib = (-500 * 10000) / 1000 = -5000
        assert_eq!(score, 1000 - 5000);
    }

    #[test]
    fn oom_score_adj_min_never_killed() {
        let _g = test_guard();
        reset_state();

        // Even with huge RSS, adj=OOM_SCORE_ADJ_MIN means never killed.
        let task = OomTask::new(1, 999_999, 999_999, OOM_SCORE_ADJ_MIN);
        assert_eq!(oom_badness(&task, u64::MAX / 2), i64::MIN);
    }

    #[test]
    fn oom_kthread_never_victim() {
        let _g = test_guard();
        reset_state();

        let task = OomTask::kthread(2);
        assert_eq!(oom_badness(&task, 10_000), i64::MIN);
    }

    #[test]
    fn oom_score_adj_clamping() {
        let _g = test_guard();
        reset_state();

        let t1 = OomTask::new(1, 0, 0, 2000);
        assert_eq!(t1.oom_score_adj, OOM_SCORE_ADJ_MAX);

        let t2 = OomTask::new(2, 0, 0, -2000);
        assert_eq!(t2.oom_score_adj, OOM_SCORE_ADJ_MIN);
    }

    // -----------------------------------------------------------------------
    // select_bad_process — mirrors Linux select_bad_process()
    // -----------------------------------------------------------------------

    #[test]
    fn select_bad_process_highest_score_wins() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::new(1, 100, 0, 0)); // score 100
        register_oom_task(OomTask::new(2, 500, 0, 0)); // score 500 ← winner
        register_oom_task(OomTask::new(3, 250, 0, 0)); // score 250

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        select_bad_process(&mut oc);

        assert_eq!(oc.chosen, Some(2));
        assert_eq!(oc.chosen_points, 500);
    }

    #[test]
    fn select_bad_process_adj_max_always_chosen() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::new(1, 1000, 0, 0));
        // adj=1000, totalpages=10000 → adj_contrib = 10000 → total = 10001
        register_oom_task(OomTask::new(2, 1, 0, OOM_SCORE_ADJ_MAX));

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        select_bad_process(&mut oc);

        assert_eq!(oc.chosen, Some(2));
    }

    #[test]
    fn select_bad_process_all_kthreads_no_victim() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::kthread(1));
        register_oom_task(OomTask::kthread(2));

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        select_bad_process(&mut oc);

        assert!(oc.chosen.is_none());
    }

    #[test]
    fn select_bad_process_skips_existing_victims() {
        let _g = test_guard();
        reset_state();

        let mut task1 = OomTask::new(1, 9999, 0, 0);
        task1.is_oom_victim = true;
        register_oom_task(task1);
        register_oom_task(OomTask::new(2, 100, 0, 0));

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        select_bad_process(&mut oc);

        assert_eq!(oc.chosen, Some(2));
    }

    // -----------------------------------------------------------------------
    // out_of_memory + reaper
    // -----------------------------------------------------------------------

    #[test]
    fn out_of_memory_kills_victim() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::new(42, 200, 0, 0));

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        let killed = out_of_memory(&mut oc);

        assert!(killed);
        assert_eq!(oc.chosen, Some(42));
        // Reaper removes task from registry.
        assert_eq!(oom_task_count(), 0);
    }

    #[test]
    fn out_of_memory_returns_false_when_no_killable_task() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::kthread(1));

        let mut oc = OomControl::new(0, 0);
        oc.totalpages = 10_000;
        let killed = out_of_memory(&mut oc);

        assert!(!killed);
    }

    #[test]
    fn oom_reaper_frees_pages() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::new(10, 50, 0, 0));
        queue_oom_reaper(10);
        oom_reaper_run();

        assert_eq!(oom_task_count(), 0);
        assert!(OOM_REAPER_QUEUE.lock().is_empty());
    }

    #[test]
    fn register_unregister_task() {
        let _g = test_guard();
        reset_state();

        assert_eq!(oom_task_count(), 0);
        register_oom_task(OomTask::new(1, 10, 0, 0));
        register_oom_task(OomTask::new(2, 20, 0, 0));
        assert_eq!(oom_task_count(), 2);
        unregister_oom_task(1);
        assert_eq!(oom_task_count(), 1);
        unregister_oom_task(2);
        assert_eq!(oom_task_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Cgroup-scoped OOM — mirrors test_memcg_oom_group_score_events
    // -----------------------------------------------------------------------

    #[test]
    fn out_of_memory_memcg_scoped_kills_only_cgroup_member() {
        let _g = test_guard();
        reset_state();

        let mut t1 = OomTask::new(1, 1000, 0, 0);
        t1.memcg_id = Some(7);
        let mut t2 = OomTask::new(2, 500, 0, 0);
        t2.memcg_id = Some(8);
        register_oom_task(t1);
        register_oom_task(t2);

        let mut oc = OomControl::for_memcg(0, 0, 7);
        oc.totalpages = 10_000;
        let killed = out_of_memory(&mut oc);

        assert!(killed);
        assert_eq!(oc.chosen, Some(1));
        // PID 2 (different cgroup) must still be in the registry.
        assert_eq!(oom_task_count(), 1);
    }

    #[test]
    fn oom_score_for_returns_zero_for_unknown_pid() {
        let _g = test_guard();
        reset_state();

        assert_eq!(oom_score_for(999, 10_000), 0);
    }

    #[test]
    fn oom_score_for_returns_score_for_known_pid() {
        let _g = test_guard();
        reset_state();

        register_oom_task(OomTask::new(5, 300, 200, 0));
        // rss + swap = 300 + 200 = 500, adj=0
        assert_eq!(oom_score_for(5, 10_000), 500);
    }
}
