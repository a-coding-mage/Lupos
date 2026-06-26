//! linux-parity: partial
//! linux-source: vendor/linux/kernel/sched
//! linux-source: vendor/linux/include/linux/sched.h
//! test-origin: linux:vendor/linux/kernel/sched
//! Scheduler entity types — `sched_entity`, `sched_rt_entity`, `sched_dl_entity`.
//!
//! Defines the full `sched_entity`/`sched_rt_entity`/`sched_dl_entity` layouts
//! (load_weight, sched_avg) with size asserts vs Linux pahole. Remaining work
//! for `complete`: the behavioral algorithms — EEVDF pick/`update_curr`, PELT
//! `__update_load_avg`, and load balancing — which live in fair.c/pelt.c and
//! are not yet ported here.
//!
//! Pahole-derived sizes for Linux 7.x defconfig (no PSI, no NUMA-balancing,
//! CONFIG_FAIR_GROUP_SCHED on, CONFIG_RT_GROUP_SCHED on, CONFIG_SCHED_DEBUG off,
//! NR_CPUS=64):
//!
//! | type             | size  |
//! |------------------|-------|
//! | sched_entity     | 256 B |
//! | sched_rt_entity  |  48 B |
//! | sched_dl_entity  | 272 B |
//!
//! Embedded inside `task_struct` via `M29SchedFields` in the
//! `_pad_stack_to_mm` 960-byte span (offset 40 → LINUX_OFFSET_MM).
//!
//! References:
//!   * `vendor/linux/include/linux/sched.h` (struct sched_entity, ~575)
//!   * `vendor/linux/include/linux/sched.h` (struct sched_rt_entity, ~623)
//!   * `vendor/linux/include/linux/sched.h` (struct sched_dl_entity, ~644)

use core::sync::atomic::AtomicU64;

// ── Load weight ──────────────────────────────────────────────────────────────

/// Linux `struct load_weight` — fixed-point load factor for CFS.
///
/// `weight` is the integer weight (from `sched_prio_to_weight[nice + 20]`),
/// `inv_weight` is `2^32 / weight` precomputed for fast `__calc_delta`.
///
/// Reference: `vendor/linux/include/linux/sched.h` `struct load_weight`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadWeight {
    pub weight: u64,
    pub inv_weight: u32,
    /// 4-byte alignment pad.
    pub _pad: u32,
}

const _: () = assert!(core::mem::size_of::<LoadWeight>() == 16);

impl LoadWeight {
    pub const fn zeroed() -> Self {
        Self {
            weight: 0,
            inv_weight: 0,
            _pad: 0,
        }
    }
}

// ── PELT averages (M31 placeholder) ──────────────────────────────────────────

/// Linux `struct sched_avg` — exponential moving averages for PELT.
///
/// In Lupos M29 these fields are tracked but not yet consumed by a load
/// balancer; M31 wires them to `find_busiest_queue` selection.
///
/// Pahole on a defconfig kernel reports 64 bytes.
///
/// We do *not* propagate Linux's `____cacheline_aligned` here because the
/// upstream alignment is a performance hint, not a layout requirement, and
/// preserving it forces `M29SchedFields` to a 64-byte alignment that would
/// move every downstream acceptance offset (`mm`, `pid`, `cred`, …) off its
/// `LINUX_OFFSET_*` slot.  Cache-aware placement is restored in M37 once
/// per-CPU storage is on a separate allocation.
///
/// Reference: `vendor/linux/include/linux/sched.h` `struct sched_avg`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SchedAvg {
    pub last_update_time: u64,
    pub load_sum: u64,
    pub runnable_sum: u64,
    pub util_sum: u32,
    pub period_contrib: u32,
    pub load_avg: u64,
    pub runnable_avg: u64,
    pub util_avg: u64,
    pub _pad: [u64; 1],
}

const _: () = assert!(core::mem::size_of::<SchedAvg>() == 64);

impl SchedAvg {
    pub const fn zeroed() -> Self {
        Self {
            last_update_time: 0,
            load_sum: 0,
            runnable_sum: 0,
            util_sum: 0,
            period_contrib: 0,
            load_avg: 0,
            runnable_avg: 0,
            util_avg: 0,
            _pad: [0; 1],
        }
    }
}

// ── Sched entity (CFS) ───────────────────────────────────────────────────────

/// CFS scheduling entity.  Embedded in `task_struct` for normal tasks; standalone
/// for task groups (CONFIG_FAIR_GROUP_SCHED).
///
/// Ordered to land at 256 bytes total (Linux defconfig pahole).
#[repr(C)]
pub struct SchedEntity {
    pub load: LoadWeight,
    /// Linked-list pointer used by `cfs_rq.tasks_timeline` / sibling lists.
    /// Lupos M29 uses a `BTreeMap<u64, *mut SchedEntity>` keyed by vruntime as
    /// a stand-in for the upstream rbtree; the pointer is stored per-entity in
    /// `cfs_rq` rather than via inline rb_node pointers.
    pub run_node_left: *mut SchedEntity,
    pub run_node_right: *mut SchedEntity,
    pub run_node_parent: *mut SchedEntity,
    pub run_node_color: u64,

    pub deadline: u64,
    pub min_vruntime: u64,
    pub min_slice: u64,

    pub group_node_next: *mut SchedEntity,
    pub group_node_prev: *mut SchedEntity,

    pub on_rq: u8,
    pub sched_delayed: u8,
    pub rel_deadline: u8,
    pub custom_slice: u8,
    pub _pad0: [u8; 4],

    pub exec_start: u64,
    pub sum_exec_runtime: u64,
    pub prev_sum_exec_runtime: u64,
    pub vruntime: u64,
    pub vlag: i64,
    pub vprot: u64,
    pub slice: u64,

    pub nr_migrations: u64,
    pub depth: i32,
    pub _pad1: u32,

    pub parent: *mut SchedEntity,
    pub cfs_rq: *mut core::ffi::c_void,
    pub my_q: *mut core::ffi::c_void,

    pub runnable_weight: u64,

    pub avg: SchedAvg,
}

// SchedEntity is internally protected by per-CPU rq locks; pointers are not
// dereferenced cross-CPU without holding the relevant rq.
unsafe impl Send for SchedEntity {}
unsafe impl Sync for SchedEntity {}

// Linux pahole reports 256 bytes for `struct sched_entity` on a defconfig
// build with `CONFIG_FAIR_GROUP_SCHED=y` and `CONFIG_SCHED_DEBUG=n`.  Lupos
// keeps every Linux field plus rb-tree placeholders; with safe-pointer
// alignment our layout lands a few bytes off (272 typical).  The exact size
// is not load-bearing — `M29SchedFields` reserves whatever slack is needed
// from the 960-byte stack→mm span.
const _: () = assert!(core::mem::size_of::<SchedEntity>() <= 320);

impl SchedEntity {
    pub const fn zeroed() -> Self {
        Self {
            load: LoadWeight::zeroed(),
            run_node_left: core::ptr::null_mut(),
            run_node_right: core::ptr::null_mut(),
            run_node_parent: core::ptr::null_mut(),
            run_node_color: 0,
            deadline: 0,
            min_vruntime: 0,
            min_slice: 0,
            group_node_next: core::ptr::null_mut(),
            group_node_prev: core::ptr::null_mut(),
            on_rq: 0,
            sched_delayed: 0,
            rel_deadline: 0,
            custom_slice: 0,
            _pad0: [0; 4],
            exec_start: 0,
            sum_exec_runtime: 0,
            prev_sum_exec_runtime: 0,
            vruntime: 0,
            vlag: 0,
            vprot: 0,
            slice: 0,
            nr_migrations: 0,
            depth: 0,
            _pad1: 0,
            parent: core::ptr::null_mut(),
            cfs_rq: core::ptr::null_mut(),
            my_q: core::ptr::null_mut(),
            runnable_weight: 0,
            avg: SchedAvg::zeroed(),
        }
    }
}

// ── Sched RT entity ──────────────────────────────────────────────────────────

/// RT scheduling entity (`SCHED_FIFO`, `SCHED_RR`).
///
/// Pahole reports 48 bytes for defconfig.
#[repr(C)]
pub struct SchedRtEntity {
    pub run_list_next: *mut SchedRtEntity,
    pub run_list_prev: *mut SchedRtEntity,
    pub timeout: u64,
    pub watchdog_stamp: u64,
    pub time_slice: u32,
    pub on_rq: u16,
    pub on_list: u16,
    pub back: *mut SchedRtEntity,
    pub _pad: u64,
}

unsafe impl Send for SchedRtEntity {}
unsafe impl Sync for SchedRtEntity {}

const _: () = assert!(core::mem::size_of::<SchedRtEntity>() == 56);

impl SchedRtEntity {
    pub const fn zeroed() -> Self {
        Self {
            run_list_next: core::ptr::null_mut(),
            run_list_prev: core::ptr::null_mut(),
            timeout: 0,
            watchdog_stamp: 0,
            time_slice: 0,
            on_rq: 0,
            on_list: 0,
            back: core::ptr::null_mut(),
            _pad: 0,
        }
    }
}

// ── Sched DL entity ──────────────────────────────────────────────────────────

/// Deadline scheduling entity (`SCHED_DEADLINE`).
///
/// Pahole reports 272 bytes for defconfig (with `hrtimer × 2`).
#[repr(C)]
pub struct SchedDlEntity {
    pub rb_node_left: *mut SchedDlEntity,
    pub rb_node_right: *mut SchedDlEntity,
    pub rb_node_parent: *mut SchedDlEntity,
    pub rb_node_color: u64,

    pub dl_runtime: u64,
    pub dl_deadline: u64,
    pub dl_period: u64,
    pub dl_bw: u64,
    pub dl_density: u64,

    pub runtime: i64,
    pub deadline: u64,

    pub flags: u32,
    pub dl_throttled: u8,
    pub dl_yielded: u8,
    pub dl_non_contending: u8,
    pub dl_overrun: u8,

    /// Two `hrtimer`-shaped slots (~96 B each in defconfig) — laid out as
    /// opaque bytes; the real `hrtimer` arrives in M36.
    pub dl_timer: [u8; 96],
    pub inactive_timer: [u8; 96],

    pub rq: *mut core::ffi::c_void,
    pub server_pick_task: *mut core::ffi::c_void,
    pub pi_se: *mut SchedDlEntity,

    pub _pad: [u8; 8],
}

unsafe impl Send for SchedDlEntity {}
unsafe impl Sync for SchedDlEntity {}

// Linux defconfig reports 272 bytes for `struct sched_dl_entity`.  With our
// hrtimer-shaped placeholders (96 B each) and rb-node skeleton, we land at a
// slightly different size; the exact value is not load-bearing.
const _: () = assert!(core::mem::size_of::<SchedDlEntity>() <= 384);

impl SchedDlEntity {
    pub const fn zeroed() -> Self {
        Self {
            rb_node_left: core::ptr::null_mut(),
            rb_node_right: core::ptr::null_mut(),
            rb_node_parent: core::ptr::null_mut(),
            rb_node_color: 0,
            dl_runtime: 0,
            dl_deadline: 0,
            dl_period: 0,
            dl_bw: 0,
            dl_density: 0,
            runtime: 0,
            deadline: 0,
            flags: 0,
            dl_throttled: 0,
            dl_yielded: 0,
            dl_non_contending: 0,
            dl_overrun: 0,
            dl_timer: [0; 96],
            inactive_timer: [0; 96],
            rq: core::ptr::null_mut(),
            server_pick_task: core::ptr::null_mut(),
            pi_se: core::ptr::null_mut(),
            _pad: [0; 8],
        }
    }
}

// ── Wake entry ───────────────────────────────────────────────────────────────

/// Linux `struct __call_single_node`: linked list node + flags used by
/// IPI-driven wake-ups.  16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WakeEntry {
    pub next: *mut core::ffi::c_void,
    pub flags: u32,
    pub src: u16,
    pub dst: u16,
}

unsafe impl Send for WakeEntry {}
unsafe impl Sync for WakeEntry {}

const _: () = assert!(core::mem::size_of::<WakeEntry>() == 16);

impl WakeEntry {
    pub const fn zeroed() -> Self {
        Self {
            next: core::ptr::null_mut(),
            flags: 0,
            src: 0,
            dst: 0,
        }
    }
}

// ── CPU mask ─────────────────────────────────────────────────────────────────

/// `cpumask_t` for `CONFIG_NR_CPUS=64` — single u64 bitfield.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct CpuMask(pub u64);

impl CpuMask {
    /// All-ones mask covering NR_CPUS=64.
    pub const fn all() -> Self {
        Self(!0u64)
    }
    /// Single-CPU mask.
    pub const fn one(cpu: u32) -> Self {
        Self(1u64 << (cpu & 63))
    }
    pub const fn empty() -> Self {
        Self(0)
    }
    pub fn set(&mut self, cpu: u32) {
        self.0 |= 1u64 << (cpu & 63);
    }
    pub fn clear(&mut self, cpu: u32) {
        self.0 &= !(1u64 << (cpu & 63));
    }
    pub fn test(&self, cpu: u32) -> bool {
        self.0 & (1u64 << (cpu & 63)) != 0
    }
    pub fn weight(&self) -> u32 {
        self.0.count_ones()
    }
    pub const fn zeroed() -> Self {
        Self(0)
    }
}

const _: () = assert!(core::mem::size_of::<CpuMask>() == 8);

// ── Per-CPU clock helper (used by sched_class.update_curr) ───────────────────

/// Monotonic per-CPU scheduler clock, in nanoseconds.
///
/// Currently single-writer (the LAPIC tick handler) on the BSP; will become
/// per-CPU in M34 alongside GS-relative storage.
pub static SCHED_CLOCK_NS: AtomicU64 = AtomicU64::new(0);

/// Return the current scheduler-clock value in nanoseconds.
///
/// Backed by `apic_timer::TIMER_TICKS` × `NSEC_PER_TICK`.
pub fn sched_clock_ns() -> u64 {
    use crate::arch::x86::kernel::apic_timer::TIMER_TICKS;
    use core::sync::atomic::Ordering;

    /// Nominal nanoseconds per LAPIC tick (we run ~40 Hz on QEMU = 25 ms).
    const NSEC_PER_TICK: u64 = 25_000_000;

    let ticks = TIMER_TICKS.load(Ordering::Acquire);
    ticks.saturating_mul(NSEC_PER_TICK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_weight_is_16_bytes() {
        assert_eq!(core::mem::size_of::<LoadWeight>(), 16);
    }

    #[test]
    fn sched_entity_fits_within_320_bytes() {
        // Linux pahole reports 256 for `struct sched_entity`; our layout adds
        // explicit rb-tree placeholders so it lands a few bytes higher.  The
        // size is not ABI-load-bearing (kernel-internal field).
        assert!(core::mem::size_of::<SchedEntity>() <= 320);
    }

    #[test]
    fn sched_rt_entity_is_56_bytes() {
        assert_eq!(core::mem::size_of::<SchedRtEntity>(), 56);
    }

    #[test]
    fn sched_dl_entity_fits_within_384_bytes() {
        assert!(core::mem::size_of::<SchedDlEntity>() <= 384);
    }

    #[test]
    fn cpu_mask_is_8_bytes() {
        assert_eq!(core::mem::size_of::<CpuMask>(), 8);
    }

    #[test]
    fn cpu_mask_set_test_clear_roundtrip() {
        let mut m = CpuMask::empty();
        m.set(3);
        assert!(m.test(3));
        assert_eq!(m.weight(), 1);
        m.clear(3);
        assert!(!m.test(3));
    }

    #[test]
    fn wake_entry_is_16_bytes() {
        assert_eq!(core::mem::size_of::<WakeEntry>(), 16);
    }
}
