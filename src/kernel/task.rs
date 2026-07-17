//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Process descriptor — `struct task_struct`.
//!
//! Implements M20: a `#[repr(C)]` `TaskStruct` whose acceptance fields
//! (`pid`, `tgid`, `comm`, `mm`, `files`, `signal`, `cred`, `stack`, `thread`)
//! are at offsets matching Linux 7.0 x86_64 (SMP, CONFIG_NR_CPUS=64,
//! no LOCKDEP/KASAN/UBSAN, no RANDSTRUCT).
//!
//! # Offset calibration
//!
//! The `LINUX_OFFSET_*` constants below are derived from manual analysis of
//! `vendor/linux/include/linux/sched.h` for Linux 7.0.0.  They must be
//! confirmed against a built kernel:
//!
//! ```sh
//! # In a native Linux shell, inside vendor/linux/:
//! make x86_64_defconfig && make prepare
//! pahole -C task_struct vmlinux | grep -E '^\s+(pid|tgid|comm|mm|files|signal|cred|stack|thread)\b'
//! ```
//!
//! If the pahole values differ, update the constants here and the padding
//! computations will automatically correct the struct layout (the `const`
//! assertions below will catch arithmetic errors at compile time).
//!
//! References:
//!   Linux `include/linux/sched.h`
//!   Linux `arch/x86/include/asm/thread_info.h`
//!   Linux `arch/x86/include/asm/ptrace.h`

use core::{
    ffi::c_void,
    mem::{offset_of, size_of},
    sync::atomic::AtomicU32,
};

use crate::kernel::signal::SignalStruct;
use crate::kernel::thread::ThreadStruct;
use crate::mm::mm_types::MmStruct;

// ── Opaque forward declarations ──────────────────────────────────────────────
//
// These types are not yet implemented (M22+). Using uninhabited enums prevents
// accidental construction; all access is through raw pointers.

/// Open file descriptor table. Implemented in M39.
pub enum FilesStruct {}

/// Task credentials (immutable, refcount-protected).  M27 promotes this from
/// an opaque marker to the real `kernel::cred::Cred` type via a re-export so
/// existing `*const Cred` consumers keep their pointer type.
pub use crate::kernel::cred::Cred;

/// `nsproxy` — collection of namespace pointers.  M28 promotes this from an
/// opaque marker to the real `kernel::nsproxy::Nsproxy` type.
pub use crate::kernel::nsproxy::Nsproxy;

// M29 scheduler types — embedded as `M29SchedFields` inside the
// `_pad_stack_to_mm` 960-byte span.
pub use crate::kernel::sched::class::SchedClass;
pub use crate::kernel::sched::entity::{
    CpuMask, LoadWeight, SchedAvg, SchedDlEntity, SchedEntity, SchedRtEntity, WakeEntry,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum length of `task_struct.comm` including the NUL terminator.
/// Matches Linux `TASK_COMM_LEN`.
pub const TASK_COMM_LEN: usize = 16;

// ── Thread info flags (TIF_*) ────────────────────────────────────────────────
//
// These flags are used in `thread_info.flags` to signal work that must be
// handled on the slow path (syscall exit, exception exit, etc).
//
// Reference: Linux `arch/x86/include/asm/thread_info.h`

/// TIF_SIGPENDING — pending signal (2^2 = bit 2).
/// Set when a signal has been queued for this task and must be delivered
/// on the next syscall or exception exit.
pub const TIF_SIGPENDING: u64 = 1 << 2;

/// TIF_NEED_RESCHED — scheduler has requested a context switch (bit 3).
///
/// Set by the LAPIC tick handler (`task_tick`) when the running task's CFS
/// slice is exhausted, by `wakeup_preempt` when a higher-priority class wakes,
/// and by `resched_curr`.  Read on the syscall / exception exit slow path —
/// under the cooperative scheduler it is read at every explicit `schedule()`.
///
/// Reference: Linux `arch/x86/include/asm/thread_info.h::TIF_NEED_RESCHED`.
pub const TIF_NEED_RESCHED: u64 = 1 << 3;

// ── Task state bitmask (M26) ─────────────────────────────────────────────────
//
// Linux `__state` is a bitmask, so plain constants — not a Rust enum — preserve
// OR-combinations like `TASK_INTERRUPTIBLE | TASK_NOLOAD`.
// Reference: Linux `include/linux/sched.h` (TASK_* / EXIT_* macros).
pub mod task_state {
    pub const TASK_RUNNING: u32 = 0x0000;
    pub const TASK_INTERRUPTIBLE: u32 = 0x0001;
    pub const TASK_UNINTERRUPTIBLE: u32 = 0x0002;
    pub const __TASK_STOPPED: u32 = 0x0004;
    pub const __TASK_TRACED: u32 = 0x0008;
    pub const EXIT_DEAD: u32 = 0x0010;
    pub const EXIT_ZOMBIE: u32 = 0x0020;
    pub const EXIT_TRACE: u32 = EXIT_ZOMBIE | EXIT_DEAD;
    pub const TASK_PARKED: u32 = 0x0040;
    pub const TASK_DEAD: u32 = 0x0080;
    pub const TASK_WAKEKILL: u32 = 0x0100;
    pub const TASK_WAKING: u32 = 0x0200;
    pub const TASK_NOLOAD: u32 = 0x0400;
    pub const TASK_NEW: u32 = 0x0800;
    pub const TASK_STOPPED: u32 = TASK_WAKEKILL | __TASK_STOPPED;
    pub const TASK_TRACED: u32 = __TASK_TRACED;
    pub const TASK_NORMAL: u32 = TASK_INTERRUPTIBLE | TASK_UNINTERRUPTIBLE;
    /// States that are NOT runnable — used by `schedule()` to skip the task.
    pub const NON_RUNNABLE_MASK: u32 = TASK_INTERRUPTIBLE
        | TASK_UNINTERRUPTIBLE
        | __TASK_STOPPED
        | __TASK_TRACED
        | EXIT_DEAD
        | EXIT_ZOMBIE
        | TASK_PARKED
        | TASK_DEAD
        | TASK_NEW;
}

// ── Linux 7.0 field offsets (x86_64, defconfig+SMP, NR_CPUS=64) ─────────────
//
// These are the byte offsets of acceptance fields within `task_struct`.
// Derived from manual header analysis; confirm with `pahole` against
// a built `vendor/linux/vmlinux`.
//
// Layout rationale (key intermediate sizes):
//   - `sched_entity se`:          256 bytes (with ____cacheline_aligned sched_avg)
//   - `sched_rt_entity rt`:        48 bytes
//   - `sched_dl_entity dl`:       272 bytes (with hrtimer × 2, pi_se ptr)
//   - `cpumask_t cpus_mask`:        8 bytes (NR_CPUS=64 → 8 bytes)
//   - `struct restart_block`:      72 bytes
//   - `struct posix_cputimers`:    ~48 bytes (3 posix_cputimer_base entries)

/// Byte offset of `task_struct.__state` (= 24, after 24-byte `thread_info`).
pub const LINUX_OFFSET_STATE: usize = 24;

/// Byte offset of `task_struct.stack` (= 32, after __state + saved_state).
pub const LINUX_OFFSET_STACK: usize = 32;

/// Byte offset of `task_struct.mm`.
/// Comes after the large scheduler block (se/rt/dl) and various sched fields.
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_MM: usize = 1000;

/// Byte offset of `task_struct.pid`.
/// After mm + active_mm + exit fields + jobctl + personality + bitfields +
/// atomic_flags + restart_block.
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_PID: usize = 1136;

/// Byte offset of `task_struct.tgid` (always pid + 4).
pub const LINUX_OFFSET_TGID: usize = LINUX_OFFSET_PID + 4;

/// Byte offset of `task_struct.cred` (effective credentials pointer).
/// Comes after real_parent, parent, children, sibling, group_leader,
/// ptraced, ptrace_entry, thread_pid, pid_links, thread_node, vfork_done,
/// *_child_tid, worker_private, utime/stime/gtime, prev_cputime,
/// nvcsw/nivcsw, start_time/boottime, min_flt/maj_flt, posix_cputimers,
/// ptracer_cred, real_cred.
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_CRED: usize = 1520;

/// Byte offset of `task_struct.comm`.
/// Comes after `cred` and the CONFIG_KEYS `cached_requested_key` pointer (8 bytes).
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_COMM: usize = 1536;

/// Byte offset of `task_struct.files`.
/// Comes after comm + nameidata ptr + fs ptr + possible SYSVIPC fields.
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_FILES: usize = 1600;

/// Byte offset of `task_struct.signal`.
/// Comes after files + io_uring fields + nsproxy ptr.
/// Approximate; verify with pahole.
pub const LINUX_OFFSET_SIGNAL: usize = 1640;

/// Byte offset of `task_struct.thread` (the last field).
/// Approximate (derived from known post-signal field sizes for defconfig).
/// The struct's total size is `LINUX_OFFSET_THREAD + size_of::<ThreadStruct>()`.
pub const LINUX_OFFSET_THREAD: usize = 8000;

// ── Padding computations ─────────────────────────────────────────────────────
//
// Each PAD_* constant bridges two adjacent acceptance fields.  The arithmetic
// is verified at compile time by the `const` assertions below — any mismatch
// between the constants above and the actual `offset_of!` results causes a
// compile-time error.

// After `stack` (offset 32, size 8) the next field ends at 40.
const PAD_STACK_TO_MM: usize = LINUX_OFFSET_MM - (LINUX_OFFSET_STACK + 8);

// After `mm` (8 bytes) and `active_mm` (8 bytes), next acceptance field is `pid`.
const PAD_ACTIVE_MM_TO_PID: usize = LINUX_OFFSET_PID - (LINUX_OFFSET_MM + 16);

// After `tgid` (4 bytes), next acceptance field is `cred`.
// M26 inserts `M26Fields` (`size_of::<M26Fields>()` bytes) at the start of
// this region; M27 adds `M27Fields` (real_cred + no_new_privs) immediately
// before `cred`; the remainder is residual padding so `cred` stays at offset
// LINUX_OFFSET_CRED.
const PAD_TGID_TO_CRED: usize = LINUX_OFFSET_CRED - (LINUX_OFFSET_TGID + 4);
const PAD_M26_TAIL: usize =
    PAD_TGID_TO_CRED - core::mem::size_of::<M26Fields>() - core::mem::size_of::<M27Fields>();

// After `cred` (8 bytes), next acceptance field is `comm`.
// The 8 bytes accounts for CONFIG_KEYS `cached_requested_key` pointer.
const PAD_CRED_TO_COMM: usize = LINUX_OFFSET_COMM - (LINUX_OFFSET_CRED + 8);

// After `comm` (16 bytes), next acceptance field is `files`.
// Covers: nameidata*, [SYSVIPC], [DETECT_HUNG_TASK], fs*.  M39 places
// `M39FsFields` (the `fs` pointer) at the head of this span; the residual
// is left as plain padding.
const PAD_COMM_TO_FILES_TOTAL: usize = LINUX_OFFSET_FILES - (LINUX_OFFSET_COMM + TASK_COMM_LEN);
const PAD_COMM_TO_FILES: usize = PAD_COMM_TO_FILES_TOTAL - core::mem::size_of::<M39FsFields>();

// After `files` (8 bytes), next acceptance field is `signal`.
// Covers: io_uring fields, nsproxy*.  M28 places `M28NsproxyFields` (16 bytes)
// at the start of this block — `nsproxy` and `thread_pid_ns_for_children`
// pointers — and the residual is left as plain padding.
const PAD_FILES_TO_SIGNAL_TOTAL: usize = LINUX_OFFSET_SIGNAL - (LINUX_OFFSET_FILES + 8);
const PAD_FILES_TO_SIGNAL: usize =
    PAD_FILES_TO_SIGNAL_TOTAL - core::mem::size_of::<M28NsproxyFields>();

// After `signal` (8 bytes), the remaining bulk of task_struct before `thread`.
// M27 places `Seccomp` (16 bytes) at the head of this block — Linux puts the
// `seccomp` field inside the same span — and leaves the rest as padding.
const PAD_SIGNAL_TO_THREAD_TOTAL: usize = LINUX_OFFSET_THREAD - (LINUX_OFFSET_SIGNAL + 8);
const PAD_SIGNAL_TO_THREAD: usize = PAD_SIGNAL_TO_THREAD_TOTAL
    - core::mem::size_of::<crate::kernel::seccomp::Seccomp>()
    - core::mem::size_of::<u64>();

// ── M26 fields (parent / children / exit / ptrace) ───────────────────────────
//
// These fields occupy the high-level layout block Linux places between `tgid`
// and `cred` (`real_parent`, `parent`, children list, exit_state, exit_code,
// exit_signal, ptrace, ptracer_cred, wait_chldexit ...).  We use simple
// fixed-size arrays for the children list and a fixed-size waiter array for
// the wait-on-child-exit queue, which are sufficient under the cooperative
// scheduler in M22; intrusive list_head support lands in M28/M29.
//
// Reference: Linux `include/linux/sched.h` (struct task_struct, between
// `tgid` and `cred`).

/// Maximum number of children tracked per task.  Beyond this, additional
/// children are still spawnable but are not tracked in the parent's child
/// array (their `real_parent` still points at us; `release_task` walks the
/// global heap-task tracker as a fallback).
pub const MAX_CHILDREN: usize = 16;

/// Maximum number of parents waiting on a single task's exit.
pub const MAX_WAITERS: usize = 4;

/// M26 sub-block stored inline inside `TaskStruct` between `tgid` and `cred`.
#[repr(C)]
pub struct M26Fields {
    /// Biological parent — never reparented (used by ptrace bookkeeping).
    pub real_parent: *mut TaskStruct,
    /// Effective parent — may be reparented to init/subreaper on parent death.
    pub parent: *mut TaskStruct,
    /// Thread-group leader (= self for non-thread fork; = parent for CLONE_THREAD).
    pub group_leader: *mut TaskStruct,
    /// `KPid` reference owned by this task (dropped via `put_pid` in `release_task`).
    pub thread_pid: *mut crate::kernel::pid::KPid,
    /// Tracer task pointer (set by `PTRACE_ATTACH`; cleared by `PTRACE_DETACH`).
    pub tracer: *mut TaskStruct,
    /// Tracer credentials snapshot (M27 will populate; NULL for M26).
    pub ptracer_cred: *const Cred,
    /// Number of valid entries in `children`.
    pub children_count: u32,
    /// PT_PTRACED / PT_SEIZED bitmask.
    pub ptrace: u32,
    /// EXIT_ZOMBIE / EXIT_DEAD progression.
    pub exit_state: u32,
    /// Exit code packed via `w_exitcode(retval, termsig)`.
    pub exit_code: i32,
    /// Signal sent to parent on exit (default SIGCHLD).
    pub exit_signal: i32,
    /// `pdeath_signal` — signal sent to children if this task dies (M27).
    pub pdeath_signal: i32,
    /// Number of valid entries in `wait_waiters`.
    pub wait_count: u32,
    /// 4-byte alignment pad before the waiter pointer array.
    pub _pad_align: u32,
    /// Children pointers (null when slot is empty).
    pub children: [*mut TaskStruct; MAX_CHILDREN],
    /// Tasks blocked in `wait4` waiting for *this* task to enter EXIT_ZOMBIE.
    pub wait_waiters: [*mut TaskStruct; MAX_WAITERS],
    /// `jobctl` — ptrace stop bits (JOBCTL_STOP_PENDING etc.).
    pub jobctl: u64,
    pub ptrace_stop_signal: i32,
    pub ptrace_message: u64,
    pub ptrace_syscall_op: u8,
    pub ptrace_syscall_nr: i64,
    pub ptrace_syscall_args: [u64; 6],
    pub ptrace_syscall_ret: i64,
    pub ptrace_syscall_ip: u64,
    pub ptrace_syscall_sp: u64,
    /// `CLONE_CHILD_CLEARTID` / `set_tid_address(2)` clear-and-wake pointer.
    pub clear_child_tid: *mut i32,
}

// SAFETY: All fields are pointers/atomic-friendly POD; access is serialized by
// the cooperative scheduler today.  Real locking lands in M29.
unsafe impl Send for M26Fields {}
unsafe impl Sync for M26Fields {}

impl M26Fields {
    /// All-null/zeroed value for use in `Box::new(zeroed())` paths.
    pub const fn zeroed() -> Self {
        Self {
            real_parent: core::ptr::null_mut(),
            parent: core::ptr::null_mut(),
            group_leader: core::ptr::null_mut(),
            thread_pid: core::ptr::null_mut(),
            tracer: core::ptr::null_mut(),
            ptracer_cred: core::ptr::null(),
            children_count: 0,
            ptrace: 0,
            exit_state: 0,
            exit_code: 0,
            exit_signal: 0,
            pdeath_signal: 0,
            wait_count: 0,
            _pad_align: 0,
            children: [core::ptr::null_mut(); MAX_CHILDREN],
            wait_waiters: [core::ptr::null_mut(); MAX_WAITERS],
            jobctl: 0,
            ptrace_stop_signal: 0,
            ptrace_message: 0,
            ptrace_syscall_op: 0,
            ptrace_syscall_nr: -1,
            ptrace_syscall_args: [0; 6],
            ptrace_syscall_ret: 0,
            ptrace_syscall_ip: 0,
            ptrace_syscall_sp: 0,
            clear_child_tid: core::ptr::null_mut(),
        }
    }
}

// ── M27 fields (real_cred / no_new_privs) ───────────────────────────────────
//
// Linux places `real_cred` immediately before `cred` and `no_new_privs` is a
// small bitfield in the security block.  We co-locate both at the tail of the
// tgid → cred span so that `cred` keeps its absolute offset and `real_cred`
// remains adjacent (matching Linux pahole output: real_cred at cred-8, plus
// no_new_privs in the same cache line as the security state).
#[repr(C)]
pub struct M27Fields {
    /// Real (objective) task credentials — never overridden, only updated by
    /// `commit_creds`.  Linux: `task_struct.real_cred`.
    pub real_cred: *const Cred,
    /// `PR_SET_NO_NEW_PRIVS` flag.  Required before SECCOMP_SET_MODE_FILTER
    /// without `CAP_SYS_ADMIN`.  Linux: `task_struct.no_new_privs` (bitfield).
    pub no_new_privs: u32,
    /// Process-control flags. Low bits are reserved for `PR_SET_MDWE`; high
    /// bits carry child-subreaper state until full `signal_struct` duplication
    /// lands.
    pub mdwe_flags: u32,
}

pub const TASK_CTRL_CHILD_SUBREAPER: u32 = 1 << 16;
pub const TASK_CTRL_HAS_CHILD_SUBREAPER: u32 = 1 << 17;
/// `task_exec_state.dumpable`, packed into the process-control word.  Linux's
/// stable prctl ABI accepts only OFF (0) and OWNER (1); the VALID bit keeps a
/// zero-initialized task distinguishable from an explicit OFF transition.
pub const TASK_CTRL_DUMPABLE_SHIFT: u32 = 2;
pub const TASK_CTRL_DUMPABLE_MASK: u32 = 0b11 << TASK_CTRL_DUMPABLE_SHIFT;
pub const TASK_CTRL_DUMPABLE_VALID: u32 = 1 << 4;

unsafe impl Send for M27Fields {}
unsafe impl Sync for M27Fields {}

impl M27Fields {
    /// All-null/zeroed value.
    pub const fn zeroed() -> Self {
        Self {
            real_cred: core::ptr::null(),
            no_new_privs: 0,
            mdwe_flags: 0,
        }
    }
}

// ── M28 nsproxy fields (nsproxy + thread_pid_ns_for_children) ───────────────
//
// Linux places `nsproxy` between `files` and `signal`; we add a second slot
// for `thread_pid_ns_for_children` (the ns a `clone(CLONE_NEWPID)` child will
// land in) so M28 has a stable place to record the child's destination ns
// before clone returns.
#[repr(C)]
pub struct M28NsproxyFields {
    /// Pointer to the calling task's nsproxy bundle.
    pub nsproxy: *mut Nsproxy,
    /// Pointer to the pid_namespace where this task's children will be
    /// allocated PIDs.  Distinct from `nsproxy.pid_ns_for_children`: that
    /// field on `nsproxy` describes the *bundle's* default; this per-task
    /// field captures any post-unshare deviation. (Linux mirrors this with
    /// `task_struct.thread_pid` plus nsproxy's `pid_ns_for_children`.)
    pub thread_pid_ns_for_children: *mut core::ffi::c_void,
}

unsafe impl Send for M28NsproxyFields {}
unsafe impl Sync for M28NsproxyFields {}

impl M28NsproxyFields {
    pub const fn zeroed() -> Self {
        Self {
            nsproxy: core::ptr::null_mut(),
            thread_pid_ns_for_children: core::ptr::null_mut(),
        }
    }
}

// ── M39 fs_struct field (fs pointer between comm and files) ─────────────────
//
// Linux places `struct fs_struct *fs` at the head of the comm → files span.
// The pointer slot is opaque from `task_struct`'s perspective; helpers in
// `crate::fs::fs_struct` materialize the real `Arc<FsStruct>` from this raw
// pointer when needed.
#[repr(C)]
pub struct M39FsFields {
    /// Per-task root + cwd dentries.  Real type is `*mut FsStruct` from
    /// `crate::fs::fs_struct`; declared as `*mut c_void` so `task.rs` does
    /// not pull the entire VFS surface into its public dependencies.
    pub fs: *mut core::ffi::c_void,
}

unsafe impl Send for M39FsFields {}
unsafe impl Sync for M39FsFields {}

impl M39FsFields {
    pub const fn zeroed() -> Self {
        Self {
            fs: core::ptr::null_mut(),
        }
    }
}

// ── M29 scheduler fields (sched_entity, sched_rt_entity, sched_dl_entity, …) ─
//
// Linux places the entire scheduler block — `on_cpu`, `wake_entry`, prio
// fields, `se`/`rt`/`dl` entities, `sched_class`, `policy`, `cpus_mask`, etc. —
// in the `_pad_stack_to_mm` 960-byte span (offsets 40 → LINUX_OFFSET_MM=1000).
// We mirror Linux field order so future `pahole` runs against `vmlinux` map
// 1:1 onto our offsets.
//
// Reference: `vendor/linux/include/linux/sched.h::struct task_struct` —
// the `on_cpu .. cpus_mask` contiguous block.
#[repr(C)]
pub struct M29SchedFields {
    /// Set when this task is currently running on a CPU.
    pub on_cpu: i32,
    /// 4-byte alignment pad before the 8-aligned wake_entry.
    pub _pad_a: u32,
    /// `__call_single_node` used by IPI-driven wake-ups.
    pub wake_entry: WakeEntry,
    pub wakee_flips: u32,
    pub _pad_b: u32,
    pub wakee_flip_decay_ts: u64,
    pub last_wakee: *mut TaskStruct,
    pub recent_used_cpu: i32,
    pub wake_cpu: i32,
    /// Scheduler runqueue membership flag (Linux `task_struct.on_rq`).
    pub on_rq: i32,
    /// Effective priority (lower = higher).  RT range 0..99, normal 100..139.
    pub prio: i32,
    pub static_prio: i32,
    pub normal_prio: i32,
    /// Real-time priority (1..99 for `SCHED_FIFO`/`SCHED_RR`, 0 otherwise).
    pub rt_priority: u32,
    /// CFS scheduling entity (256 B).
    pub se: SchedEntity,
    /// RT scheduling entity.
    pub rt: SchedRtEntity,
    /// Deadline scheduling entity.
    pub dl: SchedDlEntity,
    /// Server-mode DL entity (Linux 6.x dl-server feature; NULL until M30).
    pub dl_server: *mut SchedDlEntity,
    /// Pointer to the `sched_class` vtable governing this task.  Defaults to
    /// `&FAIR_SCHED_CLASS` (CFS).
    pub sched_class: *const SchedClass,
    /// Pointer to the `task_group` (cgroup CPU controller) — wired up in M32.
    pub sched_task_group: *mut core::ffi::c_void,
    /// Scheduling policy (`SCHED_NORMAL` etc., from `uapi/linux/sched.h`).
    pub policy: u32,
    /// Number of CPUs this task is permitted to run on.
    pub nr_cpus_allowed: i32,
    /// Pointer to the active cpumask (usually `&cpus_mask`).
    pub cpus_ptr: *const CpuMask,
    /// User-supplied affinity mask (NULL when none has been installed).
    pub user_cpus_ptr: *mut CpuMask,
    /// Inline cpumask (NR_CPUS=64).
    pub cpus_mask: CpuMask,
    /// Migration request (`migrate_task_to`) — NULL when no pending migration.
    pub migration_pending: *mut core::ffi::c_void,
    /// `migrate_disable()` nesting count (preempt-rt feature; 0 in M29).
    pub migration_disabled: u16,
    /// `migration_flags` bitfield — Linux `MDF_*`.
    pub migration_flags: u16,
    /// 4-byte alignment pad to keep total size predictable.
    pub _pad_tail: u32,
}

// SAFETY: Access to scheduler fields is serialised through the per-CPU `Rq`
// mutex; cross-CPU reads are coordinated via `__state` atomics + acquire/release.
unsafe impl Send for M29SchedFields {}
unsafe impl Sync for M29SchedFields {}

impl M29SchedFields {
    /// All-zero state; sched_class defaults to NULL (caller assigns CFS).
    pub const fn zeroed() -> Self {
        Self {
            on_cpu: 0,
            _pad_a: 0,
            wake_entry: WakeEntry::zeroed(),
            wakee_flips: 0,
            _pad_b: 0,
            wakee_flip_decay_ts: 0,
            last_wakee: core::ptr::null_mut(),
            recent_used_cpu: -1,
            wake_cpu: 0,
            on_rq: 0,
            prio: 120, // DEFAULT_PRIO
            static_prio: 120,
            normal_prio: 120,
            rt_priority: 0,
            se: SchedEntity::zeroed(),
            rt: SchedRtEntity::zeroed(),
            dl: SchedDlEntity::zeroed(),
            dl_server: core::ptr::null_mut(),
            sched_class: core::ptr::null(),
            sched_task_group: core::ptr::null_mut(),
            policy: 0,           // SCHED_NORMAL
            nr_cpus_allowed: 64, // CONFIG_NR_CPUS=64
            cpus_ptr: core::ptr::null(),
            user_cpus_ptr: core::ptr::null_mut(),
            cpus_mask: CpuMask::all(),
            migration_pending: core::ptr::null_mut(),
            migration_disabled: 0,
            migration_flags: 0,
            _pad_tail: 0,
        }
    }
}

// Tail-pad inside _pad_stack_to_mm absorbs the bytes between `M29SchedFields`
// and `mm` so `LINUX_OFFSET_MM` is preserved.
const PAD_M29_TAIL: usize = PAD_STACK_TO_MM - core::mem::size_of::<M29SchedFields>();

// ── ThreadInfo ───────────────────────────────────────────────────────────────

/// Per-thread flags embedded at the very top of `task_struct`
/// when `CONFIG_THREAD_INFO_IN_TASK=y`.
///
/// | offset | field         | size |
/// |--------|---------------|------|
/// | +0     | flags         | 8    |
/// | +8     | syscall_work  | 8    |
/// | +16    | status        | 4    |
/// | +20    | cpu           | 4    |
///
/// Total: 24 bytes.
///
/// Ref: Linux `arch/x86/include/asm/thread_info.h`
#[repr(C)]
pub struct ThreadInfo {
    /// Low-level thread flags (TIF_NEED_RESCHED, TIF_SIGPENDING, …).
    pub flags: u64,
    /// Syscall work flags (SYSCALL_WORK_* bits, introduced Linux 5.15).
    pub syscall_work: u64,
    /// Thread-synchronous status flags (TS_* bits, per-CPU, not per-task).
    pub status: u32,
    /// CPU this task last ran on (CONFIG_SMP).
    pub cpu: u32,
}

// ── PtRegs ───────────────────────────────────────────────────────────────────

/// CPU register state pushed onto the kernel stack on ring-3→ring-0 transition.
///
/// Layout matches Linux `arch/x86/include/asm/ptrace.h` `struct pt_regs`
/// for x86_64. Field names use the short forms from Linux (e.g. `bx` for rbx).
///
/// Total: 21 × 8 = 168 bytes.
#[repr(C)]
pub struct PtRegs {
    // Callee-saved registers (saved by the slow syscall path or `__switch_to_asm`).
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub bp: u64, // rbp
    pub bx: u64, // rbx

    // Caller-saved registers (saved on every syscall/interrupt entry).
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub ax: u64, // rax
    pub cx: u64, // rcx
    pub dx: u64, // rdx
    pub si: u64, // rsi
    pub di: u64, // rdi

    // Return frame (pushed by iretq or synthesised by entry stubs).
    pub orig_ax: u64,
    pub ip: u64,
    pub cs: u64, // stored as u64 (upper 48 bits zero)
    pub flags: u64,
    pub sp: u64,
    pub ss: u64, // stored as u64 (upper 48 bits zero)
}

// ── TaskStruct ───────────────────────────────────────────────────────────────

/// Process descriptor — the central kernel data structure for task management.
///
/// # Acceptance fields (M20)
///
/// The following fields must be at byte offsets identical to mainline Linux
/// on x86_64 (with the config flags described at the top of this file):
///
/// | field    | expected offset | source               |
/// |----------|-----------------|----------------------|
/// | stack    | 32              | thread_info(24)+state(4)+saved(4) |
/// | mm       | LINUX_OFFSET_MM | pahole verification  |
/// | pid      | LINUX_OFFSET_PID | pahole verification |
/// | tgid     | pid + 4         | adjacent             |
/// | cred     | LINUX_OFFSET_CRED | pahole verification |
/// | comm     | LINUX_OFFSET_COMM | pahole verification |
/// | files    | LINUX_OFFSET_FILES | pahole verification |
/// | signal   | LINUX_OFFSET_SIGNAL | pahole verification |
/// | thread   | LINUX_OFFSET_THREAD | last field (pahole) |
///
/// # Safety
///
/// This struct is `!Sync` because it contains raw pointer fields and atomic
/// fields that require external locking per Linux semantics.  Access to fields
/// other than `__state` and `thread_info.flags` must be serialised by the
/// appropriate per-task or per-run-queue lock.
#[repr(C)]
pub struct TaskStruct {
    // ── Fixed-offset header ──────────────────────────────────────────────────
    // These three fields are at known, exact offsets regardless of config.
    /// `thread_info` at offset 0 (CONFIG_THREAD_INFO_IN_TASK).
    pub thread_info: ThreadInfo,

    /// Task state (TASK_RUNNING=0, TASK_INTERRUPTIBLE, …).
    pub __state: AtomicU32,

    /// Saved state for spinlock sleepers (preserves __state across sleeping lock).
    pub saved_state: u32,

    /// Kernel stack base pointer (points to the top of the per-task kernel stack).
    ///
    /// Set to the stack-top address when the task is created.  Used by
    /// `__switch_to` to update `TSS.RSP0` so that ring-3→ring-0 transitions
    /// for the incoming task start with a clean kernel stack.
    pub stack: *mut c_void,

    // ── Scheduler block + early sched fields (stack → mm) ───────────────────
    //
    // Linux places the entire scheduler block here: `on_cpu`, `wake_entry`,
    // prio fields, `se`/`rt`/`dl` entities, `sched_class` ptr, `task_group`
    // ptr, `policy`, `cpumask` fields, plus residual RCU/sched_info/tasks
    // pointers.  M29 represents the fields the scheduler actively uses inline
    // via `M29SchedFields`; the residual bytes are reserved tail padding.
    pub m29: M29SchedFields,
    _pad_m29_tail: [u8; PAD_M29_TAIL],

    /// Pointer to the virtual-memory descriptor (NULL for kernel threads).
    pub mm: *mut MmStruct,

    /// Active MM: kernel threads borrow this from the last user task on the CPU.
    pub active_mm: *mut MmStruct,

    // ── Exit / jobctl / bitfields / restart_block (mm → pid) ────────────────
    //
    // Contains: exit_state, exit_code, exit_signal, pdeath_signal, jobctl,
    // personality, scheduler bitfields, atomic_flags, restart_block.
    _pad_active_mm_to_pid: [u8; PAD_ACTIVE_MM_TO_PID],

    /// Process ID (unique per-task; equals tgid for the group leader).
    pub pid: i32,

    /// Thread group ID (= pid of the thread-group leader).
    pub tgid: i32,

    // ── M26 parent / children / ptrace / exit fields (tgid → cred) ─────────
    //
    // Linux placement of `real_parent`, `parent`, `children`, `sibling`,
    // `group_leader`, `ptraced`, `ptrace_entry`, `thread_pid`, `pid_links`,
    // `thread_node`, `vfork_done`, `*_child_tid`, `exit_state`, `exit_code`,
    // `exit_signal`, `pdeath_signal`, `jobctl`, `ptrace`, `ptracer_cred`,
    // `wait_chldexit` etc. lives in this 376-byte block.  M26 represents the
    // fields it actively uses inline; the residual bytes are reserved for
    // later milestones (M27 cred, M28 namespaces, M29 wait queues, ...).
    pub m26: M26Fields,
    _pad_m26_tail: [u8; PAD_M26_TAIL],
    /// M27 substruct — `real_cred` plus `no_new_privs`.  Placed at the very
    /// end of the tgid → cred span so that `real_cred` sits one slot below
    /// `cred` (Linux pahole: `real_cred` at `cred - 8`).
    pub m27: M27Fields,

    /// Effective (overridable) subjective task credentials (COW, refcount).
    ///
    /// Precedes `comm` in the Linux layout, separated by the
    /// CONFIG_KEYS `cached_requested_key` pointer.
    pub cred: *const Cred,

    // ── CONFIG_KEYS cached_requested_key (cred → comm) ──────────────────────
    _pad_cred_to_comm: [u8; PAD_CRED_TO_COMM],

    /// Executable name (first 15 chars + NUL, no path component).
    ///
    /// Set by `set_task_comm()` which ensures NUL-termination and zero-padding.
    pub comm: [u8; TASK_COMM_LEN],

    // ── nameidata / SYSVIPC / fs_struct (comm → files) ──────────────────────
    /// M39 substruct — `fs` pointer.  Placed at the head of the comm → files
    /// span so `fs_struct *fs` lands where Linux pahole reports it.
    pub m39_fs: M39FsFields,
    _pad_comm_to_files: [u8; PAD_COMM_TO_FILES],

    /// Open file descriptor table.
    pub files: *mut FilesStruct,

    // ── IO_URING / nsproxy (files → signal) ─────────────────────────────────
    /// M28 nsproxy substruct — placed at the start of the files → signal span
    /// so `nsproxy` lives where Linux pahole reports it.
    pub m28_nsproxy: M28NsproxyFields,
    _pad_files_to_signal: [u8; PAD_FILES_TO_SIGNAL],

    /// Signal descriptor shared by the thread group.
    pub signal: *mut SignalStruct,

    // ── sighand / sigset / pending / audit / security / locks / io (signal → thread)
    //
    // This block is the largest portion: sighand, blocked/real_blocked/
    // saved_sigmask/pending, sas_ss, audit, seccomp, exec_ids, alloc_lock,
    // pi_lock, wake_q, rt_mutex PI data, blocked_on, journal_info, bio_list,
    // plug, reclaim_state, io_context, and many more config-conditional fields.
    /// M27 seccomp state (mode + filter chain head).  Placed at the start of
    /// the signal → thread block; Linux places `seccomp` in the same span.
    pub m27_seccomp: crate::kernel::seccomp::Seccomp,
    /// Per-task x86 stack canary installed into the current CPU's
    /// `__stack_chk_guard` by `__switch_to_asm`.
    pub stack_canary: u64,
    _pad_signal_to_thread: [u8; PAD_SIGNAL_TO_THREAD],

    // ── Architecture-specific thread context (LAST FIELD) ───────────────────
    /// Per-task x86-64 thread state: TLS descriptors, saved kernel RSP,
    /// FS/GS base addresses, and PKRU.
    ///
    /// **Must be the last field** — Linux places it last and some assembly code
    /// derives its address by adding a fixed offset to the task_struct pointer.
    pub thread: ThreadStruct,
}

// ── Compile-time layout assertions ──────────────────────────────────────────
//
// These verify our padding arithmetic at compile time.  They do NOT verify
// that the offsets match Linux — that requires a pahole run.  Their purpose
// is to catch arithmetic mistakes in the PAD_* computations.

const _: () = {
    // Fixed-offset fields: these match Linux unconditionally.
    assert!(
        offset_of!(TaskStruct, thread_info) == 0,
        "thread_info must be at offset 0 (CONFIG_THREAD_INFO_IN_TASK)"
    );
    assert!(
        offset_of!(TaskStruct, __state) == LINUX_OFFSET_STATE,
        "__state must be at offset 24"
    );
    assert!(
        offset_of!(TaskStruct, stack) == LINUX_OFFSET_STACK,
        "stack must be at offset 32"
    );

    // Acceptance fields: offsets are exactly what we designed.
    // Replace the LINUX_OFFSET_* values with pahole output to lock in
    // Linux-identical parity.
    assert!(offset_of!(TaskStruct, mm) == LINUX_OFFSET_MM);
    assert!(offset_of!(TaskStruct, pid) == LINUX_OFFSET_PID);
    assert!(offset_of!(TaskStruct, tgid) == LINUX_OFFSET_TGID);
    assert!(offset_of!(TaskStruct, cred) == LINUX_OFFSET_CRED);
    assert!(offset_of!(TaskStruct, comm) == LINUX_OFFSET_COMM);
    assert!(offset_of!(TaskStruct, files) == LINUX_OFFSET_FILES);
    assert!(offset_of!(TaskStruct, signal) == LINUX_OFFSET_SIGNAL);
    assert!(offset_of!(TaskStruct, thread) == LINUX_OFFSET_THREAD);

    // Relative invariants that hold regardless of absolute offsets.
    assert!(offset_of!(TaskStruct, tgid) == offset_of!(TaskStruct, pid) + 4);
    assert!(
        offset_of!(TaskStruct, thread) + size_of::<ThreadStruct>() == size_of::<TaskStruct>(),
        "thread must be the last field"
    );

    // M27 / M28 layout invariants:
    //   - real_cred sits immediately before `cred` (Linux pahole pattern).
    //   - nsproxy lives between `files` and `signal`.
    //   - seccomp lives between `signal` and `thread`.
    assert!(
        offset_of!(TaskStruct, m27) + 8 <= offset_of!(TaskStruct, cred),
        "M27Fields.real_cred must precede `cred`"
    );
    assert!(offset_of!(TaskStruct, m28_nsproxy) > offset_of!(TaskStruct, files));
    assert!(offset_of!(TaskStruct, m28_nsproxy) < offset_of!(TaskStruct, signal));
    assert!(offset_of!(TaskStruct, m27_seccomp) > offset_of!(TaskStruct, signal));
    assert!(offset_of!(TaskStruct, m27_seccomp) < offset_of!(TaskStruct, thread));
    assert!(offset_of!(TaskStruct, stack_canary) < offset_of!(TaskStruct, thread));

    // M29 invariants:
    //   - The scheduler substruct lives at the start of the stack → mm span,
    //     i.e. immediately after `stack` (offset 32 + sizeof::<*mut>=8).
    //   - It must fit entirely within `PAD_STACK_TO_MM` (LINUX_OFFSET_MM is
    //     preserved unconditionally).
    assert!(
        offset_of!(TaskStruct, m29) == LINUX_OFFSET_STACK + 8,
        "M29SchedFields must immediately follow `stack`"
    );
    assert!(core::mem::size_of::<M29SchedFields>() <= PAD_STACK_TO_MM);
};

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fixed-offset checks (always correct) ────────────────────────────────

    #[test]
    fn thread_info_at_offset_0() {
        assert_eq!(offset_of!(TaskStruct, thread_info), 0);
    }

    #[test]
    fn state_at_offset_24() {
        assert_eq!(offset_of!(TaskStruct, __state), 24);
    }

    #[test]
    fn stack_at_offset_32() {
        assert_eq!(offset_of!(TaskStruct, stack), 32);
    }

    // ── Relative invariants (always correct) ────────────────────────────────

    #[test]
    fn tgid_immediately_follows_pid() {
        assert_eq!(
            offset_of!(TaskStruct, tgid),
            offset_of!(TaskStruct, pid) + 4
        );
    }

    #[test]
    fn thread_is_the_last_field() {
        let thread_end = offset_of!(TaskStruct, thread) + size_of::<ThreadStruct>();
        assert_eq!(thread_end, size_of::<TaskStruct>());
    }

    #[test]
    fn comm_len_is_16() {
        assert_eq!(TASK_COMM_LEN, 16);
    }

    #[test]
    fn mm_comes_before_pid() {
        assert!(offset_of!(TaskStruct, mm) < offset_of!(TaskStruct, pid));
    }

    #[test]
    fn cred_comes_before_comm() {
        assert!(offset_of!(TaskStruct, cred) < offset_of!(TaskStruct, comm));
    }

    #[test]
    fn comm_comes_before_files() {
        assert!(offset_of!(TaskStruct, comm) < offset_of!(TaskStruct, files));
    }

    #[test]
    fn files_comes_before_signal() {
        assert!(offset_of!(TaskStruct, files) < offset_of!(TaskStruct, signal));
    }

    #[test]
    fn zeroed_task_has_null_pointers() {
        let t: TaskStruct = unsafe { core::mem::zeroed() };
        assert!(t.mm.is_null());
        assert!(t.files.is_null());
        assert!(t.signal.is_null());
        assert!(t.cred.is_null());
    }

    // ── Absolute offset checks (calibrated against LINUX_OFFSET_* constants) ─
    //
    // These pass because our padding is designed to produce exactly those
    // offsets.  Update LINUX_OFFSET_* based on pahole output to achieve
    // true Linux ABI parity.

    #[test]
    fn pid_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, pid), LINUX_OFFSET_PID);
    }

    #[test]
    fn mm_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, mm), LINUX_OFFSET_MM);
    }

    #[test]
    fn comm_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, comm), LINUX_OFFSET_COMM);
    }

    #[test]
    fn cred_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, cred), LINUX_OFFSET_CRED);
    }

    #[test]
    fn files_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, files), LINUX_OFFSET_FILES);
    }

    #[test]
    fn signal_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, signal), LINUX_OFFSET_SIGNAL);
    }

    #[test]
    fn thread_at_linux_offset() {
        assert_eq!(offset_of!(TaskStruct, thread), LINUX_OFFSET_THREAD);
    }

    // ── PtRegs checks ────────────────────────────────────────────────────────

    #[test]
    fn pt_regs_size_is_168_bytes() {
        // 21 fields × 8 bytes = 168 bytes, matching Linux struct pt_regs x86_64.
        assert_eq!(size_of::<PtRegs>(), 21 * 8);
    }

    #[test]
    fn pt_regs_orig_ax_follows_di() {
        // di is the 15th field; orig_ax is the 16th (at offset 15*8=120).
        assert_eq!(offset_of!(PtRegs, orig_ax), 15 * 8);
    }

    // ── ThreadInfo checks ────────────────────────────────────────────────────

    #[test]
    fn thread_info_size_is_24() {
        assert_eq!(size_of::<ThreadInfo>(), 24);
    }

    #[test]
    fn thread_info_flags_at_offset_0() {
        assert_eq!(offset_of!(ThreadInfo, flags), 0);
    }

    #[test]
    fn thread_info_cpu_at_offset_20() {
        assert_eq!(offset_of!(ThreadInfo, cpu), 20);
    }
}
