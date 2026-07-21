//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched
//! linux-source: vendor/linux/kernel/sched/sched.h
//! test-origin: linux:vendor/linux/kernel/sched
//! Per-CPU runqueue — `struct rq` (Linux `kernel/sched/sched.h::struct rq`).
//!
//! Per-CPU `struct rq` with per-class sub-runqueues (cfs/rt/dl) behind a raw
//! spinlock. Remaining work vs Linux `sched.h`/`core.c` for `complete`: full
//! `rq_clock` IRQ-time accounting (currently a placeholder until M37) and the
//! per-class enqueue/dequeue/pick wiring to a real load balancer.
//!
//! Lupos M29: each CPU owns one `Rq` containing per-class sub-runqueues
//! (`cfs`, `rt`, `dl`).  ABI parity promotes the storage to an IRQ-safe
//! raw-spinlocked container so timer / IPI paths can manipulate remote
//! runqueues without relying on the cooperative global queue.

extern crate alloc;

use core::mem::offset_of;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::kernel::locking::raw_spinlock::RawSpinLocked;
use crate::kernel::task::{M29SchedFields, TaskStruct};
use crate::lib::rbtree::{
    LinuxRbNode, LinuxRbRoot, linux_rb_erase, linux_rb_insert_color, linux_rb_next,
};
use alloc::vec::Vec;

use super::entity::SchedEntity;
use super::sched_clock_ns;

// ── CFS sub-runqueue ─────────────────────────────────────────────────────────

/// Sorted task pointer map keyed by `(runtime key, task pointer)`.
///
/// Linux uses rb-trees for CFS and deadline ordering.  Lupos keeps the same
/// total ordering with a compact sorted vector so the hard-tick scheduler path
/// does not depend on `alloc::collections::BTreeMap` iterator state.
pub struct TaskOrderMap {
    entries: Vec<((u64, usize), *mut TaskStruct)>,
}

unsafe impl Send for TaskOrderMap {}

impl TaskOrderMap {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn position(&self, key: &(u64, usize)) -> Result<usize, usize> {
        self.entries
            .binary_search_by(|(entry_key, _)| entry_key.cmp(key))
    }

    pub fn insert(&mut self, key: (u64, usize), value: *mut TaskStruct) {
        match self.position(&key) {
            Ok(idx) => self.entries[idx].1 = value,
            Err(idx) => self.entries.insert(idx, (key, value)),
        }
    }

    pub fn remove(&mut self, key: &(u64, usize)) -> Option<*mut TaskStruct> {
        self.position(key)
            .ok()
            .map(|idx| self.entries.remove(idx).1)
    }

    pub fn contains_key(&self, key: &(u64, usize)) -> bool {
        self.position(key).is_ok()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&(u64, usize), &*mut TaskStruct)> + '_ {
        self.entries.iter().map(|(key, value)| (key, value))
    }
}

/// Intrusive cached rb-tree for CFS entities.
///
/// Linux keeps `sched_entity::run_node` inside the task and therefore never
/// allocates while holding `rq->lock`. Reuse the translated Linux rbtree core
/// and retain a cached leftmost node for O(1) selection.
pub struct CfsTimeline {
    root: LinuxRbRoot,
    leftmost: *mut LinuxRbNode,
    len: usize,
}

unsafe impl Send for CfsTimeline {}

const TASK_RUN_NODE_OFFSET: usize = offset_of!(TaskStruct, m29)
    + offset_of!(M29SchedFields, se)
    + offset_of!(SchedEntity, run_node);

#[inline]
unsafe fn task_from_run_node(node: *mut LinuxRbNode) -> *mut TaskStruct {
    unsafe {
        (node as *mut u8)
            .sub(TASK_RUN_NODE_OFFSET)
            .cast::<TaskStruct>()
    }
}

#[inline]
unsafe fn run_node(task: *mut TaskStruct) -> *mut LinuxRbNode {
    unsafe { ptr::addr_of_mut!((*task).m29.se.run_node) }
}

#[inline]
unsafe fn timeline_key(task: *mut TaskStruct) -> (u64, usize) {
    unsafe { ((*task).m29.se.vruntime, task as usize) }
}

impl CfsTimeline {
    pub const fn new() -> Self {
        Self {
            root: LinuxRbRoot {
                rb_node: ptr::null_mut(),
            },
            leftmost: ptr::null_mut(),
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains_key(&self, key: &(u64, usize)) -> bool {
        let mut node = self.root.rb_node;
        while !node.is_null() {
            let task = unsafe { task_from_run_node(node) };
            let existing = unsafe { timeline_key(task) };
            if *key < existing {
                node = unsafe { (*node).rb_left };
            } else if *key > existing {
                node = unsafe { (*node).rb_right };
            } else {
                return true;
            }
        }
        false
    }

    pub fn insert(&mut self, task: *mut TaskStruct, vruntime: u64) {
        debug_assert!(!task.is_null());
        let key = (vruntime, task as usize);
        let node = unsafe { run_node(task) };
        let mut parent = ptr::null_mut();
        let mut link = ptr::addr_of_mut!(self.root.rb_node);

        while !unsafe { *link }.is_null() {
            parent = unsafe { *link };
            let existing_task = unsafe { task_from_run_node(parent) };
            let existing_key = unsafe { timeline_key(existing_task) };
            if key < existing_key {
                link = unsafe { ptr::addr_of_mut!((*parent).rb_left) };
            } else if key > existing_key {
                link = unsafe { ptr::addr_of_mut!((*parent).rb_right) };
            } else {
                return;
            }
        }

        unsafe {
            (*node).__rb_parent_color = parent as usize;
            (*node).rb_left = ptr::null_mut();
            (*node).rb_right = ptr::null_mut();
            *link = node;
            linux_rb_insert_color(node, ptr::addr_of_mut!(self.root));
        }
        if self.leftmost.is_null()
            || key < unsafe { timeline_key(task_from_run_node(self.leftmost)) }
        {
            self.leftmost = node;
        }
        self.len += 1;
    }

    pub fn remove(&mut self, task: *mut TaskStruct, vruntime: u64) -> bool {
        if task.is_null() {
            return false;
        }
        let key = (vruntime, task as usize);
        let mut node = self.root.rb_node;
        while !node.is_null() {
            let existing_task = unsafe { task_from_run_node(node) };
            let existing_key = unsafe { timeline_key(existing_task) };
            if key < existing_key {
                node = unsafe { (*node).rb_left };
            } else if key > existing_key {
                node = unsafe { (*node).rb_right };
            } else {
                break;
            }
        }
        if node.is_null() {
            return false;
        }

        if self.leftmost == node {
            self.leftmost = unsafe { linux_rb_next(node) };
        }
        unsafe {
            linux_rb_erase(node, ptr::addr_of_mut!(self.root));
            // Linux RB_CLEAR_NODE(): mark the intrusive node as unlinked.
            (*node).__rb_parent_color = node as usize;
            (*node).rb_left = ptr::null_mut();
            (*node).rb_right = ptr::null_mut();
        }
        self.len -= 1;
        true
    }

    pub fn first(&self) -> *mut TaskStruct {
        if self.leftmost.is_null() {
            ptr::null_mut()
        } else {
            unsafe { task_from_run_node(self.leftmost) }
        }
    }

    pub fn last_vruntime(&self) -> Option<u64> {
        let mut node = self.root.rb_node;
        if node.is_null() {
            return None;
        }
        while unsafe { !(*node).rb_right.is_null() } {
            node = unsafe { (*node).rb_right };
        }
        let task = unsafe { task_from_run_node(node) };
        Some(unsafe { (*task).m29.se.vruntime })
    }

    pub fn iter(&self) -> CfsTimelineIter {
        CfsTimelineIter {
            node: self.leftmost,
        }
    }
}

pub struct CfsTimelineIter {
    node: *mut LinuxRbNode,
}

impl Iterator for CfsTimelineIter {
    type Item = *mut TaskStruct;

    fn next(&mut self) -> Option<Self::Item> {
        if self.node.is_null() {
            return None;
        }
        let node = self.node;
        self.node = unsafe { linux_rb_next(node) };
        Some(unsafe { task_from_run_node(node) })
    }
}

/// CFS runqueue (`struct cfs_rq` in Linux).
///
/// `tasks_timeline` is the rb-tree equivalent keyed by `vruntime`.
/// `min_vruntime` is the current floor — new tasks join the tree at
/// `min_vruntime` so they aren't given a free CPU shot.
pub struct CfsRq {
    /// Number of tasks currently enqueued (mirrors Linux `cfs_rq.nr_running`).
    pub nr_running: u32,
    /// Global vruntime floor.  Monotonically non-decreasing.
    pub min_vruntime: u64,
    /// Sum of weights across enqueued entities — used by `__sched_period`.
    pub load_weight: u64,
    /// Ordered map keyed by (vruntime, task pointer cast to usize) → task.
    /// The compound key disambiguates entities that share a vruntime so the
    /// map remains a strict total order without dropping entries.
    pub tasks_timeline: CfsTimeline,
    /// Currently running entity on this CPU (NULL if idle).
    pub current: *mut TaskStruct,
    /// Last update timestamp (ns since boot), updated by `update_curr`.
    pub last_update_ns: u64,
}

unsafe impl Send for CfsRq {}

impl CfsRq {
    pub const fn new() -> Self {
        Self {
            nr_running: 0,
            min_vruntime: 0,
            load_weight: 0,
            tasks_timeline: CfsTimeline::new(),
            current: core::ptr::null_mut(),
            last_update_ns: 0,
        }
    }

    /// Re-key an entity in the timeline (used after `vruntime` changes).
    pub fn reinsert(&mut self, p: *mut TaskStruct, old_key: u64, new_key: u64) {
        if old_key == new_key {
            return;
        }
        self.tasks_timeline.remove(p, old_key);
        self.tasks_timeline.insert(p, new_key);
    }

    /// Return the leftmost (smallest-vruntime) entity, or NULL when empty.
    pub fn leftmost(&self) -> *mut TaskStruct {
        self.tasks_timeline.first()
    }

    /// Insert `p` at vruntime `key`.
    pub fn insert(&mut self, p: *mut TaskStruct, key: u64) {
        self.tasks_timeline.insert(p, key);
    }

    /// Remove `p` from the timeline at vruntime `key`.
    pub fn remove(&mut self, p: *mut TaskStruct, key: u64) {
        self.tasks_timeline.remove(p, key);
    }

    /// Update `min_vruntime` to the smaller of (current value, leftmost).
    pub fn update_min_vruntime(&mut self) {
        let mut candidate = if self.current.is_null() {
            None
        } else {
            Some(unsafe { (*self.current).m29.se.vruntime })
        };
        let leftmost = self.tasks_timeline.first();
        if !leftmost.is_null() {
            let left_vruntime = unsafe { (*leftmost).m29.se.vruntime };
            candidate = Some(candidate.map_or(left_vruntime, |value| value.min(left_vruntime)));
        }
        if let Some(vruntime) = candidate
            && vruntime > self.min_vruntime
        {
            self.min_vruntime = vruntime;
        }
    }

    pub fn entity_load_weight(&self, _se: &SchedEntity) -> u64 {
        // Placeholder — the load tracking is per-entity (see fair.rs::update_curr).
        self.load_weight
    }
}

// ── RT sub-runqueue ──────────────────────────────────────────────────────────

/// RT runqueue (`struct rt_rq`) — priority-indexed FIFO.
///
/// Linux uses an array of `list_head[MAX_RT_PRIO]`; we mirror that with an
/// array of `alloc::collections::VecDeque<*mut TaskStruct>` indexed by RT
/// priority (0..99).
pub struct RtRq {
    pub nr_running: u32,
    pub queues: [alloc::collections::VecDeque<*mut TaskStruct>; super::prio::MAX_RT_PRIO as usize],
    /// Bitmap of priorities that currently have at least one task — fast-pick.
    pub active_bitmap: [u64; 2], // 100 bits → 2 × u64
    /// Currently running RT task on this CPU (NULL if none).
    pub current: *mut TaskStruct,
}

unsafe impl Send for RtRq {}

impl RtRq {
    pub fn new() -> Self {
        const EMPTY: alloc::collections::VecDeque<*mut TaskStruct> =
            alloc::collections::VecDeque::new();
        Self {
            nr_running: 0,
            queues: [EMPTY; super::prio::MAX_RT_PRIO as usize],
            active_bitmap: [0; 2],
            current: core::ptr::null_mut(),
        }
    }

    pub fn highest_prio(&self) -> Option<i32> {
        // Lowest priority *number* that is set (bit position) wins (0 = MAX).
        for (idx, w) in self.active_bitmap.iter().enumerate() {
            if *w != 0 {
                let bit = w.trailing_zeros() as i32;
                return Some(idx as i32 * 64 + bit);
            }
        }
        None
    }

    pub fn enqueue(&mut self, p: *mut TaskStruct, prio: i32, head: bool) {
        let prio = prio.clamp(0, super::prio::MAX_RT_PRIO - 1) as usize;
        if head {
            self.queues[prio].push_front(p);
        } else {
            self.queues[prio].push_back(p);
        }
        self.active_bitmap[prio / 64] |= 1u64 << (prio % 64);
        self.nr_running += 1;
    }

    pub fn dequeue(&mut self, p: *mut TaskStruct, prio: i32) -> bool {
        let prio = prio.clamp(0, super::prio::MAX_RT_PRIO - 1) as usize;
        let q = &mut self.queues[prio];
        if let Some(idx) = q.iter().position(|&x| x == p) {
            q.remove(idx);
            if q.is_empty() {
                self.active_bitmap[prio / 64] &= !(1u64 << (prio % 64));
            }
            self.nr_running = self.nr_running.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn pick_first(&self) -> *mut TaskStruct {
        if let Some(p) = self.highest_prio() {
            self.queues[p as usize]
                .front()
                .copied()
                .unwrap_or(core::ptr::null_mut())
        } else {
            core::ptr::null_mut()
        }
    }

    /// Round-robin: rotate the head of the bucket to the tail.
    pub fn requeue_tail(&mut self, prio: i32) {
        let prio = prio.clamp(0, super::prio::MAX_RT_PRIO - 1) as usize;
        if let Some(p) = self.queues[prio].pop_front() {
            self.queues[prio].push_back(p);
        }
    }
}

// ── DL sub-runqueue ──────────────────────────────────────────────────────────

/// Deadline runqueue — EDF order keyed on absolute deadline.
pub struct DlRq {
    pub nr_running: u32,
    pub root: TaskOrderMap,
    /// Total used bandwidth (sum of `dl_runtime / dl_period`) on this CPU,
    /// scaled by `BW_SHIFT = 20` (Linux `BW_SHIFT`).
    pub running_bw: u64,
    /// Configured admission cap (Linux `dl_runtime` default = 95% of period).
    pub bw_cap: u64,
    /// Currently running DL task.
    pub current: *mut TaskStruct,
}

unsafe impl Send for DlRq {}

/// Linux `BW_SHIFT` — fixed-point shift used for admission control.
pub const BW_SHIFT: u32 = 20;
/// 95% bandwidth cap, fixed-point (matches Linux default of `dl_runtime`).
pub const DEFAULT_DL_BW_CAP: u64 = (95u64 * (1u64 << BW_SHIFT)) / 100;

impl DlRq {
    pub const fn new() -> Self {
        Self {
            nr_running: 0,
            root: TaskOrderMap::new(),
            running_bw: 0,
            bw_cap: DEFAULT_DL_BW_CAP,
            current: core::ptr::null_mut(),
        }
    }

    pub fn earliest(&self) -> *mut TaskStruct {
        self.root
            .iter()
            .next()
            .map(|(_, &p)| p)
            .unwrap_or(core::ptr::null_mut())
    }

    pub fn insert(&mut self, p: *mut TaskStruct, deadline: u64) {
        self.root.insert((deadline, p as usize), p);
    }

    pub fn remove(&mut self, p: *mut TaskStruct, deadline: u64) {
        self.root.remove(&(deadline, p as usize));
    }
}

// ── Top-level runqueue ───────────────────────────────────────────────────────

/// Per-CPU runqueue.
pub struct Rq {
    pub cpu: u32,
    /// Embedded sub-runqueues.
    pub cfs: CfsRq,
    pub rt: RtRq,
    pub dl: DlRq,
    /// Total nr_running across all classes.
    pub nr_running: u32,
    /// Currently running task on this CPU.
    pub current: *mut TaskStruct,
    /// The idle task pointer (per-CPU swapper).
    pub idle: *mut TaskStruct,
    /// Monotonic ns clock — derived from `apic_timer::TIMER_TICKS`.
    pub clock: u64,
    /// Same as `clock` minus IRQ time (placeholder until M37).
    pub clock_task: u64,
    /// `tick_stopped` bit for NOHZ idle (M31).
    pub tick_stopped: bool,
    /// Periodic-balance counter (M31).
    pub last_balance_tick: u64,
}

unsafe impl Send for Rq {}

impl Rq {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu,
            cfs: CfsRq::new(),
            rt: RtRq::new(),
            dl: DlRq::new(),
            nr_running: 0,
            current: core::ptr::null_mut(),
            idle: core::ptr::null_mut(),
            clock: 0,
            clock_task: 0,
            tick_stopped: false,
            last_balance_tick: 0,
        }
    }

    /// Refresh `clock`/`clock_task` from the global scheduler clock.
    pub fn update_rq_clock(&mut self) {
        let now = sched_clock_ns();
        self.clock = now;
        self.clock_task = now;
    }
}

// ── Per-CPU array ────────────────────────────────────────────────────────────

/// Maximum CPUs (mirrors `sched::MAX_CPUS`).
pub const MAX_RQ_CPUS: usize = super::MAX_CPUS;

/// Per-CPU runqueue array.  Each entry is lazily initialised inside `init_rqs`.
static RQS: [RawSpinLocked<Option<Rq>>; MAX_RQ_CPUS] =
    [const { RawSpinLocked::new(None) }; MAX_RQ_CPUS];

/// Lockless load snapshots used by wake placement and domain scans.
///
/// Linux's placement path consumes scheduler load/capacity snapshots before
/// taking a target rq lock; it does not lock every active rq for every wake.
static NR_RUNNING_SNAPSHOT: [AtomicU32; MAX_RQ_CPUS] = [const { AtomicU32::new(0) }; MAX_RQ_CPUS];

/// Initialise runqueues for all CPUs the system might use.
///
/// Called once from `sched_init()`.
pub fn init_rqs() {
    for (cpu, slot) in RQS.iter().enumerate() {
        let mut g = slot.lock();
        if g.is_none() {
            *g = Some(Rq::new(cpu as u32));
        }
        NR_RUNNING_SNAPSHOT[cpu].store(g.as_ref().map_or(0, |rq| rq.nr_running), Ordering::Release);
    }
}

/// Run a closure with mutable access to the per-CPU runqueue for `cpu`.
///
/// Returns the closure's result, or `None` if the CPU index is out-of-range
/// or the runqueue hasn't been initialised yet.
/// Save RFLAGS and disable interrupts, returning the saved flags.
///
/// Mirrors Linux `local_irq_save()`
/// (vendor/linux/arch/x86/include/asm/irqflags.h).  The runqueue lock must
/// never be interrupted by the LAPIC tick: `apic_timer::on_tick` →
/// `scheduler_tick` takes the same lock from the ISR, so a tick landing
/// inside a task-context critical section would spin on a lock owned by
/// the interrupted frame and freeze the CPU (the systemd multi-user boot
/// froze exactly this way under WHPX/KVM/TCG alike).
#[cfg(not(test))]
#[inline]
fn local_irq_save() -> u64 {
    let flags: u64;
    unsafe {
        core::arch::asm!("pushfq", "pop {}", "cli", out(reg) flags, options(nomem));
    }
    flags
}

/// Restore the interrupt flag captured by [`local_irq_save`].
/// Mirrors Linux `local_irq_restore()`.
#[cfg(not(test))]
#[inline]
fn local_irq_restore(flags: u64) {
    const X86_EFLAGS_IF: u64 = 1 << 9;
    if flags & X86_EFLAGS_IF != 0 {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }
}

#[cfg(test)]
fn local_irq_save() -> u64 {
    0
}

#[cfg(test)]
fn local_irq_restore(_flags: u64) {}

pub fn with_rq<R>(cpu: u32, f: impl FnOnce(&mut Rq) -> R) -> Option<R> {
    let cpu = cpu as usize;
    if cpu >= MAX_RQ_CPUS {
        return None;
    }
    // Linux rq_lock_irqsave(): runqueue critical sections run with
    // interrupts disabled so the tick ISR can never deadlock against a
    // holder on the same CPU.
    let flags = local_irq_save();
    let result = {
        let mut g = RQS[cpu].lock();
        g.as_mut().map(|rq| {
            let result = f(rq);
            NR_RUNNING_SNAPSHOT[cpu].store(rq.nr_running, Ordering::Release);
            result
        })
    };
    local_irq_restore(flags);
    result
}

/// Run a closure while holding two runqueue locks in logical-CPU order.
///
/// Linux's `double_rq_lock()` always takes the lower-addressed runqueue first.
/// Lupos runqueues have a stable CPU-index order, so ordering by CPU provides
/// the same deadlock exclusion. Local IRQs remain disabled until both locks
/// have been released.
pub fn with_double_rq<R>(
    first_cpu: u32,
    second_cpu: u32,
    f: impl FnOnce(&mut Rq, &mut Rq) -> R,
) -> Option<R> {
    let first_cpu = first_cpu as usize;
    let second_cpu = second_cpu as usize;
    if first_cpu >= MAX_RQ_CPUS || second_cpu >= MAX_RQ_CPUS || first_cpu == second_cpu {
        return None;
    }

    let flags = local_irq_save();
    let (low_cpu, high_cpu, first_is_low) = if first_cpu < second_cpu {
        (first_cpu, second_cpu, true)
    } else {
        (second_cpu, first_cpu, false)
    };
    let mut low_guard = RQS[low_cpu].lock();
    let mut high_guard = RQS[high_cpu].lock();
    let result = match (low_guard.as_mut(), high_guard.as_mut()) {
        (Some(low_rq), Some(high_rq)) => {
            let value = if first_is_low {
                f(low_rq, high_rq)
            } else {
                f(high_rq, low_rq)
            };
            NR_RUNNING_SNAPSHOT[low_cpu].store(low_rq.nr_running, Ordering::Release);
            NR_RUNNING_SNAPSHOT[high_cpu].store(high_rq.nr_running, Ordering::Release);
            Some(value)
        }
        _ => None,
    };
    drop(high_guard);
    drop(low_guard);
    local_irq_restore(flags);
    result
}

/// Run a closure with mutable access to the runqueue of the current CPU.
pub fn with_this_rq<R>(f: impl FnOnce(&mut Rq) -> R) -> Option<R> {
    with_rq(super::current_cpu(), f)
}

/// Return the current `nr_running` snapshot for `cpu`.
pub fn rq_nr_running(cpu: u32) -> Option<u32> {
    NR_RUNNING_SNAPSHOT
        .get(cpu as usize)
        .map(|snapshot| snapshot.load(Ordering::Acquire))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    #[test]
    fn runqueue_lock_is_held_with_interrupts_disabled() {
        // The LAPIC tick ISR (apic_timer::on_tick -> scheduler_tick) takes
        // the runqueue lock.  If a tick lands while task context holds it
        // with IRQs enabled, the ISR spins on a lock owned by the frame
        // underneath it and the whole CPU freezes — the systemd multi-user
        // boot froze exactly this way.  Linux therefore only takes rq locks
        // under local_irq_save (vendor/linux/kernel/sched/core.c
        // rq_lock_irqsave); with_rq is the single Lupos lock site and must
        // do the same.
        let source = include_str!("rq.rs");
        let body = source
            .split("pub fn with_rq")
            .nth(1)
            .expect("with_rq body")
            .split("pub fn with_this_rq")
            .next()
            .expect("with_rq body end");
        assert!(
            body.contains("local_irq_save") && body.contains("local_irq_restore"),
            "with_rq must hold RQS locks with interrupts disabled"
        );
        let save = source
            .split("fn local_irq_save")
            .nth(1)
            .expect("local_irq_save body");
        assert!(
            save.contains("pushfq") && save.contains("cli"),
            "local_irq_save must capture RFLAGS and disable interrupts"
        );
    }

    #[test]
    fn with_rq_still_runs_closures_and_rejects_bad_cpu() {
        assert!(with_rq(MAX_RQ_CPUS as u32, |_| ()).is_none());
        // Closure result must round-trip through the irq-save wrapper.
        init_rqs();
        assert_eq!(with_rq(0, |_| 42), Some(42));
    }

    #[test]
    fn cfs_rq_starts_empty() {
        let rq = CfsRq::new();
        assert_eq!(rq.nr_running, 0);
        assert_eq!(rq.min_vruntime, 0);
        assert!(rq.leftmost().is_null());
    }

    #[test]
    fn cfs_rq_leftmost_returns_smallest_vruntime() {
        let mut rq = CfsRq::new();
        let mut a = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut b = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let pa = &mut *a as *mut TaskStruct;
        let pb = &mut *b as *mut TaskStruct;
        a.m29.se.vruntime = 100;
        b.m29.se.vruntime = 50;
        rq.insert(pa, 100);
        rq.insert(pb, 50);
        assert_eq!(rq.leftmost(), pb);
        rq.remove(pb, 50);
        assert_eq!(rq.leftmost(), pa);
    }

    /// Exercise Linux's intrusive rb-tree insertion and erase paths with
    /// enough entities to force rotations at several depths.
    ///
    /// test-origin: linux:vendor/linux/lib/rbtree_test.c
    #[test]
    fn cfs_timeline_remains_sorted_across_rotations_and_erases() {
        const TASKS: usize = 127;
        let mut timeline = CfsTimeline::new();
        let mut tasks: Vec<Box<TaskStruct>> = (0..TASKS)
            .map(|_| Box::new(unsafe { core::mem::zeroed::<TaskStruct>() }))
            .collect();

        for (index, task) in tasks.iter_mut().enumerate() {
            let vruntime = ((index * 37) % TASKS) as u64;
            task.m29.se.vruntime = vruntime;
            timeline.insert(&mut **task, vruntime);
        }

        let ordered: Vec<u64> = timeline
            .iter()
            .map(|task| unsafe { (*task).m29.se.vruntime })
            .collect();
        assert_eq!(ordered, (0..TASKS as u64).collect::<Vec<_>>());

        for task in tasks.iter_mut().step_by(2) {
            let ptr = &mut **task as *mut TaskStruct;
            assert!(timeline.remove(ptr, task.m29.se.vruntime));
        }
        let remaining: Vec<u64> = timeline
            .iter()
            .map(|task| unsafe { (*task).m29.se.vruntime })
            .collect();
        assert_eq!(
            remaining,
            (0..TASKS as u64)
                .filter(|vruntime| {
                    let insertion_index = (*vruntime as usize * 103) % TASKS;
                    insertion_index % 2 == 1
                })
                .collect::<Vec<_>>()
        );

        for task in tasks.iter_mut().skip(1).step_by(2) {
            let ptr = &mut **task as *mut TaskStruct;
            assert!(timeline.remove(ptr, task.m29.se.vruntime));
        }
        assert!(timeline.is_empty());
        assert!(timeline.first().is_null());
    }

    #[test]
    fn rt_rq_picks_lowest_prio_number() {
        let mut rq = RtRq::new();
        let mut a = 0u64;
        let mut b = 0u64;
        let pa = &mut a as *mut u64 as *mut TaskStruct;
        let pb = &mut b as *mut u64 as *mut TaskStruct;
        // Lower priority number = higher RT priority.
        rq.enqueue(pa, 10, false);
        rq.enqueue(pb, 50, false);
        assert_eq!(rq.highest_prio(), Some(10));
        assert_eq!(rq.pick_first(), pa);
    }

    #[test]
    fn rt_rq_round_robin_rotation() {
        let mut rq = RtRq::new();
        let mut a = 0u64;
        let mut b = 0u64;
        let pa = &mut a as *mut u64 as *mut TaskStruct;
        let pb = &mut b as *mut u64 as *mut TaskStruct;
        rq.enqueue(pa, 50, false);
        rq.enqueue(pb, 50, false);
        assert_eq!(rq.pick_first(), pa);
        rq.requeue_tail(50);
        assert_eq!(rq.pick_first(), pb);
    }

    #[test]
    fn dl_rq_picks_earliest_deadline() {
        let mut rq = DlRq::new();
        let mut a = 0u64;
        let mut b = 0u64;
        let pa = &mut a as *mut u64 as *mut TaskStruct;
        let pb = &mut b as *mut u64 as *mut TaskStruct;
        rq.insert(pa, 1_000);
        rq.insert(pb, 500);
        assert_eq!(rq.earliest(), pb);
    }

    #[test]
    fn rq_default_bw_cap_is_95_percent() {
        let rq = DlRq::new();
        // 95 / 100 of (1 << 20) = 996147
        assert_eq!(rq.bw_cap, (95 * (1 << BW_SHIFT)) / 100);
    }
}
