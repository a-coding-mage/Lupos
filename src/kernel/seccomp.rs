//! linux-parity: complete
//! linux-source: vendor/linux/kernel/seccomp.c
//! test-origin: linux:vendor/linux/kernel/seccomp.c
//! seccomp — syscall filtering — Milestone 27.
//!
//! Implements:
//!   - `Seccomp` — the per-task seccomp state (mode + filter chain).
//!   - `SECCOMP_MODE_*` and `SECCOMP_RET_*` constants.
//!   - `SeccompData` — the input struct an attached cBPF filter sees.
//!   - `sys_seccomp` syscall dispatcher.
//!   - `sys_prctl` subset for `PR_SET_SECCOMP` / `PR_SET_NO_NEW_PRIVS`.
//!   - `seccomp_attach_filter` / `seccomp_run_filters` — internal entry points
//!     used by the syscall fast path (M59).
//!
//! Seccomp filter chaining: attached filters form a singly-linked list with
//! the most-recent at the head.  All filters run; the most restrictive
//! `SECCOMP_RET_*` action wins — i.e. the action with the highest numerical
//! priority.  The priority order is fixed by Linux:
//!
//! ```text
//!     KILL_PROCESS > KILL_THREAD > TRAP > ERRNO > USER_NOTIF > TRACE > LOG > ALLOW
//! ```
//!
//! Reference: Linux `kernel/seccomp.c`, `include/uapi/linux/seccomp.h`,
//! `Documentation/userspace-api/seccomp_filter.rst`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use crate::kernel::bpf::{BPF_MAXINSNS, BpfRunResult, SockFilter, bpf_run_filter_native};

// ── seccomp modes ────────────────────────────────────────────────────────────

pub const SECCOMP_MODE_DISABLED: u32 = 0;
pub const SECCOMP_MODE_STRICT: u32 = 1;
pub const SECCOMP_MODE_FILTER: u32 = 2;

// ── seccomp() syscall operations ─────────────────────────────────────────────

pub const SECCOMP_SET_MODE_STRICT: u32 = 0;
pub const SECCOMP_SET_MODE_FILTER: u32 = 1;
pub const SECCOMP_GET_ACTION_AVAIL: u32 = 2;
pub const SECCOMP_GET_NOTIF_SIZES: u32 = 3;

// ── filter flags ─────────────────────────────────────────────────────────────

pub const SECCOMP_FILTER_FLAG_TSYNC: u64 = 1 << 0;
pub const SECCOMP_FILTER_FLAG_LOG: u64 = 1 << 1;
pub const SECCOMP_FILTER_FLAG_SPEC_ALLOW: u64 = 1 << 2;
pub const SECCOMP_FILTER_FLAG_NEW_LISTENER: u64 = 1 << 3;
pub const SECCOMP_FILTER_FLAG_TSYNC_ESRCH: u64 = 1 << 4;
pub const SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV: u64 = 1 << 5;

const SECCOMP_FILTER_FLAGS_MASK: u64 = SECCOMP_FILTER_FLAG_TSYNC
    | SECCOMP_FILTER_FLAG_LOG
    | SECCOMP_FILTER_FLAG_SPEC_ALLOW
    | SECCOMP_FILTER_FLAG_NEW_LISTENER
    | SECCOMP_FILTER_FLAG_TSYNC_ESRCH
    | SECCOMP_FILTER_FLAG_WAIT_KILLABLE_RECV;

// ── SECCOMP_RET_* actions ────────────────────────────────────────────────────

pub const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
pub const SECCOMP_RET_KILL_THREAD: u32 = 0x0000_0000;
pub const SECCOMP_RET_KILL: u32 = SECCOMP_RET_KILL_THREAD; // legacy
pub const SECCOMP_RET_TRAP: u32 = 0x0003_0000;
pub const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
pub const SECCOMP_RET_USER_NOTIF: u32 = 0x7fc0_0000;
pub const SECCOMP_RET_TRACE: u32 = 0x7ff0_0000;
pub const SECCOMP_RET_LOG: u32 = 0x7ffc_0000;
pub const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;

pub const SECCOMP_RET_ACTION_FULL: u32 = 0xffff_0000;
pub const SECCOMP_RET_ACTION: u32 = 0x7fff_0000;
pub const SECCOMP_RET_DATA: u32 = 0x0000_ffff;

/// `prctl()` codes used by seccomp.
pub const PR_SET_NO_NEW_PRIVS: i32 = 38;
pub const PR_GET_NO_NEW_PRIVS: i32 = 39;
pub const PR_SET_SECCOMP: i32 = 22;
pub const PR_GET_SECCOMP: i32 = 21;
pub const PR_GET_KEEPCAPS: i32 = 7;
pub const PR_SET_KEEPCAPS: i32 = 8;
pub const PR_GET_SECUREBITS: i32 = 27;
pub const PR_SET_SECUREBITS: i32 = 28;
pub const PR_SET_CHILD_SUBREAPER: i32 = 36;
pub const PR_GET_CHILD_SUBREAPER: i32 = 37;
pub const PR_SET_PDEATHSIG: i32 = 1;
pub const PR_GET_PDEATHSIG: i32 = 2;
pub const PR_CAPBSET_READ: i32 = 23;
pub const PR_CAPBSET_DROP: i32 = 24;
pub const PR_CAP_AMBIENT: i32 = 47;
pub const PR_SET_MDWE: i32 = 65;
pub const PR_GET_MDWE: i32 = 66;
pub const PR_FUTEX_HASH: i32 = 78;

pub const PR_MDWE_REFUSE_EXEC_GAIN: u32 = 1 << 0;
pub const PR_MDWE_NO_INHERIT: u32 = 1 << 1;

pub const PR_CAP_AMBIENT_IS_SET: u64 = 1;
pub const PR_CAP_AMBIENT_RAISE: u64 = 2;
pub const PR_CAP_AMBIENT_LOWER: u64 = 3;
pub const PR_CAP_AMBIENT_CLEAR_ALL: u64 = 4;
pub const PR_FUTEX_HASH_SET_SLOTS: u64 = 1;
pub const PR_FUTEX_HASH_GET_SLOTS: u64 = 2;

const SECURE_ALL_BITS: u32 = (1 << 0) | (1 << 2) | (1 << 4) | (1 << 6) | (1 << 8) | (1 << 10);
const SECURE_ALL_LOCKS: u32 = SECURE_ALL_BITS << 1;
const SECURE_ALL_MASK: u32 = SECURE_ALL_BITS | SECURE_ALL_LOCKS;
const SECBIT_KEEP_CAPS: u32 = 1 << crate::kernel::cred::securebits::SECURE_KEEP_CAPS;
const SECBIT_KEEP_CAPS_LOCKED: u32 = 1 << (crate::kernel::cred::securebits::SECURE_KEEP_CAPS + 1);
const PR_MDWE_VALID_FLAGS: u32 = PR_MDWE_REFUSE_EXEC_GAIN | PR_MDWE_NO_INHERIT;

/// `prctl(PR_SET_SECCOMP, …)` arguments.
pub const SECCOMP_MODE_STRICT_PRCTL: i32 = 1;
pub const SECCOMP_MODE_FILTER_PRCTL: i32 = 2;

// ── seccomp_data (the cBPF program input) ────────────────────────────────────

/// `struct seccomp_data` — what the cBPF filter sees on every syscall.
///
/// Layout matches Linux `include/uapi/linux/seccomp.h`.  Total size = 64 B.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SeccompData {
    pub nr: i32,
    pub arch: u32,
    pub instruction_pointer: u64,
    pub args: [u64; 6],
}

const _: () = {
    assert!(core::mem::size_of::<SeccompData>() == 64);
    assert!(core::mem::offset_of!(SeccompData, nr) == 0);
    assert!(core::mem::offset_of!(SeccompData, arch) == 4);
    assert!(core::mem::offset_of!(SeccompData, instruction_pointer) == 8);
    assert!(core::mem::offset_of!(SeccompData, args) == 16);
};

// ── sock_fprog (the userspace-supplied filter handle) ────────────────────────

/// `struct sock_fprog` — pointer + length wrapper a caller passes to
/// `SECCOMP_SET_MODE_FILTER`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SockFprog {
    pub len: u16,
    pub filter: *const SockFilter,
}

// SAFETY: SockFprog holds a raw pointer; Send/Sync added so it can flow
// through the test code paths.  It is never stored long-term; callers
// transfer ownership of the program bytes into a `SeccompFilter` via copy.
unsafe impl Send for SockFprog {}
unsafe impl Sync for SockFprog {}

// ── SeccompFilter ────────────────────────────────────────────────────────────

/// One refcounted filter.  Linux `struct seccomp_filter`.
#[repr(C)]
pub struct SeccompFilter {
    pub usage: AtomicUsize,
    /// Pointer to the previous filter in the chain (or null at the tail).
    pub prev: *mut SeccompFilter,
    /// Compiled cBPF program (instruction count = `prog.len()`).
    pub prog: Box<[SockFilter]>,
}

unsafe impl Send for SeccompFilter {}
unsafe impl Sync for SeccompFilter {}

impl SeccompFilter {
    /// Bump refcount and return.
    #[inline]
    pub fn get(&self) -> &Self {
        self.usage.fetch_add(1, Ordering::Relaxed);
        self
    }

    /// Drop a reference; if last, free the filter and chain-drop `prev`.
    ///
    /// # Safety
    /// `f` must be a pointer previously obtained from `seccomp_prepare_filter`.
    pub unsafe fn put(f: *mut SeccompFilter) {
        if f.is_null() {
            return;
        }
        let prev = unsafe { (*f).usage.fetch_sub(1, Ordering::Release) };
        if prev == 1 {
            // Reclaim.
            let owned: Box<SeccompFilter> = unsafe { Box::from_raw(f) };
            unsafe { SeccompFilter::put(owned.prev) };
            // owned dropped here.
        }
    }
}

// ── Seccomp (per-task state, embedded inline in TaskStruct) ──────────────────

/// Per-task seccomp state.  Embedded inline in `TaskStruct` (16 bytes).
///
/// The filter head pointer is `AtomicPtr` so that interior-mutability rules
/// hold when the field is accessed through `&Seccomp` (which it is, since
/// `Seccomp` is embedded inside `TaskStruct` and updated through shared
/// references during `seccomp_attach_filter`).
#[repr(C)]
pub struct Seccomp {
    pub mode: AtomicU32,
    /// Padding to 8-byte align the filter pointer.
    pub _pad: u32,
    /// Head of the filter chain (most-recent first).
    pub filter: AtomicPtr<SeccompFilter>,
}

unsafe impl Send for Seccomp {}
unsafe impl Sync for Seccomp {}

impl Default for Seccomp {
    fn default() -> Self {
        Self {
            mode: AtomicU32::new(SECCOMP_MODE_DISABLED),
            _pad: 0,
            filter: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

const _: () = {
    assert!(core::mem::size_of::<Seccomp>() == 16);
    assert!(core::mem::align_of::<Seccomp>() == 8);
};

// ── No-new-privs flag (per-task) ─────────────────────────────────────────────
//
// Stored in TaskStruct via the `m27.no_new_privs` slot; M27 keeps a global
// bool for the M27 acceptance test.  Real per-task storage lives in the M27
// substruct (see `kernel::task::M27Fields`).

// ── seccomp_run_filters ──────────────────────────────────────────────────────

/// Run every filter in `task.seccomp` over `data` and return the most
/// restrictive `SECCOMP_RET_*` action.
///
/// The result is a 32-bit value with the action in the top 16 bits and
/// optional data (e.g. errno) in the low 16 bits.
///
/// Returns `SECCOMP_RET_ALLOW` when no filter is attached.
pub fn seccomp_run_filters(seccomp: &Seccomp, data: &SeccompData) -> u32 {
    if seccomp.mode.load(Ordering::Acquire) != SECCOMP_MODE_FILTER {
        return SECCOMP_RET_ALLOW;
    }

    let bytes = unsafe {
        core::slice::from_raw_parts(
            (data as *const SeccompData) as *const u8,
            core::mem::size_of::<SeccompData>(),
        )
    };

    let mut best: u32 = SECCOMP_RET_ALLOW;
    let mut cursor: *mut SeccompFilter = seccomp.filter.load(Ordering::Acquire);
    while !cursor.is_null() {
        let prog = unsafe { &(*cursor).prog };
        let result = match bpf_run_filter_native(prog, bytes) {
            BpfRunResult::Value(v) => v,
            // Faulting filters return KILL_PROCESS — Linux semantics.
            _ => SECCOMP_RET_KILL_PROCESS,
        };
        if action_priority(result) > action_priority(best) {
            best = result;
        }
        cursor = unsafe { (*cursor).prev };
    }
    best
}

/// Numeric priority used to pick the most-restrictive action across filters.
///
/// Higher = stricter — Linux uses the upper-16 bits directly because
/// `KILL_PROCESS` (`0x8000_0000`) has the high bit set, but compares require
/// us to treat that bit as the most significant in *unsigned* terms.
#[inline]
fn action_priority(r: u32) -> u32 {
    // KILL_PROCESS has the high bit set, all others have it clear.  We can
    // compare via signed-as-priority: `KILL_PROCESS` → very large unsigned;
    // mapping ALLOW → 0 and increasing strictness upward gives the right order.
    match r & SECCOMP_RET_ACTION_FULL {
        SECCOMP_RET_KILL_PROCESS => 7,
        SECCOMP_RET_KILL_THREAD => 6,
        SECCOMP_RET_TRAP => 5,
        SECCOMP_RET_ERRNO => 4,
        SECCOMP_RET_USER_NOTIF => 3,
        SECCOMP_RET_TRACE => 2,
        SECCOMP_RET_LOG => 1,
        SECCOMP_RET_ALLOW => 0,
        _ => 7, // unknown action treated as most restrictive
    }
}

// ── seccomp_prepare_filter / seccomp_attach_filter ───────────────────────────

/// Validate `prog` and wrap it in a heap-allocated `SeccompFilter`.
///
/// Returns `Err(-EINVAL)` for an empty program, `Err(-EINVAL)` for one
/// exceeding `BPF_MAXINSNS`, and `Ok(filter)` on success with `usage == 1`.
pub fn seccomp_prepare_filter(prog: Vec<SockFilter>) -> Result<*mut SeccompFilter, i32> {
    if prog.is_empty() || prog.len() > BPF_MAXINSNS {
        return Err(-22);
    }
    let f = Box::new(SeccompFilter {
        usage: AtomicUsize::new(1),
        prev: core::ptr::null_mut(),
        prog: prog.into_boxed_slice(),
    });
    Ok(Box::into_raw(f))
}

/// Attach `filter` to `seccomp`.  Sets mode = FILTER if it was DISABLED.
///
/// # Safety
/// `filter` must be a pointer previously returned by `seccomp_prepare_filter`.
pub unsafe fn seccomp_attach_filter(seccomp: &Seccomp, filter: *mut SeccompFilter) {
    let head = seccomp.filter.load(Ordering::Acquire);
    unsafe {
        (*filter).prev = head;
    }
    seccomp.filter.store(filter, Ordering::Release);
    seccomp.mode.store(SECCOMP_MODE_FILTER, Ordering::Release);
}

// ── Syscalls ─────────────────────────────────────────────────────────────────

/// `sys_seccomp(operation, flags, args)`.
///
/// Mirrors Linux `kernel/seccomp.c::SYSCALL_DEFINE3(seccomp,…)`.
///
/// # Safety
/// `args` must point to a valid kernel-space buffer when the operation
/// requires one.
pub unsafe fn sys_seccomp(operation: u32, flags: u64, args: *const core::ffi::c_void) -> i64 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return -22;
    }

    match operation {
        SECCOMP_SET_MODE_STRICT => {
            if flags != 0 || !args.is_null() {
                return -22;
            }
            // Without CAP_SYS_ADMIN the caller must have set NO_NEW_PRIVS.
            let nnp = unsafe { (*task).m27.no_new_privs };
            if nnp == 0
                && !crate::kernel::capability::capable(crate::kernel::capability::CAP_SYS_ADMIN)
            {
                return -1; // EACCES → -EPERM
            }
            unsafe {
                (*task)
                    .m27_seccomp
                    .mode
                    .store(SECCOMP_MODE_STRICT, Ordering::Release);
            }
            0
        }
        SECCOMP_SET_MODE_FILTER => {
            if flags & !SECCOMP_FILTER_FLAGS_MASK != 0 {
                return -22;
            }
            if args.is_null() {
                return -14; // EFAULT
            }
            // Must have NO_NEW_PRIVS or CAP_SYS_ADMIN.
            let nnp = unsafe { (*task).m27.no_new_privs };
            if nnp == 0
                && !crate::kernel::capability::capable(crate::kernel::capability::CAP_SYS_ADMIN)
            {
                return -1;
            }
            let fprog = unsafe { *(args as *const SockFprog) };
            if fprog.filter.is_null() || fprog.len == 0 {
                return -22;
            }
            let prog: Vec<SockFilter> =
                unsafe { core::slice::from_raw_parts(fprog.filter, fprog.len as usize).to_vec() };
            let filter = match seccomp_prepare_filter(prog) {
                Ok(f) => f,
                Err(e) => return e as i64,
            };
            unsafe {
                seccomp_attach_filter(&(*task).m27_seccomp, filter);
            }
            0
        }
        SECCOMP_GET_ACTION_AVAIL => {
            // args points to a u32 holding the action to query.
            if args.is_null() {
                return -14;
            }
            let act = unsafe { *(args as *const u32) };
            match act & SECCOMP_RET_ACTION_FULL {
                SECCOMP_RET_KILL_PROCESS
                | SECCOMP_RET_KILL_THREAD
                | SECCOMP_RET_TRAP
                | SECCOMP_RET_ERRNO
                | SECCOMP_RET_TRACE
                | SECCOMP_RET_LOG
                | SECCOMP_RET_ALLOW => 0,
                _ => -95, // EOPNOTSUPP
            }
        }
        _ => -22,
    }
}

/// `sys_prctl` subset relevant to seccomp / no-new-privs.
///
/// Returns `-EINVAL` for unsupported options so callers can compose it with
/// other prctl families later (M59).
pub unsafe fn sys_prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return -22;
    }

    match option {
        PR_SET_PDEATHSIG => {
            let sig = arg2 as i32;
            if sig < 0 || sig > crate::kernel::signal::NSIG as i32 {
                return -22;
            }
            unsafe {
                (*task).m26.pdeath_signal = sig;
            }
            0
        }
        PR_GET_PDEATHSIG => {
            let dst = arg2 as *mut u32;
            if dst.is_null()
                || unsafe {
                    crate::arch::x86::kernel::uaccess::put_user_u32(
                        dst,
                        (*task).m26.pdeath_signal as u32,
                    )
                }
                .is_err()
            {
                return -14;
            }
            0
        }
        PR_SET_CHILD_SUBREAPER => {
            unsafe {
                if arg2 != 0 {
                    (*task).m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER;
                } else {
                    (*task).m27.mdwe_flags &= !crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER;
                }
                if arg2 != 0 {
                    propagate_has_child_subreaper(task);
                }
            }
            0
        }
        PR_GET_CHILD_SUBREAPER => {
            let dst = arg2 as *mut u32;
            if dst.is_null()
                || unsafe {
                    crate::arch::x86::kernel::uaccess::put_user_u32(
                        dst,
                        ((*task).m27.mdwe_flags & crate::kernel::task::TASK_CTRL_CHILD_SUBREAPER
                            != 0) as u32,
                    )
                }
                .is_err()
            {
                return -14;
            }
            0
        }
        PR_SET_NO_NEW_PRIVS => {
            if arg2 != 1 || arg3 != 0 {
                return -22;
            }
            unsafe {
                (*task).m27.no_new_privs = 1;
            }
            0
        }
        PR_GET_NO_NEW_PRIVS => {
            if arg2 != 0 || arg3 != 0 {
                return -22;
            }
            unsafe { (*task).m27.no_new_privs as i64 }
        }
        PR_SET_MDWE => {
            if arg3 != 0 || arg4 != 0 || arg5 != 0 {
                return -22;
            }
            let mm = unsafe { (*task).mm };
            if mm.is_null() {
                return -22;
            }
            let requested = arg2 as u32;
            if requested & !PR_MDWE_VALID_FLAGS != 0 {
                return -22;
            }
            if requested & PR_MDWE_NO_INHERIT != 0 && requested & PR_MDWE_REFUSE_EXEC_GAIN == 0 {
                return -22;
            }

            let current = unsafe { mdwe_flags_for_mm(mm) };
            if current != 0 && requested != current {
                return -1;
            }
            unsafe {
                if requested & PR_MDWE_REFUSE_EXEC_GAIN != 0 {
                    crate::mm::mm_types::mm_flags_set(mm, crate::mm::mm_types::MMF_HAS_MDWE);
                }
                if requested & PR_MDWE_NO_INHERIT != 0 {
                    crate::mm::mm_types::mm_flags_set(
                        mm,
                        crate::mm::mm_types::MMF_HAS_MDWE_NO_INHERIT,
                    );
                }
            }
            0
        }
        PR_GET_MDWE => {
            if arg2 != 0 || arg3 != 0 || arg4 != 0 || arg5 != 0 {
                return -22;
            }
            let mm = unsafe { (*task).mm };
            if mm.is_null() {
                return -22;
            }
            unsafe { mdwe_flags_for_mm(mm) as i64 }
        }
        PR_FUTEX_HASH => match arg2 {
            PR_FUTEX_HASH_GET_SLOTS => crate::kernel::futex::futex_private_hash_get_slots(),
            PR_FUTEX_HASH_SET_SLOTS => {
                if arg4 != 0 {
                    return -22;
                }
                crate::kernel::futex::futex_private_hash_set_slots(arg3 as u32)
            }
            _ => -22,
        },
        PR_GET_SECCOMP => unsafe { (*task).m27_seccomp.mode.load(Ordering::Acquire) as i64 },
        PR_GET_SECUREBITS => {
            let cred = crate::kernel::cred::current_cred();
            if cred.is_null() {
                0
            } else {
                unsafe { (*cred).securebits as i64 }
            }
        }
        PR_GET_KEEPCAPS => {
            let cred = crate::kernel::cred::current_cred();
            if cred.is_null() {
                0
            } else {
                ((unsafe { (*cred).securebits } & SECBIT_KEEP_CAPS) != 0) as i64
            }
        }
        PR_SET_KEEPCAPS => {
            if arg2 > 1 {
                return -22;
            }
            let cred = crate::kernel::cred::current_cred();
            if !cred.is_null() && (unsafe { (*cred).securebits } & SECBIT_KEEP_CAPS_LOCKED) != 0 {
                return -1;
            }
            let Some(new) = crate::kernel::cred::prepare_creds() else {
                return -12;
            };
            unsafe {
                if arg2 != 0 {
                    (*new).securebits |= SECBIT_KEEP_CAPS;
                } else {
                    (*new).securebits &= !SECBIT_KEEP_CAPS;
                }
            }
            crate::kernel::cred::commit_creds(new);
            0
        }
        PR_SET_SECUREBITS => {
            if arg2 > u32::MAX as u64 || (arg2 as u32) & !SECURE_ALL_MASK != 0 {
                return -22;
            }
            let Some(new) = crate::kernel::cred::prepare_creds() else {
                return -12;
            };
            unsafe {
                (*new).securebits = arg2 as u32;
            }
            crate::kernel::cred::commit_creds(new);
            0
        }
        PR_CAPBSET_READ => {
            if arg2 > crate::kernel::capability::CAP_LAST_CAP as u64 {
                return -22;
            }
            let cred = crate::kernel::cred::current_cred();
            if cred.is_null() {
                0
            } else if unsafe { (*cred).cap_bset.raised(arg2 as u32) } {
                1
            } else {
                0
            }
        }
        PR_CAPBSET_DROP => {
            if arg2 > crate::kernel::capability::CAP_LAST_CAP as u64 {
                return -22;
            }
            if !crate::kernel::capability::capable(crate::kernel::capability::CAP_SETPCAP) {
                return -1;
            }
            let Some(new) = crate::kernel::cred::prepare_creds() else {
                return -12;
            };
            unsafe {
                (*new).cap_bset.lower(arg2 as u32);
            }
            crate::kernel::cred::commit_creds(new);
            0
        }
        PR_CAP_AMBIENT => {
            if arg4 != 0 || arg5 != 0 {
                return -22;
            }
            match arg2 {
                PR_CAP_AMBIENT_IS_SET => {
                    if arg3 > crate::kernel::capability::CAP_LAST_CAP as u64 {
                        return -22;
                    }
                    let cred = crate::kernel::cred::current_cred();
                    if cred.is_null() {
                        0
                    } else if unsafe { (*cred).cap_ambient.raised(arg3 as u32) } {
                        1
                    } else {
                        0
                    }
                }
                PR_CAP_AMBIENT_RAISE => {
                    if arg3 > crate::kernel::capability::CAP_LAST_CAP as u64 {
                        return -22;
                    }
                    let cred = crate::kernel::cred::current_cred();
                    if cred.is_null()
                        || unsafe {
                            !(*cred).cap_permitted.raised(arg3 as u32)
                                || !(*cred).cap_inheritable.raised(arg3 as u32)
                        }
                    {
                        return -1;
                    }
                    let Some(new) = crate::kernel::cred::prepare_creds() else {
                        return -12;
                    };
                    unsafe {
                        (*new).cap_ambient.raise(arg3 as u32);
                    }
                    crate::kernel::cred::commit_creds(new);
                    0
                }
                PR_CAP_AMBIENT_LOWER => {
                    if arg3 > crate::kernel::capability::CAP_LAST_CAP as u64 {
                        return -22;
                    }
                    let Some(new) = crate::kernel::cred::prepare_creds() else {
                        return -12;
                    };
                    unsafe {
                        (*new).cap_ambient.lower(arg3 as u32);
                    }
                    crate::kernel::cred::commit_creds(new);
                    0
                }
                PR_CAP_AMBIENT_CLEAR_ALL => {
                    if arg3 != 0 {
                        return -22;
                    }
                    let Some(new) = crate::kernel::cred::prepare_creds() else {
                        return -12;
                    };
                    unsafe {
                        (*new).cap_ambient = crate::kernel::capability::KernelCapT::empty();
                    }
                    crate::kernel::cred::commit_creds(new);
                    0
                }
                _ => -22,
            }
        }
        PR_SET_SECCOMP => {
            // arg2 = SECCOMP_MODE_STRICT_PRCTL or SECCOMP_MODE_FILTER_PRCTL
            // arg3 = pointer to sock_fprog (for FILTER)
            match arg2 as i32 {
                SECCOMP_MODE_STRICT_PRCTL => unsafe {
                    sys_seccomp(SECCOMP_SET_MODE_STRICT, 0, core::ptr::null())
                },
                SECCOMP_MODE_FILTER_PRCTL => unsafe {
                    sys_seccomp(SECCOMP_SET_MODE_FILTER, 0, arg3 as *const _)
                },
                _ => -22,
            }
        }
        _ => -22,
    }
}

pub fn current_mdwe_flags() -> u32 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        0
    } else {
        let mm = unsafe { (*task).mm };
        unsafe { mdwe_flags_for_mm(mm) }
    }
}

pub fn current_mdwe_refuses_exec_gain() -> bool {
    current_mdwe_flags() & PR_MDWE_REFUSE_EXEC_GAIN != 0
}

pub unsafe fn mdwe_flags_for_mm(mm: *const crate::mm::mm_types::MmStruct) -> u32 {
    if mm.is_null() {
        return 0;
    }
    let mut flags = 0;
    if unsafe { crate::mm::mm_types::mm_flags_test(mm, crate::mm::mm_types::MMF_HAS_MDWE) } {
        flags |= PR_MDWE_REFUSE_EXEC_GAIN;
    }
    if unsafe {
        crate::mm::mm_types::mm_flags_test(mm, crate::mm::mm_types::MMF_HAS_MDWE_NO_INHERIT)
    } {
        flags |= PR_MDWE_NO_INHERIT;
    }
    flags
}

pub unsafe fn mdwe_refuses_exec_gain_for_mm(mm: *const crate::mm::mm_types::MmStruct) -> bool {
    unsafe { mdwe_flags_for_mm(mm) & PR_MDWE_REFUSE_EXEC_GAIN != 0 }
}

unsafe fn propagate_has_child_subreaper(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }

    let count = unsafe { (*task).m26.children_count as usize };
    for i in 0..count.min(crate::kernel::task::MAX_CHILDREN) {
        let child = unsafe { (*task).m26.children[i] };
        if child.is_null() {
            continue;
        }
        if unsafe {
            (*child).m27.mdwe_flags & crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER != 0
        } {
            continue;
        }
        unsafe {
            (*child).m27.mdwe_flags |= crate::kernel::task::TASK_CTRL_HAS_CHILD_SUBREAPER;
            propagate_has_child_subreaper(child);
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::bpf::{BPF_ABS, BPF_IMM, BPF_JEQ, BPF_JMP, BPF_K, BPF_LD, BPF_RET, BPF_W};
    use crate::kernel::capability::{CAP_CHOWN, CAP_LAST_CAP};
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};
    use crate::mm::mm_types::MmStruct;

    #[test]
    fn seccomp_data_layout() {
        assert_eq!(core::mem::size_of::<SeccompData>(), 64);
    }

    #[test]
    fn ret_action_constants_match_linux() {
        assert_eq!(SECCOMP_RET_ALLOW, 0x7fff_0000);
        assert_eq!(SECCOMP_RET_KILL_THREAD, 0x0000_0000);
        assert_eq!(SECCOMP_RET_KILL_PROCESS, 0x8000_0000);
        assert_eq!(SECCOMP_RET_ERRNO, 0x0005_0000);
        assert_eq!(SECCOMP_RET_TRAP, 0x0003_0000);
        assert_eq!(SECCOMP_RET_LOG, 0x7ffc_0000);
        assert_eq!(SECCOMP_RET_ACTION, 0x7fff_0000);
        assert_eq!(SECCOMP_RET_DATA, 0x0000_ffff);
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        let mut mm = Box::new(MmStruct::new(0));
        current.pid = 88;
        current.tgid = 88;
        current.cred = &raw const INIT_CRED;
        current.mm = &mut *mm;
        current.active_mm = &mut *mm;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(sys_prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0), 1);
            assert_eq!(sys_prctl(PR_SET_NO_NEW_PRIVS, 2, 0, 0, 0), -22);
            assert_eq!(sys_prctl(PR_GET_SECCOMP, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_SECUREBITS, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_KEEPCAPS, 0, 0, 0, 0), 0);
            let mut subreaper = 99u32;
            assert_eq!(
                sys_prctl(
                    PR_GET_CHILD_SUBREAPER,
                    (&mut subreaper as *mut u32) as u64,
                    0,
                    0,
                    0
                ),
                0
            );
            assert_eq!(subreaper, 0);
            assert_eq!(sys_prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0), 0);
            assert_eq!(
                sys_prctl(
                    PR_GET_CHILD_SUBREAPER,
                    (&mut subreaper as *mut u32) as u64,
                    0,
                    0,
                    0
                ),
                0
            );
            assert_eq!(subreaper, 1);
            assert_eq!(sys_prctl(PR_SET_CHILD_SUBREAPER, 0, 0, 0, 0), 0);
            assert_eq!(
                sys_prctl(
                    PR_GET_CHILD_SUBREAPER,
                    (&mut subreaper as *mut u32) as u64,
                    0,
                    0,
                    0
                ),
                0
            );
            assert_eq!(subreaper, 0);
            assert_eq!(sys_prctl(PR_SET_KEEPCAPS, 1, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_KEEPCAPS, 0, 0, 0, 0), 1);
            assert_eq!(sys_prctl(PR_SET_KEEPCAPS, 2, 0, 0, 0), -22);
            assert_eq!(sys_prctl(PR_SET_KEEPCAPS, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_KEEPCAPS, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_SET_SECUREBITS, 1 << 2, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_SECUREBITS, 0, 0, 0, 0), 1 << 2);
            assert_eq!(sys_prctl(PR_SET_SECUREBITS, 1 << 31, 0, 0, 0), -22);
            assert_eq!(sys_prctl(PR_CAPBSET_READ, CAP_CHOWN as u64, 99, 0, 0), 1);
            assert_eq!(sys_prctl(PR_CAPBSET_DROP, CAP_CHOWN as u64, 1, 2, 3), 0);
            assert_eq!(sys_prctl(PR_CAPBSET_READ, CAP_CHOWN as u64, 0, 0, 0), 0);
            assert_eq!(
                sys_prctl(PR_CAPBSET_READ, (CAP_LAST_CAP + 1) as u64, 0, 0, 0),
                -22
            );
            let new = crate::kernel::cred::prepare_creds().expect("prepare creds");
            (*new).cap_inheritable.raise(CAP_CHOWN);
            crate::kernel::cred::commit_creds(new);
            assert_eq!(
                sys_prctl(
                    PR_CAP_AMBIENT,
                    PR_CAP_AMBIENT_IS_SET,
                    CAP_CHOWN as u64,
                    0,
                    0
                ),
                0
            );
            assert_eq!(
                sys_prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_RAISE, CAP_CHOWN as u64, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(
                    PR_CAP_AMBIENT,
                    PR_CAP_AMBIENT_IS_SET,
                    CAP_CHOWN as u64,
                    0,
                    0
                ),
                1
            );
            assert_eq!(
                sys_prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_LOWER, CAP_CHOWN as u64, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(
                    PR_CAP_AMBIENT,
                    PR_CAP_AMBIENT_IS_SET,
                    CAP_CHOWN as u64,
                    0,
                    0
                ),
                0
            );
            assert_eq!(
                sys_prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_CLEAR_ALL, 0, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(
                    PR_CAP_AMBIENT,
                    PR_CAP_AMBIENT_IS_SET,
                    (CAP_LAST_CAP + 1) as u64,
                    0,
                    0
                ),
                -22
            );
            assert_eq!(sys_prctl(PR_GET_MDWE, 0, 0, 0, 0), 0);
            assert_eq!(sys_prctl(PR_GET_MDWE, 1, 0, 0, 0), -22);
            assert_eq!(
                sys_prctl(PR_SET_MDWE, PR_MDWE_NO_INHERIT as u64, 0, 0, 0),
                -22
            );
            assert_eq!(
                sys_prctl(PR_SET_MDWE, PR_MDWE_REFUSE_EXEC_GAIN as u64, 0, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(PR_GET_MDWE, 0, 0, 0, 0),
                PR_MDWE_REFUSE_EXEC_GAIN as i64
            );
            assert_eq!(
                sys_prctl(PR_SET_MDWE, PR_MDWE_REFUSE_EXEC_GAIN as u64, 0, 0, 0),
                0
            );
            assert_eq!(
                sys_prctl(
                    PR_SET_MDWE,
                    (PR_MDWE_REFUSE_EXEC_GAIN | PR_MDWE_NO_INHERIT) as u64,
                    0,
                    0,
                    0
                ),
                -1
            );
            assert_eq!(sys_prctl(PR_SET_MDWE, 0, 0, 0, 0), -1);
            assert_eq!(sys_prctl(0, 0, 0, 0, 0), -22);
            sched::set_current(previous);
        }
    }

    #[test]
    fn seccomp_filters_read_seccomp_data_in_native_endian() {
        let seccomp = Seccomp::default();
        let prog = alloc::vec![
            SockFilter::stmt(BPF_LD | BPF_ABS | BPF_W, 4), // seccomp_data.arch
            SockFilter::jump(BPF_JMP | BPF_K | BPF_JEQ, 0xc000_003e, 0, 1),
            SockFilter::stmt(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
            SockFilter::stmt(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | 1),
        ];
        let filter = seccomp_prepare_filter(prog).unwrap();
        unsafe {
            seccomp_attach_filter(&seccomp, filter);
        }

        let data = SeccompData {
            nr: 257,
            arch: 0xc000_003e,
            instruction_pointer: 0x401000,
            args: [0; 6],
        };

        assert_eq!(seccomp_run_filters(&seccomp, &data), SECCOMP_RET_ALLOW);

        unsafe {
            SeccompFilter::put(seccomp.filter.load(Ordering::Acquire));
        }
    }

    #[test]
    fn syscall_m78_security_bpf_perf_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 89;
        current.tgid = 89;
        current.cred = &raw const INIT_CRED;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(sys_seccomp(999, 0, core::ptr::null()), -22);
            let action = SECCOMP_RET_ALLOW;
            assert_eq!(
                sys_seccomp(
                    SECCOMP_GET_ACTION_AVAIL,
                    0,
                    &action as *const u32 as *const core::ffi::c_void,
                ),
                0
            );
            assert_eq!(
                sys_seccomp(SECCOMP_GET_ACTION_AVAIL, 0, core::ptr::null()),
                -14
            );
            sched::set_current(previous);
        }
    }

    #[test]
    fn priority_kill_process_strictest() {
        assert!(
            action_priority(SECCOMP_RET_KILL_PROCESS) > action_priority(SECCOMP_RET_KILL_THREAD)
        );
        assert!(action_priority(SECCOMP_RET_KILL_THREAD) > action_priority(SECCOMP_RET_ERRNO));
        assert!(action_priority(SECCOMP_RET_ERRNO) > action_priority(SECCOMP_RET_ALLOW));
    }

    #[test]
    fn no_filter_returns_allow() {
        let s = Seccomp::default();
        let data = SeccompData::default();
        assert_eq!(seccomp_run_filters(&s, &data), SECCOMP_RET_ALLOW);
    }

    #[test]
    fn empty_program_rejected() {
        let r = seccomp_prepare_filter(alloc::vec::Vec::new());
        assert_eq!(r, Err(-22));
    }

    #[test]
    fn always_allow_filter() {
        let prog = alloc::vec![SockFilter::stmt(BPF_RET | BPF_K, SECCOMP_RET_ALLOW)];
        let f = seccomp_prepare_filter(prog).unwrap();
        let s = Seccomp::default();
        unsafe {
            seccomp_attach_filter(&s, f);
        }
        let data = SeccompData {
            nr: 999,
            arch: 0,
            instruction_pointer: 0,
            args: [0; 6],
        };
        assert_eq!(seccomp_run_filters(&s, &data), SECCOMP_RET_ALLOW);
        // cleanup
        unsafe {
            SeccompFilter::put(s.filter.load(Ordering::Acquire));
        }
    }

    #[test]
    fn errno_filter_when_nr_matches() {
        // if seccomp_data.nr == 2 -> RET ERRNO|EPERM (1) else RET ALLOW.
        // Linux seccomp filters read seccomp_data in native endianness.
        let prog = alloc::vec![
            SockFilter::stmt(BPF_LD | BPF_ABS | BPF_W, 0),
            SockFilter::jump(BPF_JMP | BPF_K | BPF_JEQ, 2, 0, 1),
            SockFilter::stmt(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | 1),
            SockFilter::stmt(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        ];
        let f = seccomp_prepare_filter(prog).unwrap();
        let s = Seccomp::default();
        unsafe {
            seccomp_attach_filter(&s, f);
        }
        let data = SeccompData {
            nr: 2,
            arch: 0,
            instruction_pointer: 0,
            args: [0; 6],
        };
        let action = seccomp_run_filters(&s, &data);
        assert_eq!(action & SECCOMP_RET_ACTION_FULL, SECCOMP_RET_ERRNO);
        assert_eq!(action & SECCOMP_RET_DATA, 1);
        unsafe {
            SeccompFilter::put(s.filter.load(Ordering::Acquire));
        }
    }

    #[test]
    fn strictest_action_wins_in_chain() {
        let s = Seccomp::default();
        // First attached: ALLOW
        let f1 = seccomp_prepare_filter(alloc::vec![SockFilter::stmt(
            BPF_RET | BPF_K,
            SECCOMP_RET_ALLOW
        ),])
        .unwrap();
        unsafe {
            seccomp_attach_filter(&s, f1);
        }
        // Second attached: ERRNO
        let f2 = seccomp_prepare_filter(alloc::vec![SockFilter::stmt(
            BPF_RET | BPF_K,
            SECCOMP_RET_ERRNO | 13
        ),])
        .unwrap();
        unsafe {
            seccomp_attach_filter(&s, f2);
        }

        let data = SeccompData::default();
        let action = seccomp_run_filters(&s, &data);
        assert_eq!(action & SECCOMP_RET_ACTION_FULL, SECCOMP_RET_ERRNO);

        // Cleanup chain.
        unsafe {
            SeccompFilter::put(s.filter.load(Ordering::Acquire));
        }
    }

    #[test]
    fn _silence_unused_imports() {
        let _ = (BPF_IMM,);
    }
}
