//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! `wait4` / `waitid` / `exit` / `exit_group` syscalls — Milestone 26.
//!
//! Linux `kernel/exit.c` (`kernel_wait4`, `kernel_waitid`, `do_wait`,
//! `wait_consider_task`) plus the `W_EXITCODE` / `WIFEXITED` macros from
//! `include/uapi/linux/wait.h` and glibc `bits/waitstatus.h`.
//!
//! # Scope
//!
//! - `sys_wait4(pid, stat_addr, options, rusage)`
//! - `sys_waitid(which, upid, infop, options, rusage)`
//! - `sys_exit(code)` and `sys_exit_group(code)` — both return `!`.
//! - `w_exitcode(retval, termsig)` packing helper.
//!
//! # Pointer contract
//!
//! `stat_addr`, `infop`, and `rusage` are userspace syscall pointers.  Output
//! is returned with fault-tolerant `copy_to_user` helpers so bad user addresses
//! fail with `-EFAULT` instead of being dereferenced in kernel mode.

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::Ordering;

use crate::arch::x86::kernel::uaccess::copy_to_user;
use crate::kernel::exit::release_task;
use crate::kernel::sched;
use crate::kernel::signal::{SIGCHLD, has_unblocked_pending_signals};
use crate::kernel::task::task_state::{
    __TASK_STOPPED, EXIT_ZOMBIE, TASK_INTERRUPTIBLE, TASK_RUNNING,
};
use crate::kernel::task::{MAX_CHILDREN, MAX_WAITERS, TaskStruct};

// ── Wait flag constants (Linux ABI) ──────────────────────────────────────────

pub const WNOHANG: i32 = 0x0000_0001;
pub const WUNTRACED: i32 = 0x0000_0002;
pub const WSTOPPED: i32 = WUNTRACED;
pub const WEXITED: i32 = 0x0000_0004;
pub const WCONTINUED: i32 = 0x0000_0008;
pub const WNOWAIT: i32 = 0x0100_0000;
pub const __WNOTHREAD: i32 = 0x2000_0000;
pub const __WALL: i32 = 0x4000_0000;
pub const __WCLONE: i32 = 0x8000_0000_u32 as i32;

// `which` values for waitid.
pub const P_ALL: i32 = 0;
pub const P_PID: i32 = 1;
pub const P_PGID: i32 = 2;
pub const P_PIDFD: i32 = 3;

// CLD_* siginfo codes (`include/uapi/asm-generic/siginfo.h`).
pub const CLD_EXITED: i32 = 1;
pub const CLD_KILLED: i32 = 2;
pub const CLD_DUMPED: i32 = 3;
pub const CLD_TRAPPED: i32 = 4;
pub const CLD_STOPPED: i32 = 5;
pub const CLD_CONTINUED: i32 = 6;

const WCOREFLAG: i32 = 0x80;

// errnos used here (negated when returned from a syscall).
const ECHILD: i64 = -10;
const EFAULT: i64 = -14;
const EINVAL: i64 = -22;
const ESRCH: i64 = -3;
// Kernel-internal restart pseudo-errno from vendor/linux/include/linux/errno.h.
const ERESTARTSYS: i64 = -512;

#[inline]
unsafe fn copy_wait_status_to_user(stat_addr: *mut i32, status: i32) -> Result<(), i64> {
    if stat_addr.is_null() {
        return Ok(());
    }
    let not_copied = unsafe {
        copy_to_user(
            stat_addr.cast::<u8>(),
            (&status as *const i32).cast::<u8>(),
            core::mem::size_of::<i32>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

#[inline]
unsafe fn copy_waitid_siginfo_to_user(
    infop: *mut WaitidSigInfo,
    info: &WaitidSigInfo,
) -> Result<(), i64> {
    if infop.is_null() {
        return Ok(());
    }
    let not_copied = unsafe {
        copy_to_user(
            infop.cast::<u8>(),
            (info as *const WaitidSigInfo).cast::<u8>(),
            core::mem::size_of::<WaitidSigInfo>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

/// Pack a Linux wait status as glibc does in `bits/waitstatus.h`:
///   bits 0..6   = termination signal (0 for normal exit)
///   bit  7      = core-dumped flag
///   bits 8..15  = exit status (low 8 bits of the user's exit() argument)
///
/// Normal `exit(N)` produces `((N & 0xff) << 8)`.
#[inline]
pub const fn w_exitcode(retval: i32, termsig: i32) -> i32 {
    ((retval & 0xff) << 8) | (termsig & 0x7f)
}

#[inline]
pub const fn w_ifexited(status: i32) -> bool {
    (status & 0x7f) == 0
}

#[inline]
pub const fn w_exitstatus(status: i32) -> i32 {
    (status >> 8) & 0xff
}

#[inline]
pub const fn w_termsig(status: i32) -> i32 {
    status & 0x7f
}

#[inline]
const fn waitid_code_status(exit_code: i32) -> (i32, i32) {
    // vendor/linux/kernel/exit.c::wait_task_zombie first classifies a zero
    // low-seven-bit term signal as a normal exit, even for malformed packed
    // values carrying WCOREFLAG without a signal number.
    if w_ifexited(exit_code) {
        (CLD_EXITED, exit_code >> 8)
    } else if exit_code & WCOREFLAG != 0 {
        (CLD_DUMPED, w_termsig(exit_code))
    } else {
        (CLD_KILLED, w_termsig(exit_code))
    }
}

#[inline]
pub const fn w_stopped(stopsig: i32) -> i32 {
    ((stopsig & 0xff) << 8) | 0x7f
}

// ── Layout types ─────────────────────────────────────────────────────────────

/// `struct rusage` — placeholder; full layout lives in M27/M59.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rusage {
    pub _placeholder: [u8; 144],
}

impl Default for Rusage {
    fn default() -> Self {
        Self {
            _placeholder: [0u8; 144],
        }
    }
}

/// `siginfo_t` for waitid — only the SIGCHLD union arm is meaningful here.
///
/// Reference: `vendor/linux/include/uapi/asm-generic/siginfo.h` `_sigchld`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaitidSigInfo {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub _pad0: i32,
    pub si_pid: i32,
    pub si_uid: u32,
    pub si_status: i32,
    pub _pad_tail: [u8; 100], // pad up to 128-byte siginfo_t total (28 + 100 = 128)
}

impl Default for WaitidSigInfo {
    fn default() -> Self {
        Self {
            si_signo: 0,
            si_errno: 0,
            si_code: 0,
            _pad0: 0,
            si_pid: 0,
            si_uid: 0,
            si_status: 0,
            _pad_tail: [0u8; 100],
        }
    }
}

// ── PID matching ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum WaitTarget {
    Any,
    Pid(i32),
    CurrentPgrp,
    Pgrp(i32),
}

impl WaitTarget {
    fn trace_value(self) -> i32 {
        match self {
            WaitTarget::Any => -1,
            WaitTarget::Pid(pid) => pid,
            WaitTarget::CurrentPgrp => 0,
            WaitTarget::Pgrp(pgrp) => -pgrp,
        }
    }
}

unsafe fn task_process_group(task: *mut TaskStruct) -> i32 {
    let pid = unsafe { (*task).pid };
    crate::kernel::session::process_group(pid).unwrap_or(pid)
}

/// Linux wait filters mirror `kernel_wait4()`/`kernel_waitid()` in
/// vendor/linux/kernel/exit.c: negative wait4 PIDs and `P_PGID` select process
/// groups, while `0` selects the caller's current process group.
fn pid_matches(parent: *mut TaskStruct, target: WaitTarget, child_pid: i32) -> bool {
    match target {
        WaitTarget::Any => true,
        WaitTarget::Pid(pid) => pid == child_pid,
        WaitTarget::CurrentPgrp => {
            if parent.is_null() {
                return false;
            }
            let parent_pgrp = unsafe { task_process_group(parent) };
            crate::kernel::session::process_group(child_pid).unwrap_or(child_pid) == parent_pgrp
        }
        WaitTarget::Pgrp(pgrp) => {
            crate::kernel::session::process_group(child_pid).unwrap_or(child_pid) == pgrp
        }
    }
}

/// Find a reapable zombie child of `parent` matching `target`; returns the
/// child task pointer on success (enumerated by `real_parent`, so children that
/// overflowed the `m26.children` cache are still found).
unsafe fn find_zombie_child(
    parent: *mut TaskStruct,
    target: WaitTarget,
) -> Option<*mut TaskStruct> {
    let mut found: *mut TaskStruct = core::ptr::null_mut();
    unsafe {
        for_each_real_child(parent, target, |c| {
            if !found.is_null() {
                return;
            }
            let state = (*c).__state.load(Ordering::Acquire);
            if (*c).m26.exit_state & EXIT_ZOMBIE != 0
                && state & EXIT_ZOMBIE != 0
                && !crate::kernel::signal::delay_group_leader(c)
            {
                found = c;
            }
        });
    }
    (!found.is_null()).then_some(found)
}

unsafe fn find_stopped_child(
    parent: *mut TaskStruct,
    target: WaitTarget,
) -> Option<*mut TaskStruct> {
    let mut found: *mut TaskStruct = core::ptr::null_mut();
    unsafe {
        for_each_real_child(parent, target, |c| {
            if !found.is_null() {
                return;
            }
            let state = (*c).__state.load(Ordering::Acquire);
            if state == crate::kernel::task::task_state::__TASK_TRACED
                || (*c).m26.ptrace_stop_signal != 0
            {
                found = c;
            }
        });
    }
    (!found.is_null()).then_some(found)
}

fn lookup_task_by_pid(pid: i32) -> *mut TaskStruct {
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    sched::find_pool_task_by_pid(pid)
}

unsafe fn child_is_ptrace_stopped(child: *mut TaskStruct) -> bool {
    if child.is_null() {
        return false;
    }
    unsafe {
        let state = (*child).__state.load(Ordering::Acquire);
        state == crate::kernel::task::task_state::__TASK_TRACED
            || (*child).m26.ptrace_stop_signal != 0
    }
}

unsafe fn ptrace_task_matches_parent(
    parent: *mut TaskStruct,
    target: WaitTarget,
    task: *mut TaskStruct,
) -> bool {
    if parent.is_null() || task.is_null() {
        return false;
    }
    unsafe { (*task).m26.tracer == parent && pid_matches(parent, target, (*task).pid) }
}

unsafe fn for_each_ptrace_wait_match(
    parent: *mut TaskStruct,
    target: WaitTarget,
    mut f: impl FnMut(*mut TaskStruct),
) {
    match target {
        WaitTarget::Pid(pid) => {
            let task = lookup_task_by_pid(pid);
            if unsafe { ptrace_task_matches_parent(parent, target, task) } {
                f(task);
            }
        }
        _ => {
            let mut visit = |task: *mut TaskStruct| unsafe {
                if ptrace_task_matches_parent(parent, target, task) {
                    f(task);
                }
            };
            crate::kernel::fork::for_each_heap_task(&mut visit);
            sched::for_each_pool_task(&mut visit);
        }
    }
}

/// Visit every child of `parent` matching `target`.
///
/// Uses `real_parent` (the authoritative parent link) via the global task
/// trackers rather than the fixed-size `m26.children` cache array.  The cache
/// only holds `MAX_CHILDREN` (16) entries and silently drops children that
/// overflow it (their `real_parent` still points at us — see the note on
/// `MAX_CHILDREN`).  systemd forks well over 16 generators in parallel on a
/// multi-CPU boot, so a child-enumeration that only scanned the cache made
/// `wait4`/`waitid` return `-ECHILD` even though the (untracked) zombie was
/// still pending — the "Failed to start up manager" multi-CPU boot hang.  This
/// mirrors the global-tracker fallback `release_task` already relies on.
unsafe fn child_in_array(parent: *mut TaskStruct, task: *mut TaskStruct) -> bool {
    unsafe {
        let n = (*parent).m26.children_count as usize;
        for i in 0..n.min(MAX_CHILDREN) {
            if (*parent).m26.children[i] == task {
                return true;
            }
        }
    }
    false
}

unsafe fn for_each_real_child(
    parent: *mut TaskStruct,
    target: WaitTarget,
    mut f: impl FnMut(*mut TaskStruct),
) {
    if parent.is_null() {
        return;
    }
    unsafe {
        // First the fast cache array (also covers unit tests that populate the
        // array directly without registering in the global task trackers).
        let n = (*parent).m26.children_count as usize;
        for i in 0..n.min(MAX_CHILDREN) {
            let c = (*parent).m26.children[i];
            if !c.is_null() && (*c).m26.exit_signal >= 0 && pid_matches(parent, target, (*c).pid) {
                f(c);
            }
        }
    }
    // Then untracked children that overflowed the cache array: scan the global
    // task trackers for tasks whose `real_parent` is us and which are not
    // already in the array.
    match target {
        WaitTarget::Pid(pid) => {
            let task = lookup_task_by_pid(pid);
            if !task.is_null()
                && task != parent
                && unsafe { (*task).m26.real_parent } == parent
                // Linux never links CLONE_THREAD members on the natural
                // parent's children list. Its negative exit_signal is the
                // thread_group_leader() discriminator; ptrace wait traversal
                // remains separate and may still select an individual TID.
                && unsafe { (*task).m26.exit_signal } >= 0
                && !unsafe { child_in_array(parent, task) }
            {
                f(task);
            }
        }
        _ => {
            let mut visit = |task: *mut TaskStruct| unsafe {
                if !task.is_null()
                    && task != parent
                    && (*task).m26.real_parent == parent
                    && (*task).m26.exit_signal >= 0
                    && pid_matches(parent, target, (*task).pid)
                    && !child_in_array(parent, task)
                {
                    f(task);
                }
            };
            crate::kernel::fork::for_each_heap_task(&mut visit);
            sched::for_each_pool_task(&mut visit);
        }
    }
}

unsafe fn find_ptrace_wait_task(
    parent: *mut TaskStruct,
    target: WaitTarget,
) -> Option<*mut TaskStruct> {
    let mut found: *mut TaskStruct = core::ptr::null_mut();
    unsafe {
        for_each_ptrace_wait_match(parent, target, |task| {
            if found.is_null() && should_report_stopped_child(task, WUNTRACED) {
                found = task;
            }
        });
    }
    if found.is_null() { None } else { Some(found) }
}

unsafe fn should_report_stopped_child(child: *mut TaskStruct, options: i32) -> bool {
    if child.is_null() {
        return false;
    }
    unsafe {
        let traced = ((*child).m26.ptrace & crate::kernel::ptrace::PT_PTRACED) != 0;
        child_is_ptrace_stopped(child) && (traced || options & WUNTRACED != 0)
    }
}

unsafe fn has_reportable_wait_event(
    parent: *mut TaskStruct,
    target: WaitTarget,
    options: i32,
) -> bool {
    if unsafe { find_zombie_child(parent, target) }.is_some() {
        return true;
    }
    if let Some(child) = unsafe { find_stopped_child(parent, target) } {
        if unsafe { should_report_stopped_child(child, options) } {
            return true;
        }
    }
    if let Some(task) = unsafe { find_ptrace_wait_task(parent, target) } {
        if unsafe { should_report_stopped_child(task, options) } {
            return true;
        }
    }
    false
}

/// Returns true while a matching child has started zombie publication but is
/// not yet reapable via `find_zombie_child`.  Waiters must not sleep in this
/// half-published state because the exiting child may already have snapshotted
/// its waiter list.
unsafe fn has_exiting_wait_target(parent: *mut TaskStruct, target: WaitTarget) -> bool {
    let mut found = false;
    unsafe {
        for_each_real_child(parent, target, |c| {
            let state = (*c).__state.load(Ordering::Acquire);
            if (*c).m26.exit_state & EXIT_ZOMBIE != 0 && state & EXIT_ZOMBIE == 0 {
                found = true;
            }
        });
        if found {
            return true;
        }
        for_each_ptrace_wait_match(parent, target, |task| {
            let state = (*task).__state.load(Ordering::Acquire);
            if (*task).m26.exit_state & EXIT_ZOMBIE != 0 && state & EXIT_ZOMBIE == 0 {
                found = true;
            }
        });
    }
    found
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaitBlockAction {
    Recheck,
    Sleep,
    Yield,
}

/// Choose how a blocking child wait should give up the CPU after registering
/// itself as a waiter.
///
/// A half-published zombie has already snapshotted its waiters, so the parent
/// cannot safely sleep: it might never receive the wakeup. It must still yield,
/// though, because the cooperative scheduler may need to run that child before
/// `__state = EXIT_ZOMBIE` can become visible. Keeping the parent runnable while
/// yielding closes both the lost-wakeup window and the child-starvation loop.
fn wait_block_action(reportable: bool, exiting: bool, matching: bool) -> WaitBlockAction {
    if reportable || !matching {
        WaitBlockAction::Recheck
    } else if exiting {
        WaitBlockAction::Yield
    } else {
        WaitBlockAction::Sleep
    }
}

/// Linux's `do_wait()` returns `-ERESTARTSYS` when an interruptible wait has no
/// reportable event and a signal is pending.  Lupos has a short split-publish
/// window where `m26.exit_state` advertises exit in progress before `__state`
/// becomes `EXIT_ZOMBIE`; don't let SIGCHLD escape as EINTR in that window or
/// a parent can miss the child that is about to become reapable.
unsafe fn wait_interrupted_by_signal(parent: *mut TaskStruct, target: WaitTarget) -> bool {
    has_unblocked_pending_signals(parent) && !unsafe { has_exiting_wait_target(parent, target) }
}

/// Returns true iff `parent` has at least one child matching `pid_filter`
/// (zombie or otherwise).
unsafe fn has_matching_child(parent: *mut TaskStruct, target: WaitTarget) -> bool {
    let mut found = false;
    unsafe {
        for_each_real_child(parent, target, |_| {
            found = true;
        });
        if found {
            return true;
        }
        for_each_ptrace_wait_match(parent, target, |_| {
            found = true;
        });
    }
    found
}

unsafe fn report_stopped_task(task: *mut TaskStruct, stat_addr: *mut i32, options: i32) -> i64 {
    unsafe {
        let task_pid = (*task).pid;
        let sig = (*task).m26.ptrace_stop_signal;
        if let Err(errno) = copy_wait_status_to_user(stat_addr, w_stopped(sig)) {
            return errno;
        }
        if options & WNOWAIT == 0 {
            (*task).m26.ptrace_stop_signal = 0;
        }
        task_pid as i64
    }
}

/// Append `parent` to `child.wait_waiters` so `exit_notify` will wake it.
unsafe fn add_waiter(child: *mut TaskStruct, parent: *mut TaskStruct) -> bool {
    if child.is_null() || parent.is_null() {
        return false;
    }
    unsafe {
        let count = ((*child).m26.wait_count as usize).min(MAX_WAITERS);
        if (&(*child).m26.wait_waiters)[..count]
            .iter()
            .any(|waiter| *waiter == parent)
        {
            return false;
        }
        if count < MAX_WAITERS {
            (*child).m26.wait_waiters[count] = parent;
            (*child).m26.wait_count = (count + 1) as u32;
            return true;
        }
    }
    false
}

/// Remove every registration of `parent` from one task's child-exit queue.
unsafe fn remove_waiter(child: *mut TaskStruct, parent: *mut TaskStruct) {
    if child.is_null() || parent.is_null() {
        return;
    }
    unsafe {
        let count = ((*child).m26.wait_count as usize).min(MAX_WAITERS);
        let mut write = 0;
        for read in 0..count {
            let waiter = (*child).m26.wait_waiters[read];
            if !waiter.is_null() && waiter != parent {
                (*child).m26.wait_waiters[write] = waiter;
                write += 1;
            }
        }
        for slot in &mut (&mut (*child).m26.wait_waiters)[write..] {
            *slot = core::ptr::null_mut();
        }
        (*child).m26.wait_count = write as u32;
    }
}

#[derive(Clone, Copy)]
struct WaitRegistration {
    task: *mut TaskStruct,
    pid: i32,
}

struct WaitRegistrationGuard {
    parent: *mut TaskStruct,
    registrations: Vec<WaitRegistration>,
}

impl WaitRegistrationGuard {
    fn new(parent: *mut TaskStruct) -> Self {
        Self {
            parent,
            registrations: Vec::new(),
        }
    }

    /// Register the current wait on `child` and remember the exact inverse
    /// edge for removal.
    ///
    /// Linux removes one stack waitqueue entry from `current->signal` when
    /// `do_wait()` returns. Lupos stores waiters on each child, so tracking
    /// only edges installed by this syscall avoids scanning the entire task
    /// table on every fork/exec/wait round trip.
    unsafe fn register(&mut self, child: *mut TaskStruct) {
        if child.is_null()
            || self.parent.is_null()
            || self
                .registrations
                .iter()
                .any(|registration| registration.task == child)
        {
            return;
        }
        let pid = unsafe { (*child).pid };
        if unsafe { add_waiter(child, self.parent) } {
            self.registrations
                .push(WaitRegistration { task: child, pid });
        }
    }

    /// Remove a child edge before the caller reaps `child`.
    ///
    /// `release_task()` can free the `TaskStruct`; dropping a guard that still
    /// contains that pointer would not be equivalent to Linux's stack waitqueue
    /// removal.
    unsafe fn unregister_before_release(&mut self, child: *mut TaskStruct) {
        if let Some(pos) = self
            .registrations
            .iter()
            .position(|registration| registration.task == child)
        {
            let registration = self.registrations.swap_remove(pos);
            unsafe {
                remove_waiter(registration.task, self.parent);
            }
        }
    }

    unsafe fn cleanup(&mut self) {
        while let Some(registration) = self.registrations.pop() {
            if unsafe { registration_still_names_child(self.parent, registration) } {
                unsafe {
                    remove_waiter(registration.task, self.parent);
                }
            }
        }
    }
}

impl Drop for WaitRegistrationGuard {
    fn drop(&mut self) {
        unsafe { self.cleanup() };
    }
}

unsafe fn registration_still_names_child(
    parent: *mut TaskStruct,
    registration: WaitRegistration,
) -> bool {
    if parent.is_null() || registration.task.is_null() {
        return false;
    }

    if lookup_task_by_pid(registration.pid) == registration.task {
        return true;
    }

    unsafe {
        // Preserve stack-backed test/process-bootstrap children which are not
        // present in either global task registry. The comparison is by pointer
        // only so it does not dereference a possibly released child.
        let count = ((*parent).m26.children_count as usize).min(MAX_CHILDREN);
        for index in 0..count {
            if (*parent).m26.children[index] == registration.task {
                return true;
            }
        }
    }
    false
}

/// Linux keeps one `wait_chldexit` waitqueue in the process-shared
/// `signal_struct`. Register before each child scan so either the scan observes
/// an already-published event or `__wake_up_parent()` observes this waiter.
struct WaitChldexitGuard {
    parent: *mut TaskStruct,
    queue: Option<Arc<crate::kernel::sched::wait::WaitQueueHead>>,
}

impl WaitChldexitGuard {
    fn new(parent: *mut TaskStruct) -> Self {
        Self {
            parent,
            queue: crate::kernel::signal::wait_chldexit_queue(parent),
        }
    }

    unsafe fn prepare(&self) {
        if let Some(queue) = self.queue.as_ref() {
            unsafe {
                queue.prepare_to_wait(self.parent, TASK_INTERRUPTIBLE);
            }
        }
    }
}

impl Drop for WaitChldexitGuard {
    fn drop(&mut self) {
        if let Some(queue) = self.queue.as_ref() {
            unsafe {
                queue.finish_wait(self.parent);
            }
        }
    }
}

// ── sys_wait4 ────────────────────────────────────────────────────────────────

/// Linux `wait4(pid, stat_addr, options, rusage)` — syscall 61.
///
/// Returns the child PID on success and writes the packed status to
/// `*stat_addr` (when non-null).  Returns `-ECHILD` if no children match,
/// `0` for `WNOHANG` with no zombie ready, or a negated errno on error.
///
/// # Safety
/// `stat_addr` and `rusage`, when non-null, are userspace syscall pointers.
pub unsafe fn sys_wait4(pid: i32, stat_addr: *mut i32, options: i32, _rusage: *mut Rusage) -> i64 {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let current = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-wait4-enter pid={} wait_pid={} options={:#x}",
            current,
            pid,
            options
        );
    }
    if options & !(WNOHANG | WUNTRACED | WCONTINUED | __WNOTHREAD | __WCLONE | __WALL) != 0 {
        return EINVAL;
    }
    if pid == i32::MIN {
        return ESRCH;
    }

    let parent = unsafe { sched::get_current() };
    if parent.is_null() {
        return EINVAL;
    }
    let target = if pid == -1 {
        WaitTarget::Any
    } else if pid < 0 {
        WaitTarget::Pgrp(-pid)
    } else if pid == 0 {
        WaitTarget::CurrentPgrp
    } else {
        WaitTarget::Pid(pid)
    };
    let wait_chldexit_guard = WaitChldexitGuard::new(parent);
    let mut wait_registration_guard = WaitRegistrationGuard::new(parent);

    loop {
        unsafe {
            wait_chldexit_guard.prepare();
        }
        // Fast path: zombie child already available?
        if let Some(child) = unsafe { find_zombie_child(parent, target) } {
            let child_pid = unsafe { (*child).pid };
            let exit_code = unsafe { (*child).m26.exit_code };
            if let Err(errno) = unsafe { copy_wait_status_to_user(stat_addr, exit_code) } {
                return errno;
            }

            // WNOWAIT keeps the zombie alive; otherwise reap it.
            if options & WNOWAIT == 0 {
                unsafe {
                    wait_registration_guard.unregister_before_release(child);
                    release_task(child);
                }
            }
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-wait4-reap child={} status={:#x}",
                    child_pid,
                    exit_code
                );
            }
            return child_pid as i64;
        }

        if let Some(child) = unsafe { find_stopped_child(parent, target) } {
            if unsafe { should_report_stopped_child(child, options) } {
                return unsafe { report_stopped_task(child, stat_addr, options) };
            }
        }

        if let Some(task) = unsafe { find_ptrace_wait_task(parent, target) } {
            if unsafe { should_report_stopped_child(task, options) } {
                return unsafe { report_stopped_task(task, stat_addr, options) };
            }
        }

        // No zombie ready.  Bail out if no matching children at all.
        if !unsafe { has_matching_child(parent, target) } {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-wait4-echild wait_pid={}",
                    pid
                );
            }
            return ECHILD;
        }

        // WNOHANG: don't block.
        if options & WNOHANG != 0 {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-wait4-nohang wait_pid={}",
                    pid
                );
            }
            return 0;
        }

        // Linux `do_wait()` breaks out with -ERESTARTSYS once an interruptible
        // child wait observes a pending signal. The arch signal-exit path then
        // turns it into EINTR or rewinds the syscall for SA_RESTART handlers.
        if unsafe { wait_interrupted_by_signal(parent, target) } {
            return ERESTARTSYS;
        }

        // Cooperative-scheduler block: register on every matching child's
        // wait queue, set self interruptible, yield.  When a child exits its
        // `exit_notify` flips us back to TASK_RUNNING.
        unsafe {
            for_each_real_child(parent, target, |c| {
                wait_registration_guard.register(c);
            });
            for_each_ptrace_wait_match(parent, target, |task| {
                wait_registration_guard.register(task);
            });
            (*parent)
                .__state
                .store(TASK_INTERRUPTIBLE, Ordering::Release);
            let action = wait_block_action(
                has_reportable_wait_event(parent, target, options),
                has_exiting_wait_target(parent, target),
                has_matching_child(parent, target),
            );
            match action {
                WaitBlockAction::Recheck => {}
                WaitBlockAction::Sleep => {
                    #[cfg(not(test))]
                    if crate::kernel::debug_trace::proc_enabled() {
                        crate::linux_driver_abi::tty::serial_println!(
                            "trace-proc-wait4-block pid={} wait_pid={}",
                            (*parent).pid,
                            target.trace_value()
                        );
                    }
                    sched::schedule_with_irqs_enabled();
                }
                WaitBlockAction::Yield => {
                    (*parent).__state.store(TASK_RUNNING, Ordering::Release);
                    sched::reschedule_runnable();
                }
            }
            // After waking, reset state and re-check the children.
            (*parent).__state.store(TASK_RUNNING, Ordering::Release);
        }
    }
}

// ── sys_waitid ───────────────────────────────────────────────────────────────

/// Linux `waitid(which, upid, infop, options, rusage)` — syscall 247.
///
/// Fills `*infop` with the waited child's siginfo and returns 0 on success.
pub unsafe fn sys_waitid(
    which: i32,
    upid: i32,
    infop: *mut WaitidSigInfo,
    options: i32,
    _rusage: *mut Rusage,
) -> i64 {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let current = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-waitid-enter pid={} which={} upid={} options={:#x}",
            current,
            which,
            upid,
            options
        );
    }
    let target = match which {
        P_ALL => WaitTarget::Any,
        P_PID => {
            if upid <= 0 {
                return EINVAL;
            }
            WaitTarget::Pid(upid)
        }
        P_PGID => {
            if upid < 0 {
                return EINVAL;
            }
            if upid == 0 {
                WaitTarget::CurrentPgrp
            } else {
                WaitTarget::Pgrp(upid)
            }
        }
        P_PIDFD => {
            if upid < 0 {
                return EINVAL;
            }
            match crate::fs::pidfd::pid_for_fd(upid) {
                Ok(pid) => WaitTarget::Pid(pid),
                Err(errno) => return -(errno as i64),
            }
        }
        _ => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-waitid-einval which={}",
                    which
                );
            }
            return EINVAL;
        }
    };

    let parent = unsafe { sched::get_current() };
    if parent.is_null() {
        return EINVAL;
    }
    let wait_chldexit_guard = WaitChldexitGuard::new(parent);
    let mut wait_registration_guard = WaitRegistrationGuard::new(parent);

    loop {
        unsafe {
            wait_chldexit_guard.prepare();
        }
        if let Some(child) = unsafe { find_zombie_child(parent, target) } {
            let child_pid = unsafe { (*child).pid };
            let exit_code = unsafe { (*child).m26.exit_code };
            let (si_code, si_status) = waitid_code_status(exit_code);

            if !infop.is_null() {
                let info = WaitidSigInfo {
                    si_signo: SIGCHLD,
                    si_errno: 0,
                    si_code,
                    _pad0: 0,
                    si_pid: child_pid,
                    si_uid: 0, // M27 will populate from cred.
                    si_status,
                    _pad_tail: [0; 100],
                };
                if let Err(errno) = unsafe { copy_waitid_siginfo_to_user(infop, &info) } {
                    return errno;
                }
            }

            if options & WNOWAIT == 0 {
                unsafe {
                    wait_registration_guard.unregister_before_release(child);
                    release_task(child);
                }
            }
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-waitid-reap child={} status={:#x}",
                    child_pid,
                    exit_code
                );
            }
            return 0;
        }

        if let Some(child) = unsafe { find_stopped_child(parent, target) } {
            if unsafe { should_report_stopped_child(child, options) } {
                let child_pid = unsafe { (*child).pid };
                let sig = unsafe { (*child).m26.ptrace_stop_signal };

                if !infop.is_null() {
                    let info = WaitidSigInfo {
                        si_signo: SIGCHLD,
                        si_errno: 0,
                        si_code: CLD_TRAPPED,
                        _pad0: 0,
                        si_pid: child_pid,
                        si_uid: 0,
                        si_status: sig,
                        _pad_tail: [0; 100],
                    };
                    if let Err(errno) = unsafe { copy_waitid_siginfo_to_user(infop, &info) } {
                        return errno;
                    }
                }
                if options & WNOWAIT == 0 {
                    unsafe {
                        (*child).m26.ptrace_stop_signal = 0;
                    }
                }
                return 0;
            }
        }

        if let Some(task) = unsafe { find_ptrace_wait_task(parent, target) } {
            if unsafe { should_report_stopped_child(task, options) } {
                let task_pid = unsafe { (*task).pid };
                let sig = unsafe { (*task).m26.ptrace_stop_signal };

                if !infop.is_null() {
                    let info = WaitidSigInfo {
                        si_signo: SIGCHLD,
                        si_errno: 0,
                        si_code: CLD_TRAPPED,
                        _pad0: 0,
                        si_pid: task_pid,
                        si_uid: 0,
                        si_status: sig,
                        _pad_tail: [0; 100],
                    };
                    if let Err(errno) = unsafe { copy_waitid_siginfo_to_user(infop, &info) } {
                        return errno;
                    }
                }
                if options & WNOWAIT == 0 {
                    unsafe {
                        (*task).m26.ptrace_stop_signal = 0;
                    }
                }
                return 0;
            }
        }

        if !unsafe { has_matching_child(parent, target) } {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-waitid-echild pid_filter={}",
                    target.trace_value()
                );
            }
            return ECHILD;
        }
        if options & WNOHANG != 0 {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-waitid-nohang pid_filter={}",
                    target.trace_value()
                );
            }
            return 0;
        }

        // See `sys_wait4`: Linux's child wait is interruptible by any
        // unblocked pending signal after immediate child events are exhausted.
        if unsafe { wait_interrupted_by_signal(parent, target) } {
            return ERESTARTSYS;
        }

        unsafe {
            for_each_real_child(parent, target, |c| {
                wait_registration_guard.register(c);
            });
            for_each_ptrace_wait_match(parent, target, |task| {
                wait_registration_guard.register(task);
            });
            (*parent)
                .__state
                .store(TASK_INTERRUPTIBLE, Ordering::Release);
            let action = wait_block_action(
                has_reportable_wait_event(parent, target, options),
                has_exiting_wait_target(parent, target),
                has_matching_child(parent, target),
            );
            match action {
                WaitBlockAction::Recheck => {}
                WaitBlockAction::Sleep => {
                    #[cfg(not(test))]
                    if crate::kernel::debug_trace::proc_enabled() {
                        crate::linux_driver_abi::tty::serial_println!(
                            "trace-proc-waitid-block pid={} pid_filter={}",
                            (*parent).pid,
                            target.trace_value()
                        );
                    }
                    sched::schedule_with_irqs_enabled();
                }
                WaitBlockAction::Yield => {
                    (*parent).__state.store(TASK_RUNNING, Ordering::Release);
                    sched::reschedule_runnable();
                }
            }
            (*parent).__state.store(TASK_RUNNING, Ordering::Release);
        }
    }
}

// ── sys_exit / sys_exit_group ────────────────────────────────────────────────

/// Linux `exit(code)` — syscall 60.  Never returns.
pub unsafe fn sys_exit(code: i32) -> ! {
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!("trace-proc-exit pid={} code={}", pid, code);
    }
    unsafe {
        crate::kernel::exit::do_exit(w_exitcode(code, 0) as i64);
    }
}

/// Linux `exit_group(code)` — syscall 231.  Never returns.
///
/// `do_group_exit()` queues SIGKILL for every sibling so each unwinds its own
/// stack before the current task publishes its terminal state.
pub unsafe fn sys_exit_group(code: i32) -> ! {
    let current = unsafe { sched::get_current() };
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let pid = if current.is_null() {
            -1
        } else {
            unsafe { (*current).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-exit-group pid={} code={}",
            pid,
            code
        );
    }
    let packed = w_exitcode(code, 0) as i64;
    unsafe {
        crate::kernel::exit::do_group_exit(packed);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::fdtable::FilesStruct;
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::pid::{INIT_PID_NS, alloc_pid};
    use alloc::boxed::Box;

    fn task(pid: i32) -> Box<TaskStruct> {
        let mut t = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        t.pid = pid;
        t.tgid = pid;
        t.m26 = crate::kernel::task::M26Fields::zeroed();
        t
    }

    unsafe fn install_test_signal_handler(sig: i32) {
        let action = crate::kernel::signal::RtSigAction {
            sa_handler: 0x1234,
            ..Default::default()
        };
        assert_eq!(
            unsafe {
                crate::kernel::signal::sys_rt_sigaction(
                    sig,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<crate::kernel::signal::SigSet>(),
                )
            },
            0
        );
    }

    #[test]
    fn w_exitcode_normal_exit() {
        // exit(42) → low 7 bits termsig=0, bits 8..15 = 42.
        assert_eq!(w_exitcode(42, 0), 42 << 8);
        assert!(w_ifexited(w_exitcode(42, 0)));
        assert_eq!(w_exitstatus(w_exitcode(42, 0)), 42);
    }

    #[test]
    fn w_exitcode_signalled() {
        // Killed by signal 9 — termsig in low bits.
        let s = w_exitcode(0, 9);
        assert_eq!(s, 9);
        assert_eq!(w_termsig(s), 9);
        assert!(!w_ifexited(s));
    }

    #[test]
    fn wait_constants_match_linux() {
        assert_eq!(WNOHANG, 0x0000_0001);
        assert_eq!(WEXITED, 0x0000_0004);
        assert_eq!(WCONTINUED, 0x0000_0008);
        assert_eq!(WNOWAIT, 0x0100_0000);
        assert_eq!(P_ALL, 0);
        assert_eq!(P_PID, 1);
        assert_eq!(P_PIDFD, 3);
        assert_eq!(CLD_EXITED, 1);
    }

    #[test]
    fn waitid_zombie_status_uses_linux_branch_order_and_full_exit_status() {
        assert_eq!(waitid_code_status(WCOREFLAG), (CLD_EXITED, 0));
        assert_eq!(waitid_code_status(0x1234_00), (CLD_EXITED, 0x1234));
        assert_eq!(waitid_code_status(WCOREFLAG | 11), (CLD_DUMPED, 11));
        assert_eq!(waitid_code_status(9), (CLD_KILLED, 9));
    }

    #[test]
    fn stopped_ptrace_child_is_waitable() {
        let mut parent = task(100);
        let mut child = task(101);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;
        child.m26.ptrace_stop_signal = 5;
        child.__state.store(
            crate::kernel::task::task_state::__TASK_TRACED,
            Ordering::Release,
        );

        let found = unsafe { find_stopped_child(&mut *parent as *mut TaskStruct, WaitTarget::Any) };

        assert_eq!(found, Some(&mut *child as *mut TaskStruct));
        assert_eq!(w_stopped(5), (5 << 8) | 0x7f);
    }

    #[test]
    fn reportable_wait_event_detects_zombie_before_sleep() {
        let mut parent = task(100);
        let mut child = task(101);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);

        assert!(unsafe {
            has_reportable_wait_event(&mut *parent as *mut TaskStruct, WaitTarget::Any, 0)
        });
    }

    #[test]
    fn wait_chldexit_registration_is_visible_before_child_scan() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        let mut parent = task(109);
        assert_eq!(
            unsafe { crate::kernel::signal::prepare_timer_signal_target(&mut *parent) },
            0
        );
        let queue = crate::kernel::signal::wait_chldexit_queue(&mut *parent)
            .expect("registered parent wait_chldexit queue");
        let guard = WaitChldexitGuard::new(&mut *parent);

        unsafe {
            guard.prepare();
        }
        assert_eq!(queue.len(), 1);
        assert_eq!(parent.__state.load(Ordering::Acquire), TASK_INTERRUPTIBLE);
        assert_eq!(queue.wake_up_all(), 1);
        assert_eq!(parent.__state.load(Ordering::Acquire), TASK_RUNNING);

        drop(guard);
        assert!(queue.is_empty());
        crate::kernel::signal::reset_for_tests();
    }

    #[test]
    fn wait4_returns_erestartsys_for_unblocked_pending_signal() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { sched::get_current() };

        let mut parent = task(110);
        let mut child = task(111);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            install_test_signal_handler(crate::kernel::signal::SIGALRM);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *parent as *mut TaskStruct,
                    crate::kernel::signal::SIGALRM,
                ),
                0
            );

            assert_eq!(
                sys_wait4(-1, core::ptr::null_mut(), 0, core::ptr::null_mut()),
                ERESTARTSYS
            );
            assert_eq!(parent.m26.children_count, 1);
            assert_eq!(child.m26.wait_count, 0);

            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
    }

    #[test]
    fn waitid_returns_erestartsys_for_unblocked_pending_signal() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { sched::get_current() };

        let mut parent = task(112);
        let mut child = task(113);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            install_test_signal_handler(crate::kernel::signal::SIGALRM);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *parent as *mut TaskStruct,
                    crate::kernel::signal::SIGALRM,
                ),
                0
            );

            assert_eq!(
                sys_waitid(
                    P_ALL,
                    0,
                    core::ptr::null_mut(),
                    WEXITED,
                    core::ptr::null_mut(),
                ),
                ERESTARTSYS
            );
            assert_eq!(parent.m26.children_count, 1);
            assert_eq!(child.m26.wait_count, 0);

            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
    }

    #[test]
    fn half_published_zombie_yields_without_sleeping() {
        let mut parent = task(1200);
        let mut child = task(1201);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;
        child.m26.exit_state = EXIT_ZOMBIE;

        unsafe {
            add_waiter(
                &mut *child as *mut TaskStruct,
                &mut *parent as *mut TaskStruct,
            );
            parent.__state.store(TASK_INTERRUPTIBLE, Ordering::Release);

            assert!(has_exiting_wait_target(
                &mut *parent as *mut TaskStruct,
                WaitTarget::Any
            ));
            assert!(!has_reportable_wait_event(
                &mut *parent as *mut TaskStruct,
                WaitTarget::Any,
                0
            ));
            assert!(has_matching_child(
                &mut *parent as *mut TaskStruct,
                WaitTarget::Any
            ));
            assert_eq!(
                wait_block_action(
                    has_reportable_wait_event(&mut *parent as *mut TaskStruct, WaitTarget::Any, 0),
                    has_exiting_wait_target(&mut *parent as *mut TaskStruct, WaitTarget::Any),
                    has_matching_child(&mut *parent as *mut TaskStruct, WaitTarget::Any),
                ),
                WaitBlockAction::Yield,
                "the parent must remain runnable and yield while zombie publication finishes"
            );
        }
    }

    #[test]
    fn ordinary_child_wait_sleeps_and_immediate_events_recheck() {
        assert_eq!(
            wait_block_action(false, false, true),
            WaitBlockAction::Sleep
        );
        assert_eq!(
            wait_block_action(true, false, true),
            WaitBlockAction::Recheck
        );
        assert_eq!(
            wait_block_action(false, false, false),
            WaitBlockAction::Recheck
        );
    }

    #[test]
    fn pending_signal_does_not_interrupt_half_published_zombie() {
        let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { sched::get_current() };

        let mut parent = task(1300);
        let mut child = task(1301);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;
        child.m26.exit_state = EXIT_ZOMBIE;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            install_test_signal_handler(crate::kernel::signal::SIGALRM);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *parent as *mut TaskStruct,
                    crate::kernel::signal::SIGALRM,
                ),
                0
            );

            assert!(has_exiting_wait_target(
                &mut *parent as *mut TaskStruct,
                WaitTarget::Any,
            ));
            assert!(!wait_interrupted_by_signal(
                &mut *parent as *mut TaskStruct,
                WaitTarget::Any,
            ));

            sched::set_current(previous);
        }
        crate::kernel::signal::reset_for_tests();
    }

    #[test]
    fn waitid_pidfd_resolves_target_child() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(500);
        parent.cred = &raw const INIT_CRED;
        let mut child = task(501);
        child.cred = &raw const INIT_CRED;
        let kpid = alloc_pid(&INIT_PID_NS, Some(child.pid)).expect("pid alloc");
        child.m26.thread_pid = Box::into_raw(kpid);
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);
        child.m26.exit_code = w_exitcode(7, 0);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            crate::kernel::files::set_task_files(
                &mut *parent as *mut TaskStruct,
                FilesStruct::new(),
            );
            sched::set_current(&mut *parent as *mut TaskStruct);
            let fd =
                crate::fs::pidfd::install_pidfd(&mut *child as *mut TaskStruct, false).unwrap();
            let mut info = WaitidSigInfo::default();

            assert_eq!(
                sys_waitid(
                    P_PIDFD,
                    fd,
                    &mut info,
                    WEXITED | WNOWAIT,
                    core::ptr::null_mut()
                ),
                0
            );
            assert_eq!(info.si_pid, 501);
            assert_eq!(info.si_status, 7);

            let files =
                crate::kernel::files::get_task_files(&mut *parent as *mut TaskStruct).unwrap();
            files.close(fd).unwrap();
            crate::kernel::files::drop_task_files(&mut *parent as *mut TaskStruct);
            sched::set_current(previous);
            child.m26.thread_pid = core::ptr::null_mut();
        }
    }

    #[test]
    fn waitid_reports_sigkill_zombie_as_cld_killed() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(600);
        let mut child = task(601);
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);
        child.m26.exit_code = w_exitcode(0, crate::kernel::signal::SIGKILL);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            let mut info = WaitidSigInfo::default();

            assert_eq!(
                sys_waitid(
                    P_ALL,
                    0,
                    &mut info,
                    WEXITED | WNOWAIT,
                    core::ptr::null_mut(),
                ),
                0
            );
            assert_eq!(info.si_pid, 601);
            assert_eq!(info.si_code, CLD_KILLED);
            assert_eq!(info.si_status, crate::kernel::signal::SIGKILL);

            sched::set_current(previous);
        }
    }

    #[test]
    fn wait4_negative_pid_selects_process_group() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(700);
        let mut child = task(701);
        child.m26.ptrace_stop_signal = crate::kernel::signal::SIGTSTP;
        child.__state.store(
            crate::kernel::task::task_state::__TASK_STOPPED,
            Ordering::Release,
        );
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            crate::kernel::session::reset_for_tests();
            sched::set_current(&mut *parent as *mut TaskStruct);
            let mut status = 0;

            assert_eq!(
                sys_wait4(
                    -child.pid,
                    &mut status,
                    WNOHANG | WUNTRACED,
                    core::ptr::null_mut()
                ),
                child.pid as i64
            );
            assert_eq!(status, w_stopped(crate::kernel::signal::SIGTSTP));
            assert_eq!(
                sys_wait4(
                    -child.pid,
                    &mut status,
                    WNOHANG | WUNTRACED,
                    core::ptr::null_mut()
                ),
                0
            );
            assert_eq!(
                child.__state.load(Ordering::Acquire),
                crate::kernel::task::task_state::__TASK_STOPPED
            );
            assert_eq!(
                sys_wait4(
                    -(child.pid + 1),
                    core::ptr::null_mut(),
                    WNOHANG | WUNTRACED,
                    core::ptr::null_mut()
                ),
                ECHILD
            );

            sched::set_current(previous);
        }
    }

    #[test]
    fn waitid_pgid_selects_process_group() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(710);
        let mut child = task(711);
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);
        child.m26.exit_code = w_exitcode(3, 0);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            crate::kernel::session::reset_for_tests();
            sched::set_current(&mut *parent as *mut TaskStruct);
            let mut info = WaitidSigInfo::default();

            assert_eq!(
                sys_waitid(
                    P_PGID,
                    child.pid,
                    &mut info,
                    WEXITED | WNOWAIT | WNOHANG,
                    core::ptr::null_mut(),
                ),
                0
            );
            assert_eq!(info.si_pid, child.pid);
            assert_eq!(info.si_status, 3);
            assert_eq!(
                sys_waitid(
                    P_PID,
                    0,
                    core::ptr::null_mut(),
                    WEXITED | WNOHANG,
                    core::ptr::null_mut(),
                ),
                EINVAL
            );

            sched::set_current(previous);
        }
    }

    #[test]
    fn rusage_size_is_at_least_144() {
        assert!(core::mem::size_of::<Rusage>() >= 144);
    }

    #[test]
    fn wait4_rejects_kernel_status_pointer_without_reaping() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(700);
        let mut child = task(701);
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);
        child.m26.exit_code = w_exitcode(3, 0);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            assert_eq!(
                sys_wait4(
                    -1,
                    0xffff_8000_0000_0000usize as *mut i32,
                    0,
                    core::ptr::null_mut(),
                ),
                EFAULT
            );
            assert_eq!(parent.m26.children_count, 1);
            sched::set_current(previous);
        }
    }

    #[test]
    fn waitid_rejects_kernel_siginfo_pointer_without_reaping() {
        let previous = unsafe { sched::get_current() };

        let mut parent = task(710);
        let mut child = task(711);
        child.m26.exit_state = EXIT_ZOMBIE;
        child.__state.store(EXIT_ZOMBIE, Ordering::Release);
        child.m26.exit_code = w_exitcode(4, 0);
        parent.m26.children[0] = &mut *child as *mut TaskStruct;
        parent.m26.children_count = 1;

        unsafe {
            sched::set_current(&mut *parent as *mut TaskStruct);
            assert_eq!(
                sys_waitid(
                    P_ALL,
                    0,
                    0xffff_8000_0000_0000usize as *mut WaitidSigInfo,
                    WEXITED,
                    core::ptr::null_mut(),
                ),
                EFAULT
            );
            assert_eq!(parent.m26.children_count, 1);
            sched::set_current(previous);
        }
    }

    #[test]
    fn waitid_siginfo_total_size_is_128() {
        assert_eq!(core::mem::size_of::<WaitidSigInfo>(), 128);
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let previous = unsafe { sched::get_current() };
        unsafe {
            sched::set_current(core::ptr::null_mut());
            assert_eq!(
                sys_wait4(-2, core::ptr::null_mut(), 0, core::ptr::null_mut()),
                EINVAL
            );
            assert_eq!(
                sys_wait4(-1, core::ptr::null_mut(), WNOHANG, core::ptr::null_mut()),
                EINVAL
            );
            assert_eq!(
                sys_waitid(
                    P_PIDFD,
                    0,
                    core::ptr::null_mut(),
                    WEXITED | WNOHANG,
                    core::ptr::null_mut()
                ),
                -(crate::include::uapi::errno::EBADF as i64)
            );
            assert_eq!(
                sys_waitid(
                    P_PGID,
                    0,
                    core::ptr::null_mut(),
                    WNOHANG,
                    core::ptr::null_mut()
                ),
                EINVAL
            );
            let _: unsafe fn(i32) -> ! = sys_exit;
            let _: unsafe fn(i32) -> ! = sys_exit_group;
            sched::set_current(previous);
        }
    }
}
