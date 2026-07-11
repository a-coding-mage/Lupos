//! linux-parity: complete
//! linux-source: vendor/linux/kernel/signal.c
//! test-origin: linux:vendor/linux/kernel/signal.c
//! Linux-style signal state scaffolding (M25).
//!
//! This module implements core signal mask/action/pending semantics and syscall
//! surfaces. Delivery frame wiring is deferred, but queueing and selection
//! behavior is implemented and unit-tested.
//!
//! Reference: vendor/linux/kernel/signal.c
//!            vendor/linux/include/linux/signal.h
//!            vendor/linux/include/uapi/asm-generic/signal.h
//!
//! The Linux signal subsystem in `kernel/signal.c` provides the queueing
//! datastructures (`sigqueue`, `sigpending`), the delivery selection routine
//! (`dequeue_signal`/`next_signal`), the action installation surfaces
//! (`rt_sigaction`), mask manipulation (`sigprocmask`), synchronous wait
//! (`rt_sigtimedwait`), and the unblockable delivery helpers (`force_sig`,
//! `force_sig_info`, `kill_pid`, `kill_pgrp`).  This module ports those entry
//! points; the architecture-specific signal-frame setup lives in
//! `arch/x86/signal.rs`, mirroring Linux's `arch/x86/kernel/signal.c`.

extern crate alloc;

use alloc::{collections::VecDeque, vec::Vec};
use core::ffi::c_void;

use crate::arch::x86::kernel::uaccess;
use crate::include::uapi::errno::EINTR;
use crate::kernel::sched;

pub const NSIG: usize = 64;
/// Signal number range — RT signals start at SIGRTMIN.
pub const SIGRTMIN: i32 = 32;
pub const SIGRTMAX: i32 = NSIG as i32;
pub const SI_USER: i32 = 0;
pub const SI_TKILL: i32 = -6;

pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGIOT: i32 = SIGABRT;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

pub const BUS_MCEERR_AR: i32 = 4;

pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

pub const SA_NOCLDSTOP: u64 = 0x0000_0001;
pub const SA_NOCLDWAIT: u64 = 0x0000_0002;
pub const SA_SIGINFO: u64 = 0x0000_0004;
pub const SA_RESTORER: u64 = 0x0400_0000;
pub const SA_ONSTACK: u64 = 0x0800_0000;
pub const SA_RESTART: u64 = 0x1000_0000;
pub const SA_NODEFER: u64 = 0x4000_0000;
pub const SA_RESETHAND: u64 = 0x8000_0000;

// Kernel-internal restart pseudo-errors from vendor/linux/include/linux/errno.h.
const ERESTARTSYS: i32 = 512;
const ERESTARTNOINTR: i32 = 513;
const ERESTARTNOHAND: i32 = 514;
const ERESTART_RESTARTBLOCK: i32 = 516;

// x86-64 Linux values used by arch_do_signal_or_restart().
const X86_64_NR_RESTART_SYSCALL: u64 = 219;
const X86_64_SYSCALL_INSN_LEN: u64 = 2;
const NO_SYSCALL: u64 = u64::MAX;

const SS_ONSTACK: u32 = 1;
const SS_DISABLE: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SigSet {
    pub bits: u64,
}

impl SigSet {
    #[inline]
    pub fn contains(self, sig: i32) -> bool {
        let bit = sig_bit(sig);
        bit != 0 && (self.bits & bit) != 0
    }

    #[inline]
    pub fn add(&mut self, sig: i32) {
        let bit = sig_bit(sig);
        if bit != 0 {
            self.bits |= bit;
        }
    }

    #[inline]
    pub fn remove(&mut self, sig: i32) {
        let bit = sig_bit(sig);
        if bit != 0 {
            self.bits &= !bit;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RtSigAction {
    pub sa_handler: usize,
    pub sa_flags: u64,
    pub sa_restorer: usize,
    pub sa_mask: SigSet,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SigInfo {
    pub signo: i32,
    pub errno: i32,
    pub code: i32,
    _pad0: i32,
    _sifields: [u8; 112],
}

impl SigInfo {
    pub const fn new(signo: i32, code: i32) -> Self {
        Self {
            signo,
            errno: 0,
            code,
            _pad0: 0,
            _sifields: [0; 112],
        }
    }

    pub fn with_sigchld(signo: i32, pid: i32, status: i32) -> Self {
        let mut info = Self::new(signo, 0);
        info._sifields[0..4].copy_from_slice(&pid.to_ne_bytes());
        info._sifields[4..8].copy_from_slice(&0u32.to_ne_bytes());
        info._sifields[8..12].copy_from_slice(&status.to_ne_bytes());
        info
    }

    pub fn with_sender(signo: i32, code: i32, pid: i32, uid: u32) -> Self {
        let mut info = Self::new(signo, code);
        info._sifields[0..4].copy_from_slice(&pid.to_ne_bytes());
        info._sifields[4..8].copy_from_slice(&uid.to_ne_bytes());
        info
    }

    pub fn with_sigfault(signo: i32, code: i32, addr: u64, addr_lsb: i16) -> Self {
        let mut info = Self::new(signo, code);
        info._sifields[0..8].copy_from_slice(&addr.to_ne_bytes());
        info._sifields[8..10].copy_from_slice(&addr_lsb.to_ne_bytes());
        info
    }
}

impl Default for SigInfo {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SigAltStack {
    pub ss_sp: usize,
    pub ss_flags: u32,
    pub ss_size: usize,
}

#[derive(Clone, Debug)]
struct SignalState {
    actions: [RtSigAction; NSIG + 1],
    blocked: SigSet,
    sigsuspend_saved: Option<SigSet>,
    pending: SigSet,
    shared_pending: SigSet,
    /// Siginfo records for task-directed signals. Standard signals keep one
    /// record; realtime signals keep every queued instance.
    rt_queue: VecDeque<SigInfo>,
    /// Siginfo records for process-directed signals. Keeping this separate
    /// prevents task/shared instances of the same signal from exchanging
    /// sender metadata during dequeue.
    shared_queue: VecDeque<SigInfo>,
    altstack: SigAltStack,
    pid: i32,
    tgid: i32,
}

impl SignalState {
    fn new(pid: i32, tgid: i32) -> Self {
        let mut actions = [RtSigAction::default(); NSIG + 1];
        actions[SIGCHLD as usize].sa_handler = 0; // SIG_DFL
        Self {
            actions,
            blocked: SigSet::default(),
            sigsuspend_saved: None,
            pending: SigSet::default(),
            shared_pending: SigSet::default(),
            rt_queue: VecDeque::new(),
            shared_queue: VecDeque::new(),
            altstack: SigAltStack::default(),
            pid,
            tgid,
        }
    }

    fn dequeue_unblocked_signal(&mut self) -> Option<SigInfo> {
        let mut merged = self.pending.bits | self.shared_pending.bits;
        merged &= !self.blocked.bits;
        if merged == 0 {
            return None;
        }
        let sig = merged.trailing_zeros() as i32 + 1;
        if self.pending.contains(sig) {
            Some(take_task_signal_info(self, sig))
        } else {
            Some(take_shared_signal_info(self, sig))
        }
    }

    fn dequeue_specific_signal(&mut self, sig: i32) -> Option<SigInfo> {
        if !self.pending.contains(sig) && !self.shared_pending.contains(sig) {
            return None;
        }
        if self.pending.contains(sig) {
            return Some(take_task_signal_info(self, sig));
        }
        Some(take_shared_signal_info(self, sig))
    }

    fn dequeue_masked_signal(&mut self, mask: u64) -> Option<SigInfo> {
        let mask = user_dequeue_signal_mask(mask);
        if mask == 0 {
            return None;
        }

        let merged = (self.pending.bits | self.shared_pending.bits) & mask;
        if merged == 0 {
            return None;
        }
        let sig = merged.trailing_zeros() as i32 + 1;
        if self.pending.contains(sig) {
            Some(take_task_signal_info(self, sig))
        } else {
            Some(take_shared_signal_info(self, sig))
        }
    }

    fn remove_signal(&mut self, sig: i32) {
        self.pending.remove(sig);
        self.shared_pending.remove(sig);
        self.rt_queue.retain(|info| info.signo != sig);
        self.shared_queue.retain(|info| info.signo != sig);
    }

    fn remove_signal_mask(&mut self, mask: u64) {
        self.pending.bits &= !mask;
        self.shared_pending.bits &= !mask;
        self.rt_queue.retain(|info| sig_bit(info.signo) & mask == 0);
        self.shared_queue
            .retain(|info| sig_bit(info.signo) & mask == 0);
    }
}

fn take_task_signal_info(state: &mut SignalState, sig: i32) -> SigInfo {
    let info = state
        .rt_queue
        .iter()
        .position(|info| info.signo == sig)
        .and_then(|idx| state.rt_queue.remove(idx))
        .unwrap_or_else(|| SigInfo::new(sig, 0));
    if !state.rt_queue.iter().any(|queued| queued.signo == sig) {
        state.pending.remove(sig);
    }
    info
}

fn take_shared_signal_info(state: &mut SignalState, sig: i32) -> SigInfo {
    let info = state
        .shared_queue
        .iter()
        .position(|info| info.signo == sig)
        .and_then(|idx| state.shared_queue.remove(idx))
        .unwrap_or_else(|| SigInfo::new(sig, 0));
    if !state.shared_queue.iter().any(|queued| queued.signo == sig) {
        state.shared_pending.remove(sig);
    }
    info
}

/// Queue one signal while retaining the siginfo record Linux exposes to
/// SA_SIGINFO handlers. Realtime signals queue every instance; standard
/// signals coalesce and retain the first sender's record until delivery.
fn queue_signal_info(state: &mut SignalState, sig: i32, info: SigInfo, shared: bool) {
    let already_pending = if shared {
        state.shared_pending.contains(sig)
    } else {
        state.pending.contains(sig)
    };
    if sig >= SIGRTMIN || !already_pending {
        if shared {
            state.shared_queue.push_back(info);
        } else {
            state.rt_queue.push_back(info);
        }
    }
    if shared {
        state.shared_pending.add(sig);
    } else {
        state.pending.add(sig);
    }
}

fn sanitize_blocked_mask(mask: &mut SigSet) {
    // Linux never blocks SIGKILL/SIGSTOP.
    mask.remove(SIGKILL);
    mask.remove(SIGSTOP);
}

/// Shared by thread-group tasks. Attached to `task_struct.signal`.
#[repr(C)]
pub struct SignalStruct {
    pub blocked: SigSet,
    pub pending: SigSet,
    pub shared_pending: SigSet,
}

struct SignalTable {
    states: Vec<SignalState>,
}

impl SignalTable {
    fn state_for_new_task(&self, pid: i32, tgid: i32) -> SignalState {
        let mut state = SignalState::new(pid, tgid);
        if let Some(source) = self
            .states
            .iter()
            .find(|state| state.pid == tgid && state.tgid == tgid)
            .or_else(|| self.states.iter().find(|state| state.tgid == tgid))
        {
            state.actions = source.actions;
        }
        state
    }

    fn get_or_create_current_index(&mut self) -> Result<usize, i32> {
        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return Err(-3); // ESRCH
        }
        let (pid, tgid) = unsafe { ((*task).pid, (*task).tgid) };
        if let Some(pos) = self.states.iter().position(|s| s.pid == pid) {
            return Ok(pos);
        }
        let state = self.state_for_new_task(pid, tgid);
        self.states.push(state);
        Ok(self.states.len() - 1)
    }

    fn get_or_create_current(&mut self) -> Result<&mut SignalState, i32> {
        let idx = self.get_or_create_current_index()?;
        Ok(self.states.get_mut(idx).expect("index exists"))
    }

    fn get_by_pid_mut(&mut self, pid: i32) -> Option<&mut SignalState> {
        let pos = self.states.iter().position(|s| s.pid == pid)?;
        self.states.get_mut(pos)
    }

    fn get_or_create_task_index(&mut self, pid: i32, tgid: i32) -> usize {
        if let Some(pos) = self.states.iter().position(|s| s.pid == pid) {
            return pos;
        }
        let state = self.state_for_new_task(pid, tgid);
        self.states.push(state);
        self.states.len() - 1
    }

    fn inherit_for_clone(&mut self, parent_pid: i32, child_pid: i32, child_tgid: i32) {
        let inherited = self
            .states
            .iter()
            .find(|state| state.pid == parent_pid)
            .cloned()
            .unwrap_or_else(|| self.state_for_new_task(parent_pid, child_tgid));

        let mut child = SignalState::new(child_pid, child_tgid);
        child.actions = inherited.actions;
        child.blocked = inherited.blocked;
        child.altstack = inherited.altstack;

        if let Some(existing) = self.states.iter_mut().find(|state| state.pid == child_pid) {
            *existing = child;
        } else {
            self.states.push(child);
        }
    }
}

static SIGNAL_TABLE: spin::Mutex<SignalTable> =
    spin::Mutex::new(SignalTable { states: Vec::new() });
#[cfg(test)]
pub(crate) static SIGNAL_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

fn valid_signal(sig: i32) -> bool {
    (1..=NSIG as i32).contains(&sig)
}

fn current_signal_sender() -> (i32, u32) {
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        0
    } else {
        unsafe {
            if (*task).tgid > 0 {
                (*task).tgid
            } else {
                (*task).pid.max(0)
            }
        }
    };
    let cred = crate::kernel::cred::current_cred();
    let uid = if cred.is_null() {
        0
    } else {
        unsafe { (*cred).uid.0 }
    };
    (pid, uid)
}

fn kill_siginfo(sig: i32, code: i32) -> SigInfo {
    let (pid, uid) = current_signal_sender();
    SigInfo::with_sender(sig, code, pid, uid)
}

fn task_for_pid(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() && unsafe { (*current).pid == pid } {
        return current;
    }
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
}

fn task_is_exiting(task: *mut crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() {
        return true;
    }
    let exit_bits =
        crate::kernel::task::task_state::EXIT_ZOMBIE | crate::kernel::task::task_state::EXIT_DEAD;
    unsafe { (*task).m26.exit_state & exit_bits != 0 }
}

fn task_is_stopped_or_traced(task: *mut crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let state = unsafe { (*task).__state.load(core::sync::atomic::Ordering::Acquire) };
    state == crate::kernel::task::task_state::__TASK_STOPPED
        || state == crate::kernel::task::task_state::TASK_STOPPED
        || state == crate::kernel::task::task_state::__TASK_TRACED
}

fn task_wants_signal(
    table: &SignalTable,
    task: *mut crate::kernel::task::TaskStruct,
    sig: i32,
) -> bool {
    if task.is_null() {
        return false;
    }
    let pid = unsafe { (*task).pid };
    let Some(state) = table.states.iter().find(|state| state.pid == pid) else {
        return false;
    };
    if state.blocked.contains(sig) || task_is_exiting(task) {
        return false;
    }
    if sig == SIGKILL {
        return true;
    }
    !task_is_stopped_or_traced(task)
}

fn select_signal_wake_target(
    table: &SignalTable,
    tasks: &[*mut crate::kernel::task::TaskStruct],
    suggested: *mut crate::kernel::task::TaskStruct,
    sig: i32,
) -> *mut crate::kernel::task::TaskStruct {
    if task_wants_signal(table, suggested, sig) {
        return suggested;
    }
    tasks
        .iter()
        .copied()
        .find(|&task| task_wants_signal(table, task, sig))
        .unwrap_or(core::ptr::null_mut())
}

fn push_unique_task(
    tasks: &mut Vec<*mut crate::kernel::task::TaskStruct>,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if task.is_null() || tasks.iter().any(|&existing| existing == task) {
        return;
    }
    tasks.push(task);
}

fn tasks_for_tgid(tgid: i32) -> Vec<*mut crate::kernel::task::TaskStruct> {
    let mut tasks = Vec::new();
    if tgid <= 0 {
        return tasks;
    }

    crate::kernel::fork::for_each_heap_task(|task| unsafe {
        if !task.is_null() && (*task).tgid == tgid {
            push_unique_task(&mut tasks, task);
        }
    });
    crate::kernel::sched::for_each_pool_task(|task| unsafe {
        if !task.is_null() && (*task).tgid == tgid {
            push_unique_task(&mut tasks, task);
        }
    });

    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() && unsafe { (*current).tgid == tgid } {
        push_unique_task(&mut tasks, current);
    }
    tasks
}

fn ensure_task_in_group(
    tasks: &mut Vec<*mut crate::kernel::task::TaskStruct>,
    task: *mut crate::kernel::task::TaskStruct,
    tgid: i32,
) {
    if !task.is_null() && unsafe { (*task).tgid == tgid } {
        push_unique_task(tasks, task);
    }
}

fn is_stop_signal(sig: i32) -> bool {
    matches!(sig, SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU)
}

fn sig_bit(sig: i32) -> u64 {
    if !valid_signal(sig) {
        return 0;
    }
    1u64 << (sig - 1)
}

const KERNEL_ONLY_SIGNAL_MASK: u64 = (1u64 << (SIGKILL - 1)) | (1u64 << (SIGSTOP - 1));

/// Remove signals that Linux never allows user space to catch, block, ignore,
/// or consume through interfaces such as signalfd.
pub const fn user_dequeue_signal_mask(mask: u64) -> u64 {
    mask & !KERNEL_ONLY_SIGNAL_MASK
}

#[inline]
unsafe fn read_user_value<T: Copy>(ptr: *const T) -> Result<T, i64> {
    let mut value = core::mem::MaybeUninit::<T>::uninit();
    let not_copied = unsafe {
        uaccess::copy_from_user(
            value.as_mut_ptr().cast::<u8>(),
            ptr.cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 {
        Ok(unsafe { value.assume_init() })
    } else {
        Err(-14)
    }
}

#[inline]
unsafe fn write_user_value<T: Copy>(ptr: *mut T, value: &T) -> Result<(), i64> {
    let not_copied = unsafe {
        uaccess::copy_to_user(
            ptr.cast::<u8>(),
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(-14) }
}

pub unsafe fn sys_rt_sigaction(
    sig: i32,
    act: *const RtSigAction,
    oldact: *mut RtSigAction,
    sigsetsize: usize,
) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22; // EINVAL
    }
    if !valid_signal(sig) || sig == SIGKILL || sig == SIGSTOP {
        return -22;
    }

    let next = if act.is_null() {
        None
    } else {
        match unsafe { read_user_value(act) } {
            Ok(action) => Some(action),
            Err(e) => return e,
        }
    };
    let old = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(s) => s,
            Err(e) => return e as i64,
        };

        let old = state.actions[sig as usize];
        if let Some(action) = next {
            let tgid = state.tgid;
            for state in table.states.iter_mut().filter(|state| state.tgid == tgid) {
                state.actions[sig as usize] = action;
            }
        }
        old
    };
    if !oldact.is_null() {
        if let Err(e) = unsafe { write_user_value(oldact, &old) } {
            return e;
        }
    }
    0
}

pub unsafe fn sys_rt_sigprocmask(
    how: i32,
    set: *const SigSet,
    oldset: *mut SigSet,
    sigsetsize: usize,
) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22;
    }

    let mut next = if set.is_null() {
        None
    } else {
        match unsafe { read_user_value(set) } {
            Ok(set) => Some(set),
            Err(e) => return e,
        }
    };
    if let Some(set) = next.as_mut() {
        sanitize_blocked_mask(set);
    }

    let task = unsafe { sched::get_current() };
    let (old, blocked) = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(s) => s,
            Err(e) => return e as i64,
        };
        let old = state.blocked;

        if let Some(set) = next {
            match how {
                SIG_BLOCK => state.blocked.bits |= set.bits,
                SIG_UNBLOCK => state.blocked.bits &= !set.bits,
                SIG_SETMASK => state.blocked = set,
                _ => return -22,
            }
        }
        (old, state.blocked)
    };
    recalc_tif_sigpending(task);
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-sigmask pid={} how={} set={} old={:#x} new={:#x}",
            pid,
            how,
            next.map(|set| set.bits).unwrap_or(0),
            old.bits,
            blocked.bits
        );
    }

    if !oldset.is_null() {
        if let Err(e) = unsafe { write_user_value(oldset, &old) } {
            return e;
        }
    }
    0
}

pub unsafe fn sys_rt_sigsuspend(set: *const SigSet, sigsetsize: usize) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22;
    }
    let mut next = match unsafe { read_user_value(set) } {
        Ok(set) => set,
        Err(e) => return e,
    };
    sanitize_blocked_mask(&mut next);

    let task = unsafe { sched::get_current() };
    let mut pending = {
        let mut table = SIGNAL_TABLE.lock();
        let idx = match table.get_or_create_current_index() {
            Ok(idx) => idx,
            Err(e) => return e as i64,
        };
        table.states[idx].sigsuspend_saved = Some(table.states[idx].blocked);
        table.states[idx].blocked = next;
        state_has_unblocked_pending_signal(&table, idx)
    };
    recalc_tif_sigpending(task);

    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-sigsuspend pid={} mask={:#x} pending={}",
            pid,
            next.bits,
            pending
        );
    }

    #[cfg(not(test))]
    while !pending {
        if !task.is_null() {
            unsafe {
                (*task).__state.store(
                    crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                    core::sync::atomic::Ordering::Release,
                );
                crate::kernel::sched::schedule_with_irqs_enabled();
                (*task).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    core::sync::atomic::Ordering::Release,
                );
            }
        } else {
            unsafe {
                crate::kernel::sched::schedule_with_irqs_enabled();
            }
        }
        pending = current_has_unblocked_pending_signals();
    }

    -4
}

pub unsafe fn sys_rt_sigpending(set: *mut SigSet, sigsetsize: usize) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22;
    }
    if set.is_null() {
        return -14;
    }
    let pending = {
        let mut table = SIGNAL_TABLE.lock();
        let idx = match table.get_or_create_current_index() {
            Ok(idx) => idx,
            Err(e) => return e as i64,
        };
        SigSet {
            bits: pending_bits_for_state(&table, &table.states[idx]),
        }
    };
    if let Err(e) = unsafe { write_user_value(set, &pending) } {
        return e;
    }
    0
}

pub unsafe fn sys_rt_sigqueueinfo(pid: i32, sig: i32, uinfo: *const SigInfo) -> i64 {
    let mut info = SigInfo::new(sig, 0);
    if !uinfo.is_null() {
        info = match unsafe { read_user_value(uinfo) } {
            Ok(info) => info,
            Err(e) => return e,
        };
        info.signo = sig;
    }
    if !valid_signal(sig) {
        return -22;
    }
    let target = task_for_pid(pid);
    if target.is_null() {
        return -3;
    }
    send_signal_info_to_process_for_target(target, sig, info) as i64
}

pub unsafe fn sys_tkill(pid: i32, sig: i32) -> i64 {
    if pid <= 0 {
        return -22;
    }
    if sig == 0 {
        let table = SIGNAL_TABLE.lock();
        if table.states.iter().any(|state| state.pid == pid) {
            return 0;
        }
        drop(table);
        return if task_for_pid(pid).is_null() { -3 } else { 0 };
    }
    enqueue_for_pid(pid, sig, kill_siginfo(sig, SI_TKILL))
}

pub unsafe fn sys_tgkill(tgid: i32, pid: i32, sig: i32) -> i64 {
    if pid <= 0 || tgid <= 0 {
        return -22;
    }
    if sig != 0 && !valid_signal(sig) {
        return -22;
    }
    let info = kill_siginfo(sig, SI_TKILL);
    {
        let mut table = SIGNAL_TABLE.lock();
        if let Some(state) = table.get_by_pid_mut(pid) {
            if state.tgid != tgid {
                return -3;
            }
            if sig == 0 {
                return 0;
            }
            queue_signal_info(state, sig, info, false);
            drop(table);
            wake_signal_task_if_live(pid, sig);
            return 0;
        }
    }

    let task = task_for_pid(pid);
    if task.is_null() || unsafe { (*task).tgid } != tgid {
        return -3;
    }
    if sig == 0 {
        return 0;
    }
    queue_signal_for_live_task(task, sig, info);
    wake_signal_task_if_live(pid, sig);
    0
}

fn queue_signal_for_live_task(task: *mut crate::kernel::task::TaskStruct, sig: i32, info: SigInfo) {
    if task.is_null() {
        return;
    }
    let target_pid = unsafe { (*task).pid };
    let target_tgid = unsafe { (*task).tgid };
    let mut table = SIGNAL_TABLE.lock();
    let state_idx = table.get_or_create_task_index(target_pid, target_tgid);
    let state = table.states.get_mut(state_idx).expect("index valid");
    queue_signal_info(state, sig, info, false);
}

fn wake_signal_task_if_live(pid: i32, sig: i32) {
    let task = task_for_pid(pid);
    if task.is_null() {
        return;
    }
    unsafe { wake_signal_task(task, sig) };
}

unsafe fn wake_signal_task(task: *mut crate::kernel::task::TaskStruct, sig: i32) {
    if task.is_null() {
        return;
    }
    let state = unsafe { (*task).__state.load(core::sync::atomic::Ordering::Acquire) };
    set_tif_sigpending(task);
    if state & crate::kernel::task::task_state::TASK_INTERRUPTIBLE != 0
        || state == crate::kernel::task::task_state::TASK_RUNNING
        || sig == SIGKILL
    {
        unsafe {
            let _ = crate::kernel::sched::wake_task(task);
        }
    }
    if sig == SIGKILL {
        wake_fatal_signal_target(task);
    }
}

fn enqueue_for_pid(pid: i32, sig: i32, info: SigInfo) -> i64 {
    if !valid_signal(sig) {
        return -22;
    }
    {
        let mut table = SIGNAL_TABLE.lock();
        if let Some(state) = table.get_by_pid_mut(pid) {
            queue_signal_info(state, sig, info, false);
            drop(table);
            wake_signal_task_if_live(pid, sig);
            return 0;
        }
    }

    let task = task_for_pid(pid);
    if task.is_null() {
        return -3;
    }
    queue_signal_for_live_task(task, sig, info);
    wake_signal_task_if_live(pid, sig);
    0
}

pub unsafe fn sys_rt_sigtimedwait(
    set: *const SigSet,
    info: *mut SigInfo,
    _timeout: *const c_void,
    sigsetsize: usize,
) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22;
    }
    if set.is_null() {
        return -14;
    }
    let wait = match unsafe { read_user_value(set) } {
        Ok(set) => set,
        Err(e) => return e,
    };
    let task = unsafe { sched::get_current() };
    let candidate = {
        let mut table = SIGNAL_TABLE.lock();
        let idx = match table.get_or_create_current_index() {
            Ok(idx) => idx,
            Err(e) => return e as i64,
        };
        let Some(candidate) = dequeue_masked_signal_for_state_index(&mut table, idx, wait.bits)
        else {
            return -11; // EAGAIN
        };
        candidate
    };
    recalc_tif_sigpending(task);
    if !info.is_null() {
        if let Err(e) = unsafe { write_user_value(info, &candidate) } {
            return e;
        }
    }
    candidate.signo as i64
}

pub unsafe fn sys_sigaltstack(new_ss: *const SigAltStack, old_ss: *mut SigAltStack) -> i64 {
    let next = if new_ss.is_null() {
        None
    } else {
        let next = match unsafe { read_user_value(new_ss) } {
            Ok(stack) => stack,
            Err(e) => return e,
        };
        if next.ss_flags & !(SS_DISABLE) != 0 {
            return -22;
        }
        Some(next)
    };

    let old = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(s) => s,
            Err(e) => return e as i64,
        };
        let old = state.altstack;
        if let Some(stack) = next {
            state.altstack = stack;
        }
        old
    };

    if !old_ss.is_null() {
        if let Err(e) = unsafe { write_user_value(old_ss, &old) } {
            return e;
        }
    }
    0
}

#[derive(Clone, Copy)]
enum SignalQueueScope {
    Task,
    Shared,
}

fn task_for_tgid(tgid: i32) -> *mut crate::kernel::task::TaskStruct {
    if tgid <= 0 {
        return core::ptr::null_mut();
    }
    let tasks = tasks_for_tgid(tgid);
    if let Some(leader) = tasks
        .iter()
        .copied()
        .find(|&task| unsafe { (*task).pid == tgid })
    {
        return leader;
    }
    tasks
        .first()
        .copied()
        .unwrap_or_else(|| crate::kernel::sched::find_pool_task_by_tgid(tgid))
}

fn send_signal_info_to_process_for_target(
    target: *mut crate::kernel::task::TaskStruct,
    sig: i32,
    info: SigInfo,
) -> i32 {
    if target.is_null() {
        return -3; // ESRCH
    }
    if !valid_signal(sig) {
        return -22; // EINVAL
    }

    let target_pid = unsafe { (*target).pid };
    let target_tgid = unsafe { (*target).tgid };
    let current = unsafe { crate::kernel::sched::get_current() };
    let stop_now = is_stop_signal(sig) && target != current;
    let mut tasks = tasks_for_tgid(target_tgid);
    ensure_task_in_group(&mut tasks, target, target_tgid);

    let (pending_bits, blocked_bits, ignored, wake_target) = {
        let mut table = SIGNAL_TABLE.lock();
        for &task in tasks.iter() {
            let pid = unsafe { (*task).pid };
            let tgid = unsafe { (*task).tgid };
            table.get_or_create_task_index(pid, tgid);
        }
        let target_idx = table.get_or_create_task_index(target_pid, target_tgid);
        let owner_idx = table
            .states
            .iter()
            .position(|state| state.tgid == target_tgid && state.pid == target_tgid)
            .unwrap_or(target_idx);

        {
            let owner = table.states.get_mut(owner_idx).expect("owner index valid");
            if is_stop_signal(sig) {
                owner.remove_signal(SIGCONT);
            } else if sig == SIGCONT {
                owner.remove_signal_mask(
                    sig_bit(SIGSTOP) | sig_bit(SIGTSTP) | sig_bit(SIGTTIN) | sig_bit(SIGTTOU),
                );
            }
        }

        let ignored = {
            let owner = table.states.get(owner_idx).expect("owner index valid");
            !stop_now && signal_ignored_for_state(owner, sig)
        };
        if !stop_now && !ignored {
            let owner = table.states.get_mut(owner_idx).expect("owner index valid");
            queue_signal_info(owner, sig, info, true);
        }
        let pending_bits = pending_bits_for_state(&table, &table.states[owner_idx]);
        let blocked_bits = table.states[owner_idx].blocked.bits;
        let wake_target = if ignored {
            core::ptr::null_mut()
        } else {
            select_signal_wake_target(&table, &tasks, target, sig)
        };
        (pending_bits, blocked_bits, ignored, wake_target)
    };

    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-signal-send target={} sig={} pending={:#x} blocked={:#x} stop_now={}",
            target_pid,
            sig,
            pending_bits,
            blocked_bits,
            stop_now
        );
    }
    if ignored {
        return 0;
    }

    if sig == SIGCONT {
        unsafe {
            let state = (*target)
                .__state
                .load(core::sync::atomic::Ordering::Acquire);
            if state == crate::kernel::task::task_state::__TASK_STOPPED
                || state == crate::kernel::task::task_state::TASK_STOPPED
            {
                (*target).m26.ptrace_stop_signal = 0;
                (*target).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    core::sync::atomic::Ordering::Release,
                );
            }
        }
    }

    if stop_now {
        unsafe {
            (*target).m26.ptrace_stop_signal = sig;
            (*target).__state.store(
                crate::kernel::task::task_state::__TASK_STOPPED,
                core::sync::atomic::Ordering::Release,
            );
            let parent = (*target).m26.real_parent;
            if !parent.is_null() {
                let _ = send_signal_to_task(parent, SIGCHLD);
            }
            wake_waiters(target);
        }
        return 0;
    }

    if !wake_target.is_null() {
        unsafe { wake_signal_task(wake_target, sig) };
    }
    0
}

/// Queue a process-directed signal to a thread group.
///
/// Linux enqueues `ITIMER_REAL` SIGALRM on `signal_struct::shared_pending`
/// via `kill_pid_info(..., PIDTYPE_TGID)`. Lupos stores that bit on the group
/// leader's signal state and makes every thread in the tgid consult it, matching
/// Linux's shared-pending visibility without a separate `signal_struct` table.
pub fn send_signal_to_process(tgid: i32, sig: i32) -> i32 {
    if !valid_signal(sig) {
        return -22; // EINVAL
    }
    let target = task_for_tgid(tgid);
    if target.is_null() {
        return -3; // ESRCH
    }
    send_signal_info_to_process_for_target(target, sig, SigInfo::new(sig, 0))
}

/// Queue a process-directed signal generated by the calling userspace task.
/// Unlike kernel-originated process signals, kill(2) must expose SI_USER and
/// the caller's PID/UID through siginfo_t.
pub fn send_user_signal_to_process(tgid: i32, sig: i32) -> i32 {
    if !valid_signal(sig) {
        return -22; // EINVAL
    }
    let target = task_for_tgid(tgid);
    if target.is_null() {
        return -3; // ESRCH
    }
    send_signal_info_to_process_for_target(target, sig, kill_siginfo(sig, SI_USER))
}

/// Queue `sig` for delivery to `target` and raise `TIF_SIGPENDING`.
///
/// M26 helper used by `exit_notify` (SIGCHLD to parent) and
/// `ptrace::ptrace_attach` (SIGSTOP to tracee).  Returns 0 on success or
/// a negative errno on failure.
///
/// # Safety
/// `target` must be a valid `*mut TaskStruct`.
pub unsafe fn send_signal_to_task(target: *mut crate::kernel::task::TaskStruct, sig: i32) -> i32 {
    unsafe { send_signal_to_task_scoped(target, sig, SignalQueueScope::Task) }
}

unsafe fn send_signal_to_task_scoped(
    target: *mut crate::kernel::task::TaskStruct,
    sig: i32,
    scope: SignalQueueScope,
) -> i32 {
    if target.is_null() {
        return -3; // ESRCH
    }
    if !valid_signal(sig) {
        return -22; // EINVAL
    }
    if matches!(scope, SignalQueueScope::Shared) {
        return send_signal_info_to_process_for_target(target, sig, SigInfo::new(sig, 0));
    }
    let target_pid = unsafe { (*target).pid };
    let target_tgid = unsafe { (*target).tgid };
    let current = unsafe { crate::kernel::sched::get_current() };
    let stop_now = is_stop_signal(sig) && target != current;

    // Ensure a SignalState exists for the target — create one on demand,
    // mirroring the lazy registration in `get_or_create_current`.
    let mut table = SIGNAL_TABLE.lock();
    let state_idx = table.get_or_create_task_index(target_pid, target_tgid);
    let state = table.states.get_mut(state_idx).expect("index valid");
    // Linux `prepare_signal()` applies these job-control side effects at
    // signal generation time, regardless of blocking or default disposition.
    if is_stop_signal(sig) {
        state.remove_signal(SIGCONT);
    } else if sig == SIGCONT {
        state.remove_signal_mask(
            sig_bit(SIGSTOP) | sig_bit(SIGTSTP) | sig_bit(SIGTTIN) | sig_bit(SIGTTOU),
        );
    }
    let ignored = !stop_now && signal_ignored_for_state(state, sig);
    let (pending_bits, blocked_bits) = if !stop_now && !ignored {
        queue_signal_info(state, sig, SigInfo::new(sig, 0), false);
        (
            state.pending.bits | state.shared_pending.bits,
            state.blocked.bits,
        )
    } else {
        (
            state.pending.bits | state.shared_pending.bits,
            state.blocked.bits,
        )
    };
    drop(table);

    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-signal-send target={} sig={} pending={:#x} blocked={:#x} stop_now={}",
            target_pid,
            sig,
            pending_bits,
            blocked_bits,
            stop_now
        );
    }
    if ignored {
        return 0;
    }

    if sig == SIGCONT {
        unsafe {
            let state = (*target)
                .__state
                .load(core::sync::atomic::Ordering::Acquire);
            if state == crate::kernel::task::task_state::__TASK_STOPPED
                || state == crate::kernel::task::task_state::TASK_STOPPED
            {
                (*target).m26.ptrace_stop_signal = 0;
                (*target).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    core::sync::atomic::Ordering::Release,
                );
            }
        }
    }

    if stop_now {
        unsafe {
            (*target).m26.ptrace_stop_signal = sig;
            (*target).__state.store(
                crate::kernel::task::task_state::__TASK_STOPPED,
                core::sync::atomic::Ordering::Release,
            );
            let parent = (*target).m26.real_parent;
            if !parent.is_null() {
                let _ = send_signal_to_task(parent, SIGCHLD);
            }
            wake_waiters(target);
        }
        return 0;
    }

    // Raise TIF_SIGPENDING so the target sees the signal on its next
    // syscall/exception exit.
    unsafe { wake_signal_task(target, sig) };
    0
}

/// Queue a signal with caller-supplied `siginfo_t` for synchronous faults.
///
/// Linux queues precise `siginfo` for faults such as machine-check SIGBUS.
/// Unlike normal non-RT signals, this must preserve `si_code` and `si_addr`,
/// so the entry is kept in `rt_queue` even when the signal number is below
/// `SIGRTMIN`.
///
/// # Safety
/// `target` must be a valid `*mut TaskStruct`.
pub unsafe fn send_signal_info_to_task(
    target: *mut crate::kernel::task::TaskStruct,
    info: SigInfo,
) -> i32 {
    if target.is_null() {
        return -3; // ESRCH
    }
    if !valid_signal(info.signo) {
        return -22; // EINVAL
    }
    let target_pid = unsafe { (*target).pid };
    let target_tgid = unsafe { (*target).tgid };
    let mut table = SIGNAL_TABLE.lock();
    let state_idx = table.get_or_create_task_index(target_pid, target_tgid);
    let state = table.states.get_mut(state_idx).expect("index valid");
    queue_signal_info(state, info.signo, info, false);
    drop(table);
    set_tif_sigpending(target);
    0
}

/// Queue `sig` for all tasks in process group `pgrp`.
///
/// This is the small login-console bridge used by the fd-backed console TTY. Linux
/// routes terminal-generated signals through tty job-control helpers and PID
/// indexes; Lupos keeps the same visible behavior with the current heap-task
/// tracker until those indexes land.
pub fn send_signal_to_process_group(pgrp: i32, sig: i32) -> i32 {
    if pgrp <= 0 {
        return -3;
    }
    if sig != 0 && !valid_signal(sig) {
        return -22;
    }
    let mut sent = 0i32;
    let current = unsafe { crate::kernel::sched::get_current() };
    let mut saw_current_in_heap = false;
    crate::kernel::fork::for_each_heap_task(|task| {
        if task == current {
            saw_current_in_heap = true;
        }
        let pid = unsafe { (*task).pid };
        if crate::kernel::session::process_group(pid).unwrap_or(pid) == pgrp {
            if sig == 0 || unsafe { send_signal_to_task(task, sig) } == 0 {
                sent += 1;
            }
        }
    });
    if !current.is_null() && !saw_current_in_heap {
        let pid = unsafe { (*current).pid };
        if crate::kernel::session::process_group(pid).unwrap_or(pid) == pgrp
            && (sig == 0 || unsafe { send_signal_to_task(current, sig) } == 0)
        {
            sent += 1;
        }
    }
    if sent == 0 { -3 } else { 0 }
}

/// User-generated process-group signal used by kill(2). Unlike tty/job-control
/// generation, every target must observe SI_USER and the caller's credentials.
pub fn send_user_signal_to_process_group(pgrp: i32, sig: i32) -> i32 {
    if pgrp <= 0 {
        return -3;
    }
    if sig != 0 && !valid_signal(sig) {
        return -22;
    }
    let current = unsafe { crate::kernel::sched::get_current() };
    let mut sent_tgids = Vec::new();
    let mut targets = Vec::new();
    let mut collect = |task: *mut crate::kernel::task::TaskStruct| {
        if task.is_null() {
            return;
        }
        let pid = unsafe { (*task).pid };
        let tgid = unsafe { if (*task).tgid > 0 { (*task).tgid } else { pid } };
        if crate::kernel::session::process_group(pid).unwrap_or(pid) != pgrp
            || sent_tgids.contains(&tgid)
        {
            return;
        }
        sent_tgids.push(tgid);
        targets.push(task);
    };
    // for_each_heap_task holds HEAP_TASKS across its callback. Collect raw
    // targets first, then deliver after that lock is released because process
    // delivery scans the tracker again to find thread-group siblings.
    crate::kernel::fork::for_each_heap_task(&mut collect);
    collect(current);
    let mut sent = 0i32;
    for task in targets {
        if sig == 0
            || send_signal_info_to_process_for_target(task, sig, kill_siginfo(sig, SI_USER)) == 0
        {
            sent += 1;
        }
    }
    if sent == 0 { -3 } else { 0 }
}

unsafe fn wake_waiters(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    let count = unsafe { (*task).m26.wait_count as usize };
    for idx in 0..count.min(crate::kernel::task::MAX_WAITERS) {
        let waiter = unsafe { (*task).m26.wait_waiters[idx] };
        if !waiter.is_null() {
            unsafe {
                (*waiter).__state.store(
                    crate::kernel::task::task_state::TASK_RUNNING,
                    core::sync::atomic::Ordering::Release,
                );
            }
        }
    }
}

unsafe fn stop_current_for_signal(task: *mut crate::kernel::task::TaskStruct, sig: i32) -> bool {
    if task.is_null() {
        return false;
    }
    unsafe {
        (*task).m26.ptrace_stop_signal = sig;
        (*task).__state.store(
            crate::kernel::task::task_state::__TASK_STOPPED,
            core::sync::atomic::Ordering::Release,
        );
        wake_waiters(task);
        crate::kernel::sched::schedule_with_irqs_enabled();
        (*task).__state.store(
            crate::kernel::task::task_state::TASK_RUNNING,
            core::sync::atomic::Ordering::Release,
        );
        (*task).m26.ptrace_stop_signal = 0;
    }
    true
}

/// Check whether `pid` has `sig` queued in its pending mask.
///
/// Used by host unit tests to verify SIGCHLD delivery and by
/// `process_mrelease(2)` to mirror Linux's "fatal signal pending" gate.
pub fn has_pending_signal_for_pid(pid: i32, sig: i32) -> bool {
    let table = SIGNAL_TABLE.lock();
    if let Some(idx) = table.states.iter().position(|s| s.pid == pid) {
        return pending_bits_for_state(&table, &table.states[idx]) & sig_bit(sig) != 0;
    }
    false
}

/// Set the TIF_SIGPENDING flag on a task to indicate pending signals.
///
/// Called when enqueueing a signal to a task that might be sleeping.
/// The flag is checked on syscall exit and exception return.
pub fn set_tif_sigpending(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        let thread_info = &mut (*task).thread_info;
        thread_info.flags |= crate::kernel::task::TIF_SIGPENDING;
        let state = (*task).__state.load(core::sync::atomic::Ordering::Acquire);
        if state & crate::kernel::task::task_state::TASK_INTERRUPTIBLE != 0 {
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                core::sync::atomic::Ordering::Release,
            );
            let _ = crate::kernel::sched::wake_task(task);
        }
    }
}

fn wake_fatal_signal_target(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        let state = (*task).__state.load(core::sync::atomic::Ordering::Acquire);
        let wake_mask = crate::kernel::task::task_state::TASK_INTERRUPTIBLE
            | crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE
            | crate::kernel::task::task_state::__TASK_STOPPED
            | crate::kernel::task::task_state::__TASK_TRACED
            | crate::kernel::task::task_state::TASK_WAKEKILL;
        if state & wake_mask != 0 {
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                core::sync::atomic::Ordering::Release,
            );
        }
    }
}

/// Check if the task has pending signals that need delivery.
pub fn has_pending_signals(task: *const crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    unsafe {
        let thread_info = &(*task).thread_info;
        (thread_info.flags & crate::kernel::task::TIF_SIGPENDING) != 0
    }
}

fn group_shared_pending_bits(table: &SignalTable, tgid: i32) -> u64 {
    table
        .states
        .iter()
        .filter(|state| state.tgid == tgid)
        .fold(0, |bits, state| bits | state.shared_pending.bits)
}

fn pending_bits_for_state(table: &SignalTable, state: &SignalState) -> u64 {
    state.pending.bits | group_shared_pending_bits(table, state.tgid)
}

fn first_matching_signal(
    state: &SignalState,
    bits: u64,
    mut pred: impl FnMut(&SignalState, i32) -> bool,
) -> Option<i32> {
    for sig in 1..=NSIG as i32 {
        if bits & sig_bit(sig) != 0 && pred(state, sig) {
            return Some(sig);
        }
    }
    None
}

fn select_pending_signal(
    state: &SignalState,
    task_bits: u64,
    shared_bits: u64,
    mut pred: impl FnMut(&SignalState, i32) -> bool,
) -> Option<(i32, SignalQueueScope)> {
    for sig in 1..=NSIG as i32 {
        if !pred(state, sig) {
            continue;
        }
        if task_bits & sig_bit(sig) != 0 {
            return Some((sig, SignalQueueScope::Task));
        }
        if shared_bits & sig_bit(sig) != 0 {
            return Some((sig, SignalQueueScope::Shared));
        }
    }
    None
}

fn take_group_shared_signal_info(table: &mut SignalTable, tgid: i32, sig: i32) -> SigInfo {
    let mut info = None;
    for state in table.states.iter_mut().filter(|state| state.tgid == tgid) {
        if !state.shared_pending.contains(sig) {
            continue;
        }
        if let Some(idx) = state
            .shared_queue
            .iter()
            .position(|queued| queued.signo == sig)
        {
            info = state.shared_queue.remove(idx);
            break;
        }
    }
    let remains = table
        .states
        .iter()
        .filter(|state| state.tgid == tgid)
        .any(|state| state.shared_queue.iter().any(|queued| queued.signo == sig));
    if !remains {
        for state in table.states.iter_mut().filter(|state| state.tgid == tgid) {
            state.shared_pending.remove(sig);
        }
    }
    info.unwrap_or_else(|| SigInfo::new(sig, 0))
}

fn state_has_unblocked_pending_signal(table: &SignalTable, idx: usize) -> bool {
    let Some(state) = table.states.get(idx) else {
        return false;
    };
    select_pending_signal(
        state,
        state.pending.bits,
        group_shared_pending_bits(table, state.tgid),
        signal_unblocked_for_state,
    )
    .is_some()
}

fn dequeue_unblocked_signal_for_state_index(
    table: &mut SignalTable,
    idx: usize,
) -> Option<SigInfo> {
    let tgid = table.states.get(idx)?.tgid;
    let shared_bits = group_shared_pending_bits(table, tgid);
    let (sig, scope) = {
        let state = table.states.get(idx)?;
        select_pending_signal(
            state,
            state.pending.bits,
            shared_bits,
            signal_unblocked_for_state,
        )
    }?;
    match scope {
        SignalQueueScope::Task => Some(take_task_signal_info(table.states.get_mut(idx)?, sig)),
        SignalQueueScope::Shared => Some(take_group_shared_signal_info(table, tgid, sig)),
    }
}

fn dequeue_masked_signal_for_state_index(
    table: &mut SignalTable,
    idx: usize,
    mask: u64,
) -> Option<SigInfo> {
    let mask = user_dequeue_signal_mask(mask);
    if mask == 0 {
        return None;
    }
    let tgid = table.states.get(idx)?.tgid;

    let shared_bits = group_shared_pending_bits(table, tgid) & mask;
    let (sig, scope) = {
        let state = table.states.get(idx)?;
        select_pending_signal(state, state.pending.bits & mask, shared_bits, |_, _| true)
    }?;
    match scope {
        SignalQueueScope::Task => Some(take_task_signal_info(table.states.get_mut(idx)?, sig)),
        SignalQueueScope::Shared => Some(take_group_shared_signal_info(table, tgid, sig)),
    }
}

fn dequeue_default_fatal_signal_for_state_index(
    table: &mut SignalTable,
    idx: usize,
) -> Option<SigInfo> {
    let tgid = table.states.get(idx)?.tgid;
    let shared_bits = group_shared_pending_bits(table, tgid);
    let (sig, scope) = {
        let state = table.states.get(idx)?;
        select_pending_signal(
            state,
            state.pending.bits,
            shared_bits,
            signal_is_default_fatal,
        )
    }?;
    match scope {
        SignalQueueScope::Task => Some(take_task_signal_info(table.states.get_mut(idx)?, sig)),
        SignalQueueScope::Shared => Some(take_group_shared_signal_info(table, tgid, sig)),
    }
}

pub fn has_unblocked_pending_signals(task: *const crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() || !has_pending_signals(task) {
        return false;
    }
    task_has_unblocked_pending_signal_state(task)
}

fn task_has_unblocked_pending_signal_state(task: *const crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let pid = unsafe { (*task).pid };
    let table = SIGNAL_TABLE.lock();
    table
        .states
        .iter()
        .position(|state| state.pid == pid)
        .is_some_and(|idx| state_has_unblocked_pending_signal(&table, idx))
}

fn recalc_tif_sigpending(task: *mut crate::kernel::task::TaskStruct) -> bool {
    if task.is_null() {
        return false;
    }
    let pending = task_has_unblocked_pending_signal_state(task);
    unsafe {
        if pending {
            (*task).thread_info.flags |= crate::kernel::task::TIF_SIGPENDING;
        } else {
            (*task).thread_info.flags &= !crate::kernel::task::TIF_SIGPENDING;
        }
    }
    pending
}

pub fn current_has_unblocked_pending_signals() -> bool {
    let task = unsafe { crate::kernel::sched::get_current() };
    has_unblocked_pending_signals(task)
}

pub fn current_has_pending_signals() -> bool {
    let task = unsafe { crate::kernel::sched::get_current() };
    has_pending_signals(task)
}

pub fn has_current_pending_signal_mask(mask: u64) -> bool {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return false;
    }
    let pid = unsafe { (*task).pid };
    let table = SIGNAL_TABLE.lock();
    table
        .states
        .iter()
        .position(|state| state.pid == pid)
        .is_some_and(|state| {
            (pending_bits_for_state(&table, &table.states[state]) & user_dequeue_signal_mask(mask))
                != 0
        })
}

pub fn current_pending_signal_bits() -> u64 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return 0;
    }
    let pid = unsafe { (*task).pid };
    let table = SIGNAL_TABLE.lock();
    table
        .states
        .iter()
        .position(|state| state.pid == pid)
        .map(|idx| pending_bits_for_state(&table, &table.states[idx]))
        .unwrap_or(0)
}

pub fn dequeue_current_pending_signal_mask(mask: u64) -> Option<SigInfo> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return None;
    }

    let info = {
        let mut table = SIGNAL_TABLE.lock();
        let idx = table.get_or_create_current_index().ok()?;
        dequeue_masked_signal_for_state_index(&mut table, idx, mask)?
    };

    recalc_tif_sigpending(task);
    Some(info)
}

/// Clear the TIF_SIGPENDING flag after all pending signals have been delivered.
pub fn clear_tif_sigpending(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        let thread_info = &mut (*task).thread_info;
        thread_info.flags &= !crate::kernel::task::TIF_SIGPENDING;
    }
}

fn restore_current_blocked_mask(mut mask: SigSet) {
    sanitize_blocked_mask(&mut mask);
    let task = unsafe { crate::kernel::sched::get_current() };
    let mut table = SIGNAL_TABLE.lock();
    if let Ok(state) = table.get_or_create_current() {
        state.blocked = mask;
    }
    drop(table);
    recalc_tif_sigpending(task);
}

fn signal_is_default_fatal(state: &SignalState, sig: i32) -> bool {
    if sig == SIGKILL {
        return true;
    }
    if !valid_signal(sig) || state.blocked.contains(sig) {
        return false;
    }
    matches!(
        handler_kind(&state.actions[sig as usize]),
        HandlerKind::Default
    ) && matches!(
        default_action(sig),
        DefaultAction::Term | DefaultAction::Core
    )
}

fn signal_ignored_for_state(state: &SignalState, sig: i32) -> bool {
    if !valid_signal(sig) || state.blocked.contains(sig) {
        return false;
    }
    match handler_kind(&state.actions[sig as usize]) {
        HandlerKind::Ignore => true,
        HandlerKind::Default => matches!(default_action(sig), DefaultAction::Ign),
        HandlerKind::User(_) => false,
    }
}

fn signal_unblocked_for_state(state: &SignalState, sig: i32) -> bool {
    valid_signal(sig) && !state.blocked.contains(sig)
}

fn signal_deliverable_for_state(state: &SignalState, sig: i32) -> bool {
    signal_unblocked_for_state(state, sig) && !signal_ignored_for_state(state, sig)
}

/// Consume a pending terminating signal for the current task without
/// delivering a user signal frame. Blocking syscall loops use this before
/// sleeping again so service timeout kills take effect promptly. Linux's
/// `complete_signal()` turns unblocked default-fatal signals into group-exit
/// wakeups; until Lupos grows that group-exit state, consume the same class of
/// signals here while preserving user handlers and ignored/stopped defaults.
pub fn take_current_fatal_signal() -> Option<i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() || !has_pending_signals(task) {
        return None;
    }

    let (signal, still_pending) = {
        let mut table = SIGNAL_TABLE.lock();
        let idx = match table.get_or_create_current_index() {
            Ok(idx) => idx,
            Err(_) => return None,
        };
        let signal = dequeue_default_fatal_signal_for_state_index(&mut table, idx).map(|info| {
            sigqueue_release(info.signo);
            info.signo
        });
        let still_pending = state_has_unblocked_pending_signal(&table, idx);
        (signal, still_pending)
    };

    if !still_pending {
        clear_tif_sigpending(task);
    }
    signal
}

pub unsafe fn exit_current_for_signal(sig: i32) -> ! {
    unsafe { crate::kernel::exit::do_exit(crate::kernel::wait::w_exitcode(0, sig) as i64) }
}

pub unsafe fn exit_if_fatal_signal_pending_current() {
    if let Some(sig) = take_current_fatal_signal() {
        unsafe { exit_current_for_signal(sig) };
    }
}

/// Minimal syscall-exit signal handling used by the current user-mode path.
///
/// This deliberately handles only the stop/terminate cases the kernel already
/// models correctly. Other pending signals are drained and the thread flag is
/// cleared, matching the pre-Phase-16 behavior without routing through the
/// still-incomplete user signal-frame setup.
pub unsafe fn do_signal_stop_only() -> bool {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return false;
    }

    let (signal_info, saved_mask) = {
        let mut table = SIGNAL_TABLE.lock();
        if let Ok(idx) = table.get_or_create_current_index() {
            let saved_mask = table.states[idx].sigsuspend_saved.take();
            (
                dequeue_unblocked_signal_for_state_index(&mut table, idx),
                saved_mask,
            )
        } else {
            (None, None)
        }
    };

    let Some(info) = signal_info else {
        clear_tif_sigpending(task);
        return false;
    };
    if let Some(mask) = saved_mask {
        restore_current_blocked_mask(mask);
    }

    match info.signo {
        SIGKILL | SIGINT => unsafe {
            crate::kernel::exit::do_exit(crate::kernel::wait::w_exitcode(0, info.signo) as i64)
        },
        SIGSTOP | SIGTSTP => unsafe { stop_current_for_signal(task, info.signo) },
        _ => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                let pid = unsafe { (*task).pid };
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-signal-drain pid={} sig={}",
                    pid,
                    info.signo
                );
            }
            recalc_tif_sigpending(task);
            true
        }
    }
}

// ── Default action table ─────────────────────────────────────────────────────
//
// Mirrors Linux `kernel/signal.c::sig_handler_ignored` / `sig_kernel_*` macros
// plus the `__sig_kernel_table` lookup that picks the per-signal default.
//
// Reference: vendor/linux/include/linux/signal.h
//            vendor/linux/kernel/signal.c (sig_kernel_only / sig_kernel_coredump /
//            sig_kernel_ignore / sig_kernel_stop)

/// What the kernel does when a signal arrives at a task whose
/// `sa_handler == SIG_DFL`.  Linux: `enum siginfo_layout` derived from the
/// `__sig_kernel_*` macros.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultAction {
    /// Terminate the process.  Linux: `SIG_DFL_TERM`.
    Term,
    /// Terminate and produce a core dump.  Linux: `SIG_DFL_CORE`.
    Core,
    /// Ignore the signal.  Linux: `SIG_DFL_IGN`.
    Ign,
    /// Stop the process.  Linux: `SIG_DFL_STOP`.
    Stop,
    /// Continue the process if stopped.  Linux: `SIG_DFL_CONT`.
    Cont,
}

/// Look up the per-signal default action.  Linux:
/// `sig_handler_ignored` / `sig_kernel_coredump` / `sig_kernel_stop`.
pub const fn default_action(sig: i32) -> DefaultAction {
    match sig {
        // Coredump signals — terminate and produce a core dump.
        SIGQUIT | SIGILL | SIGTRAP | SIGABRT | SIGBUS | SIGFPE | SIGSEGV | SIGXCPU | SIGXFSZ
        | SIGSYS => DefaultAction::Core,
        // Stop signals.
        SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU => DefaultAction::Stop,
        // Continue signal.
        SIGCONT => DefaultAction::Cont,
        // Ignored by default.
        SIGCHLD | SIGURG | SIGWINCH => DefaultAction::Ign,
        // Everything else terminates.
        _ => DefaultAction::Term,
    }
}

/// True if `sig` cannot be caught, blocked, or ignored.  Linux:
/// `sig_kernel_only` covers SIGKILL/SIGSTOP.
pub const fn is_kernel_only(sig: i32) -> bool {
    sig == SIGKILL || sig == SIGSTOP
}

/// Resolve the standard sa_handler magic values.  `0 → SIG_DFL`,
/// `1 → SIG_IGN`.  Anything else is a user-installed handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HandlerKind {
    Default,
    Ignore,
    User(usize),
}

fn handler_kind(action: &RtSigAction) -> HandlerKind {
    match action.sa_handler {
        0 => HandlerKind::Default,
        1 => HandlerKind::Ignore,
        h => HandlerKind::User(h),
    }
}

#[inline]
fn neg_errno(errno: i32) -> u64 {
    (-(errno as i64)) as u64
}

fn syscall_restart_error(regs: &crate::kernel::task::PtRegs) -> Option<i32> {
    if regs.orig_ax == NO_SYSCALL {
        return None;
    }

    match regs.ax {
        value if value == neg_errno(ERESTARTSYS) => Some(ERESTARTSYS),
        value if value == neg_errno(ERESTARTNOINTR) => Some(ERESTARTNOINTR),
        value if value == neg_errno(ERESTARTNOHAND) => Some(ERESTARTNOHAND),
        value if value == neg_errno(ERESTART_RESTARTBLOCK) => Some(ERESTART_RESTARTBLOCK),
        _ => None,
    }
}

fn rewind_syscall(regs: &mut crate::kernel::task::PtRegs) {
    regs.ip = regs.ip.wrapping_sub(X86_64_SYSCALL_INSN_LEN);
}

fn restart_original_syscall(regs: &mut crate::kernel::task::PtRegs) {
    regs.ax = regs.orig_ax;
    rewind_syscall(regs);
}

fn apply_syscall_restart_for_handler(regs: &mut crate::kernel::task::PtRegs, action: &RtSigAction) {
    match syscall_restart_error(regs) {
        Some(ERESTART_RESTARTBLOCK) | Some(ERESTARTNOHAND) => {
            regs.ax = neg_errno(EINTR);
        }
        Some(ERESTARTSYS) if action.sa_flags & SA_RESTART == 0 => {
            regs.ax = neg_errno(EINTR);
        }
        Some(ERESTARTSYS) | Some(ERESTARTNOINTR) => restart_original_syscall(regs),
        _ => {}
    }
}

fn apply_syscall_restart_without_handler(regs: &mut crate::kernel::task::PtRegs) {
    match syscall_restart_error(regs) {
        Some(ERESTARTNOHAND) | Some(ERESTARTSYS) | Some(ERESTARTNOINTR) => {
            restart_original_syscall(regs);
        }
        Some(ERESTART_RESTARTBLOCK) => {
            regs.ax = X86_64_NR_RESTART_SYSCALL;
            rewind_syscall(regs);
        }
        _ => {}
    }
}

/// Deliver pending signals to the current task.
///
/// Called from the syscall exit path when TIF_SIGPENDING is set.
/// Dequeues one unblocked signal, sets up the signal frame, and modifies the
/// return registers to jump to the signal handler.
///
/// Returns true if a signal was delivered, false if no unblocked signals remain.
///
/// Reference: vendor/linux/kernel/signal.c::get_signal +
///            vendor/linux/arch/x86/kernel/signal.c::arch_do_signal_or_restart
///
/// # Safety
/// Must be called with the current task context valid and interrupts disabled.
pub unsafe fn do_signal(regs: *mut crate::kernel::task::PtRegs) -> bool {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return false;
    }

    loop {
        let (signal_info, saved_mask, mask_to_save) = {
            let mut table = SIGNAL_TABLE.lock();
            if let Ok(idx) = table.get_or_create_current_index() {
                let saved_mask = table.states[idx].sigsuspend_saved.take();
                let mask_to_save = saved_mask.unwrap_or(table.states[idx].blocked);
                (
                    dequeue_unblocked_signal_for_state_index(&mut table, idx),
                    saved_mask,
                    mask_to_save,
                )
            } else {
                (None, None, SigSet::default())
            }
        };

        let Some(info) = signal_info else {
            if let Some(mask) = saved_mask {
                restore_current_blocked_mask(mask);
            }
            clear_tif_sigpending(task);
            if let Some(regs_mut) = unsafe { regs.as_mut() } {
                apply_syscall_restart_without_handler(regs_mut);
            }
            return false;
        };
        if info.signo == SIGALRM {
            let key = unsafe {
                if (*task).tgid > 0 {
                    (*task).tgid
                } else {
                    (*task).pid
                }
            };
            crate::kernel::syscalls::rearm_real_itimer_after_sigalrm(key);
        }
        if let Some(mask) = saved_mask {
            restore_current_blocked_mask(mask);
        }

        // Release the rlimit charge held by the queue entry, if any.
        sigqueue_release(info.signo);

        let action = {
            let mut table = SIGNAL_TABLE.lock();
            if let Ok(state) = table.get_or_create_current() {
                state.actions[info.signo as usize].clone()
            } else {
                RtSigAction::default()
            }
        };

        // Linux: signals fatal-by-design (SIGKILL) bypass the handler lookup —
        // they always terminate via the default-action path.
        let kind = if info.signo == SIGKILL {
            HandlerKind::Default
        } else {
            handler_kind(&action)
        };
        match kind {
            HandlerKind::Ignore => {
                // Loop to drain the next unblocked signal.
                recalc_tif_sigpending(task);
                continue;
            }
            HandlerKind::Default => match default_action(info.signo) {
                DefaultAction::Ign => {
                    recalc_tif_sigpending(task);
                    continue;
                }
                DefaultAction::Term => unsafe {
                    crate::kernel::exit::do_exit(
                        crate::kernel::wait::w_exitcode(0, info.signo) as i64
                    );
                },
                DefaultAction::Core => unsafe {
                    // Drive the coredump pipeline before terminating.
                    do_coredump(task, &info);
                    crate::kernel::exit::do_exit(crate::kernel::wait::w_exitcode(
                        0,
                        info.signo | 0x80,
                    ) as i64);
                },
                DefaultAction::Stop => {
                    recalc_tif_sigpending(task);
                    return unsafe { stop_current_for_signal(task, info.signo) };
                }
                DefaultAction::Cont => {
                    unsafe {
                        (*task).__state.store(
                            crate::kernel::task::task_state::TASK_RUNNING,
                            core::sync::atomic::Ordering::Release,
                        );
                    }
                    recalc_tif_sigpending(task);
                    return true;
                }
            },
            HandlerKind::User(_) => {
                if let Some(regs_mut) = unsafe { regs.as_mut() } {
                    apply_syscall_restart_for_handler(regs_mut, &action);
                }
                if unsafe {
                    crate::arch::x86::kernel::signal::setup_rt_frame(
                        regs,
                        info.signo,
                        &action,
                        &info,
                        mask_to_save,
                    )
                }
                .is_err()
                {
                    // Stack-overflow / EFAULT during frame setup forces SIGSEGV.
                    // Linux: `force_sigsegv` resets the handler to SIG_DFL and
                    // re-raises SIGSEGV.
                    let _ = unsafe { force_sigsegv(task) };
                    return false;
                }
                // Linux merges sa_mask (and the delivered signal unless
                // SA_NODEFER is set) into the task's blocked mask.
                {
                    let mut table = SIGNAL_TABLE.lock();
                    if let Ok(state) = table.get_or_create_current() {
                        state.blocked.bits |= action.sa_mask.bits;
                        if action.sa_flags & SA_NODEFER == 0 {
                            state.blocked.add(info.signo);
                        }
                        // SIGKILL/SIGSTOP never blocked.
                        state.blocked.remove(SIGKILL);
                        state.blocked.remove(SIGSTOP);
                        // SA_RESETHAND restores SIG_DFL after one delivery.
                        if action.sa_flags & SA_RESETHAND != 0 {
                            state.actions[info.signo as usize] = RtSigAction::default();
                        }
                    }
                }
                recalc_tif_sigpending(task);
                return true;
            }
        }
    }
}

/// Force SIGSEGV on `task` with SIG_DFL.  Linux: `force_sigsegv` in
/// kernel/signal.c.
///
/// # Safety
/// `task` must be a valid TaskStruct.
unsafe fn force_sigsegv(task: *mut crate::kernel::task::TaskStruct) -> i32 {
    let mut table = SIGNAL_TABLE.lock();
    if let Ok(state) = table.get_or_create_current() {
        state.actions[SIGSEGV as usize] = RtSigAction::default();
        state.blocked.remove(SIGSEGV);
        state.pending.add(SIGSEGV);
    }
    drop(table);
    unsafe { set_tif_sigpending(task) };
    0
}

/// Restore the user-mode register set from the signal frame on the user
/// stack.  Linux: `sys_rt_sigreturn` /
/// `arch/x86/kernel/signal.c::restore_sigcontext`.
///
/// # Safety
/// Must be invoked from the syscall entry path with `regs` pointing at the
/// current task's saved `PtRegs`.
pub unsafe fn sys_rt_sigreturn_impl(regs: *mut crate::kernel::task::PtRegs) -> i64 {
    use crate::arch::x86::kernel::signal::SigContext;
    if regs.is_null() {
        return -14; // EFAULT
    }
    // Linux pops the pretcode pushed by setup_rt_frame, so `rsp` now points at
    // the ucontext.  Our setup_rt_frame leaves `rsp` at the base of the frame
    // including pretcode; the user restorer should `add $8, %rsp` before
    // calling rt_sigreturn.  We tolerate both layouts by re-reading from
    // the documented offset.
    let sp = unsafe { (*regs).sp };
    let Some((_frame_addr, frame)) = (unsafe { rt_sigframe_from_sp(sp) }) else {
        return unsafe { bad_rt_sigreturn() };
    };
    let sc: &SigContext = &frame.uc.uc_mcontext;
    let mask = frame.uc.uc_sigmask;

    let regs_mut = unsafe { &mut *regs };
    const FIX_EFLAGS: u64 = 0x50dd5;
    regs_mut.r8 = sc.r8;
    regs_mut.r9 = sc.r9;
    regs_mut.r10 = sc.r10;
    regs_mut.r11 = sc.r11;
    regs_mut.r12 = sc.r12;
    regs_mut.r13 = sc.r13;
    regs_mut.r14 = sc.r14;
    regs_mut.r15 = sc.r15;
    regs_mut.di = sc.rdi;
    regs_mut.si = sc.rsi;
    regs_mut.bp = sc.rbp;
    regs_mut.bx = sc.rbx;
    regs_mut.dx = sc.rdx;
    regs_mut.ax = sc.rax;
    regs_mut.cx = sc.rcx;
    regs_mut.sp = sc.rsp;
    regs_mut.ip = sc.rip;
    regs_mut.flags = (regs_mut.flags & !FIX_EFLAGS) | (sc.eflags & FIX_EFLAGS);
    regs_mut.cs = (sc.cs as u64) | 0x3;
    regs_mut.ss = (sc.ss as u64) | 0x3;
    regs_mut.orig_ax = u64::MAX;

    // Restore the saved signal mask.
    {
        let mut table = SIGNAL_TABLE.lock();
        if let Ok(state) = table.get_or_create_current() {
            let mut next = mask;
            next.remove(SIGKILL);
            next.remove(SIGSTOP);
            state.blocked = next;
        }
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    recalc_tif_sigpending(task);

    // Linux: rt_sigreturn returns the restored rax verbatim so the resumed
    // program sees the syscall return value untouched.
    sc.rax as i64
}

unsafe fn bad_rt_sigreturn() -> i64 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return -14;
    }

    let _ = unsafe { force_sigsegv(task) };
    0
}

unsafe fn rt_sigframe_addr_from_sp(sp: u64) -> u64 {
    unsafe { rt_sigframe_from_sp(sp).map(|(addr, _)| addr).unwrap_or(0) }
}

unsafe fn rt_sigframe_from_sp(
    sp: u64,
) -> Option<(u64, crate::arch::x86::kernel::signal::RtSigFrame)> {
    let popped_restorer = sp.saturating_sub(core::mem::size_of::<u64>() as u64);
    if let Some(frame) = unsafe { read_rt_sigframe(popped_restorer) } {
        if rt_sigframe_candidate_is_plausible(&frame) {
            return Some((popped_restorer, frame));
        }
    }
    if let Some(frame) = unsafe { read_rt_sigframe(sp) } {
        if rt_sigframe_candidate_is_plausible(&frame) {
            return Some((sp, frame));
        }
    }
    None
}

#[cfg(not(test))]
unsafe fn read_rt_sigframe(addr: u64) -> Option<crate::arch::x86::kernel::signal::RtSigFrame> {
    use crate::arch::x86::kernel::signal::RtSigFrame;
    if addr == 0
        || !crate::arch::x86::kernel::uaccess::access_ok(
            addr,
            core::mem::size_of::<RtSigFrame>() as u64,
        )
    {
        return None;
    }
    let mut frame: RtSigFrame = unsafe { core::mem::zeroed() };
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            (&mut frame as *mut RtSigFrame).cast::<u8>(),
            addr as *const u8,
            core::mem::size_of::<RtSigFrame>(),
        )
    };
    if not_copied == 0 { Some(frame) } else { None }
}

#[cfg(test)]
unsafe fn read_rt_sigframe(addr: u64) -> Option<crate::arch::x86::kernel::signal::RtSigFrame> {
    if addr == 0 {
        return None;
    }
    Some(unsafe { core::ptr::read(addr as *const crate::arch::x86::kernel::signal::RtSigFrame) })
}

fn rt_sigframe_candidate_is_plausible(
    frame: &crate::arch::x86::kernel::signal::RtSigFrame,
) -> bool {
    let uc_flags_known = frame.uc.uc_flags & !0x7 == 0;
    let sc = &frame.uc.uc_mcontext;
    let ip_user = sc.rip < crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;
    let sp_user = sc.rsp < crate::arch::x86::kernel::uaccess::TASK_SIZE_MAX;
    let cs_user = (sc.cs & 0x3) == 0x3;
    let ss_user = sc.ss == 0 || (sc.ss & 0x3) == 0x3;
    uc_flags_known && ip_user && sp_user && cs_user && ss_user
}

/// Legacy entry kept for syscall wrappers that don't yet pass `regs`.
/// Falls back to returning 0 (no register restore).
pub unsafe fn sys_rt_sigreturn() -> i64 {
    0
}

// ── sigqueue rlimit accounting (RLIMIT_SIGPENDING) ───────────────────────────
//
// Linux's `__sigqueue_alloc` charges `sigpending` against the current uid's
// ucount entry.  Each successful queue increments the per-user counter; each
// successful dequeue decrements it.  This is the bridge to `kernel/user.c`
// and `kernel/ucount.c`.

/// Charge one sigqueue allocation against the current task's user.  Returns
/// 0 on success or `-EAGAIN` if RLIMIT_SIGPENDING is hit.  Linux:
/// `__sigqueue_alloc` (kernel/signal.c).
pub fn sigqueue_charge() -> i32 {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return 0;
    }
    let uid = unsafe {
        let cred = (*task).cred;
        if cred.is_null() { 0 } else { (*cred).uid.0 }
    };
    let user = crate::kernel::user::alloc_uid(crate::kernel::cred::KUid(uid));
    let prev = user
        .sigpending
        .fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    let cap = sigpending_cap();
    if cap > 0 && prev + 1 > cap {
        user.sigpending
            .fetch_sub(1, core::sync::atomic::Ordering::Release);
        crate::kernel::user::free_uid(user);
        return -11; // EAGAIN
    }
    crate::kernel::user::free_uid(user);
    0
}

/// Drop one sigqueue charge.  Called from `dequeue_unblocked_signal` after a
/// signal has been consumed.  Linux: `__sigqueue_free`.
fn sigqueue_release(sig: i32) {
    // Only real-time signals carry queued sigqueue allocations in our model
    // (matching Linux: standard signals are coalesced into the pending mask
    // without per-instance allocation).
    if sig < SIGRTMIN {
        return;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return;
    }
    let uid = unsafe {
        let cred = (*task).cred;
        if cred.is_null() { 0 } else { (*cred).uid.0 }
    };
    if let Some(user) = crate::kernel::user::find_user(crate::kernel::cred::KUid(uid)) {
        let _ = user.sigpending.fetch_update(
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
            |v| if v > 0 { Some(v - 1) } else { None },
        );
        crate::kernel::user::free_uid(user);
    }
}

/// Read the effective RLIMIT_SIGPENDING for the current task.  Returns 0
/// when no cap is set (unlimited).
fn sigpending_cap() -> i64 {
    // Linux pulls this from `task->signal->rlim[RLIMIT_SIGPENDING].rlim_cur`.
    // Until rlimit is wired per-task we use the per-user-namespace cap
    // recorded on the root user namespace.
    let ns = core::ptr::null::<core::ffi::c_void>();
    crate::kernel::ucount::get_userns_rlimit_max(ns, crate::kernel::ucount::RlimitType::SigPending)
}

// ── Coredump pipeline ────────────────────────────────────────────────────────
//
// Linux's `do_coredump` (fs/coredump.c) walks the per-task mm to produce an
// ELF core file matching what gdb expects.  Lupos's port lives in
// `crate::fs::binfmt_elf::elf_core_dump`; this helper is the kernel entry
// point that arms it.

/// Drive the coredump pipeline for `task` when delivering `info`.  Linux:
/// `do_coredump` (fs/coredump.c).  No-op when coredumps are disabled (the
/// initramfs is read-only by default; coredump_filter / setrlimit gating
/// happens inside `crate::fs::binfmt_elf::elf_core_dump`).
///
/// # Safety
/// `task` and `info` must be valid pointers.
unsafe fn do_coredump(task: *mut crate::kernel::task::TaskStruct, info: &SigInfo) {
    let _ = crate::fs::binfmt_elf::elf_core_dump(task, info.signo);
}

#[cfg(test)]
pub fn reset_for_tests() {
    SIGNAL_TABLE.lock().states.clear();
}

#[cfg(test)]
pub fn register_test_task(pid: i32, tgid: i32) {
    let mut table = SIGNAL_TABLE.lock();
    if table.states.iter().all(|s| s.pid != pid) {
        let state = table.state_for_new_task(pid, tgid);
        table.states.push(state);
    }
}

pub(crate) fn inherit_signal_state_for_clone(parent_pid: i32, child_pid: i32, child_tgid: i32) {
    SIGNAL_TABLE
        .lock()
        .inherit_for_clone(parent_pid, child_pid, child_tgid);
}

pub(crate) fn flush_signal_handlers_for_exec(force_default: bool) {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return;
    }

    let mut table = SIGNAL_TABLE.lock();
    let Ok(state) = table.get_or_create_current() else {
        return;
    };

    for action in state.actions.iter_mut().skip(1) {
        let keep_ignored = !force_default && handler_kind(action) == HandlerKind::Ignore;
        *action = RtSigAction {
            sa_handler: if keep_ignored { 1 } else { 0 },
            sa_flags: 0,
            sa_restorer: 0,
            sa_mask: SigSet::default(),
        };
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    use super::SIGNAL_TEST_LOCK as TEST_LOCK;

    fn syscall_restart_regs(errno: i32) -> crate::kernel::task::PtRegs {
        crate::kernel::task::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            bp: 0,
            bx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            ax: neg_errno(errno),
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            orig_ax: 202,
            ip: 0x400102,
            cs: crate::arch::x86::kernel::gdt::sel::USER_CS as u64,
            flags: 0x202,
            sp: 0x700000,
            ss: crate::arch::x86::kernel::gdt::sel::USER_DS as u64,
        }
    }

    #[test]
    fn sigset_add_remove_contains() {
        let mut s = SigSet::default();
        s.add(2);
        assert!(s.contains(2));
        s.remove(2);
        assert!(!s.contains(2));
    }

    #[test]
    fn tgkill_routes_by_tgid() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(101, 1);
        assert_eq!(unsafe { sys_tgkill(1, 101, 0) }, 0);
        assert_eq!(unsafe { sys_tgkill(2, 101, 10) }, -3);
        assert_eq!(unsafe { sys_tgkill(1, 101, 10) }, 0);
        assert_eq!(unsafe { sys_tgkill(1, 0, 10) }, -22);
        assert_eq!(unsafe { sys_tkill(0, 10) }, -22);
        assert_eq!(unsafe { sys_tkill(101, 0) }, 0);
    }

    #[test]
    fn clone_signal_state_inherits_handlers_but_not_pending_signals() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(100, 100);

        {
            let mut table = SIGNAL_TABLE.lock();
            let parent = table.get_by_pid_mut(100).expect("parent signal state");
            parent.actions[(SIGRTMIN + 1) as usize] = RtSigAction {
                sa_handler: 0xfeed_cafe,
                sa_flags: SA_RESTART,
                sa_restorer: 0x1234,
                sa_mask: SigSet { bits: sig_bit(12) },
            };
            parent.blocked.add(SIGUSR1);
            parent.pending.add(SIGTERM);
            parent.shared_pending.add(SIGALRM);
            parent.rt_queue.push_back(SigInfo::new(SIGRTMIN + 1, 0));
        }

        inherit_signal_state_for_clone(100, 101, 100);

        let table = SIGNAL_TABLE.lock();
        let child = table
            .states
            .iter()
            .find(|state| state.pid == 101)
            .expect("child signal state");
        assert_eq!(
            child.actions[(SIGRTMIN + 1) as usize].sa_handler,
            0xfeed_cafe
        );
        assert!(child.blocked.contains(SIGUSR1));
        assert_eq!(child.pending.bits, 0);
        assert_eq!(child.shared_pending.bits, 0);
        assert!(child.rt_queue.is_empty());
        assert!(child.shared_queue.is_empty());
        assert!(
            !signal_is_default_fatal(child, SIGRTMIN + 1),
            "inherited NPTL realtime handler must prevent default termination"
        );
    }

    #[test]
    fn rt_sigaction_updates_existing_thread_group_signal_states() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(100, 100);
        register_test_task(101, 100);

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 100;
        current.tgid = 100;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            let action = RtSigAction {
                sa_handler: 0x5555_aaaa,
                sa_flags: SA_RESTART,
                sa_restorer: 0x1111,
                sa_mask: SigSet::default(),
            };
            assert_eq!(
                sys_rt_sigaction(
                    SIGRTMIN + 1,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>()
                ),
                0
            );
            sched::set_current(previous);
        }

        let table = SIGNAL_TABLE.lock();
        for pid in [100, 101] {
            let state = table
                .states
                .iter()
                .find(|state| state.pid == pid)
                .expect("thread signal state");
            assert_eq!(
                state.actions[(SIGRTMIN + 1) as usize].sa_handler,
                0x5555_aaaa
            );
        }
    }

    #[test]
    fn late_thread_signal_state_inherits_thread_group_actions() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(100, 100);

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 100;
        current.tgid = 100;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            let action = RtSigAction {
                sa_handler: 0x7777_aaaa,
                sa_flags: SA_RESTART,
                sa_restorer: 0x2222,
                sa_mask: SigSet {
                    bits: sig_bit(SIGUSR1),
                },
            };
            assert_eq!(
                sys_rt_sigaction(
                    SIGRTMIN + 1,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>()
                ),
                0
            );

            register_test_task(101, 100);
            assert_eq!(sys_tgkill(100, 101, SIGRTMIN + 1), 0);
            sched::set_current(previous);
        }

        let table = SIGNAL_TABLE.lock();
        let thread = table
            .states
            .iter()
            .find(|state| state.pid == 101)
            .expect("late thread signal state");
        assert_eq!(
            thread.actions[(SIGRTMIN + 1) as usize].sa_handler,
            0x7777_aaaa
        );
        assert!(thread.pending.contains(SIGRTMIN + 1));
        let queued = thread.rt_queue.front().expect("queued realtime siginfo");
        assert_eq!(queued.code, SI_TKILL);
        assert_eq!(
            i32::from_ne_bytes(queued._sifields[0..4].try_into().unwrap()),
            100
        );
        assert!(
            !signal_is_default_fatal(thread, SIGRTMIN + 1),
            "late-created thread must not treat NPTL realtime signal as default fatal"
        );
    }

    #[test]
    fn standard_user_process_signal_preserves_sender_siginfo() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 140;
        current.tgid = 140;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            register_test_task(140, 140);
            assert_eq!(send_user_signal_to_process(140, SIGUSR1), 0);
            let info = dequeue_current_pending_signal_mask(sig_bit(SIGUSR1))
                .expect("queued standard user signal");
            assert_eq!(info.signo, SIGUSR1);
            assert_eq!(info.code, SI_USER);
            assert_eq!(
                i32::from_ne_bytes(info._sifields[0..4].try_into().unwrap()),
                140
            );
            assert_eq!(
                u32::from_ne_bytes(info._sifields[4..8].try_into().unwrap()),
                0
            );
            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn standard_signal_coalescing_retains_first_sender() {
        let _guard = TEST_LOCK.lock();
        let mut state = SignalState::new(150, 150);
        queue_signal_info(
            &mut state,
            SIGUSR1,
            SigInfo::with_sender(SIGUSR1, SI_USER, 151, 1),
            false,
        );
        queue_signal_info(
            &mut state,
            SIGUSR1,
            SigInfo::with_sender(SIGUSR1, SI_USER, 152, 2),
            false,
        );

        assert_eq!(state.rt_queue.len(), 1);
        let info = state
            .dequeue_specific_signal(SIGUSR1)
            .expect("coalesced standard signal");
        assert_eq!(
            i32::from_ne_bytes(info._sifields[0..4].try_into().unwrap()),
            151
        );
        assert_eq!(
            u32::from_ne_bytes(info._sifields[4..8].try_into().unwrap()),
            1
        );
    }

    #[test]
    fn standard_signals_dequeue_in_signal_number_order() {
        let _guard = TEST_LOCK.lock();
        let mut state = SignalState::new(153, 153);
        queue_signal_info(
            &mut state,
            SIGUSR2,
            SigInfo::with_sender(SIGUSR2, SI_USER, 154, 1),
            false,
        );
        queue_signal_info(
            &mut state,
            SIGUSR1,
            SigInfo::with_sender(SIGUSR1, SI_USER, 155, 2),
            false,
        );

        let first = state.dequeue_unblocked_signal().expect("first signal");
        let second = state.dequeue_unblocked_signal().expect("second signal");
        assert_eq!(first.signo, SIGUSR1);
        assert_eq!(second.signo, SIGUSR2);
        assert_eq!(
            i32::from_ne_bytes(first._sifields[0..4].try_into().unwrap()),
            155
        );
        assert_eq!(
            i32::from_ne_bytes(second._sifields[0..4].try_into().unwrap()),
            154
        );
    }

    #[test]
    fn task_and_shared_standard_siginfo_do_not_cross_scopes() {
        let _guard = TEST_LOCK.lock();
        let mut state = SignalState::new(156, 156);
        queue_signal_info(
            &mut state,
            SIGUSR1,
            SigInfo::with_sender(SIGUSR1, SI_TKILL, 157, 3),
            false,
        );
        queue_signal_info(
            &mut state,
            SIGUSR1,
            SigInfo::with_sender(SIGUSR1, SI_USER, 158, 4),
            true,
        );

        let task = state.dequeue_unblocked_signal().expect("task signal");
        assert_eq!(task.code, SI_TKILL);
        assert!(state.shared_pending.contains(SIGUSR1));
        let shared = state.dequeue_unblocked_signal().expect("shared signal");
        assert_eq!(shared.code, SI_USER);
        assert_eq!(
            i32::from_ne_bytes(task._sifields[0..4].try_into().unwrap()),
            157
        );
        assert_eq!(
            i32::from_ne_bytes(shared._sifields[0..4].try_into().unwrap()),
            158
        );
    }

    #[test]
    fn exec_flush_resets_caught_handlers_but_keeps_ignored_handlers() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 110;
        current.tgid = 110;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            register_test_task(110, 110);
            {
                let mut table = SIGNAL_TABLE.lock();
                let state = table.get_by_pid_mut(110).expect("registered");
                state.actions[SIGTERM as usize] = RtSigAction {
                    sa_handler: 0x1234,
                    sa_flags: SA_RESTART,
                    sa_restorer: 0x5678,
                    sa_mask: SigSet {
                        bits: sig_bit(SIGUSR1),
                    },
                };
                state.actions[SIGPIPE as usize] = RtSigAction {
                    sa_handler: 1,
                    sa_flags: SA_RESTART,
                    sa_restorer: 0x5678,
                    sa_mask: SigSet {
                        bits: sig_bit(SIGUSR1),
                    },
                };
            }

            flush_signal_handlers_for_exec(false);

            let table = SIGNAL_TABLE.lock();
            let state = table
                .states
                .iter()
                .find(|state| state.pid == 110)
                .expect("state after exec flush");
            assert_eq!(state.actions[SIGTERM as usize].sa_handler, 0);
            assert_eq!(state.actions[SIGTERM as usize].sa_flags, 0);
            assert_eq!(state.actions[SIGTERM as usize].sa_restorer, 0);
            assert_eq!(state.actions[SIGTERM as usize].sa_mask.bits, 0);
            assert_eq!(state.actions[SIGPIPE as usize].sa_handler, 1);
            assert_eq!(state.actions[SIGPIPE as usize].sa_flags, 0);
            assert_eq!(state.actions[SIGPIPE as usize].sa_restorer, 0);
            assert_eq!(state.actions[SIGPIPE as usize].sa_mask.bits, 0);
            drop(table);

            sched::set_current(previous);
        }
    }

    #[test]
    fn queueinfo_rejects_invalid_signal() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(5, 5);
        assert_eq!(
            unsafe { sys_rt_sigqueueinfo(5, 1000, core::ptr::null()) },
            -22
        );
    }

    #[test]
    fn signal_syscalls_reject_kernel_user_pointers() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 82;
        current.tgid = 82;
        current.cred = &raw const INIT_CRED;

        let kernel_ptr = uaccess::TASK_SIZE_MAX as *mut u8;
        let bad_action = kernel_ptr.cast::<RtSigAction>();
        let bad_set = kernel_ptr.cast::<SigSet>();
        let bad_info = kernel_ptr.cast::<SigInfo>();
        let bad_stack = kernel_ptr.cast::<SigAltStack>();

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                sys_rt_sigaction(
                    12,
                    bad_action.cast_const(),
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>(),
                ),
                -14
            );
            assert_eq!(
                sys_rt_sigaction(
                    12,
                    core::ptr::null(),
                    bad_action,
                    core::mem::size_of::<SigSet>(),
                ),
                -14
            );
            assert_eq!(
                sys_rt_sigprocmask(
                    SIG_BLOCK,
                    bad_set.cast_const(),
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>(),
                ),
                -14
            );
            assert_eq!(
                sys_rt_sigprocmask(
                    SIG_BLOCK,
                    core::ptr::null(),
                    bad_set,
                    core::mem::size_of::<SigSet>(),
                ),
                -14
            );
            assert_eq!(
                sys_rt_sigpending(bad_set, core::mem::size_of::<SigSet>()),
                -14
            );
            assert_eq!(sys_rt_sigqueueinfo(82, 12, bad_info.cast_const()), -14);
            assert_eq!(
                sys_rt_sigtimedwait(
                    bad_set.cast_const(),
                    core::ptr::null_mut(),
                    core::ptr::null(),
                    core::mem::size_of::<SigSet>(),
                ),
                -14
            );
            assert_eq!(
                sys_sigaltstack(bad_stack.cast_const(), core::ptr::null_mut()),
                -14
            );
            assert_eq!(sys_sigaltstack(core::ptr::null(), bad_stack), -14);

            sched::set_current(previous);
        }
    }

    #[test]
    fn sigtimedwait_returns_queued_signal() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        register_test_task(7, 7);
        assert_eq!(unsafe { sys_tkill(7, 12) }, 0);

        let mut info = SigInfo::default();
        let wait = SigSet { bits: sig_bit(12) };
        // emulate current-task lookup by directly consuming table state
        let mut tbl = SIGNAL_TABLE.lock();
        let st = tbl.get_by_pid_mut(7).expect("state");
        let got = st.dequeue_unblocked_signal().expect("queued");
        assert_eq!(got.signo, 12);
        info = got;
        assert_eq!(info.signo, 12);
        let _ = wait;
    }

    #[test]
    fn rt_sigsuspend_uses_temporary_mask_until_signal_delivery() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 77;
        current.tgid = 77;
        current.cred = &raw const INIT_CRED;

        let mut stack = [0u8; 4096];
        let stack_top = unsafe { stack.as_mut_ptr().add(stack.len()) as u64 };
        let mut regs = crate::kernel::task::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            bp: 0,
            bx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            ax: 0,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            orig_ax: 0,
            ip: 0x7000,
            cs: crate::arch::x86::kernel::gdt::sel::USER_CS as u64,
            flags: 0x202,
            sp: stack_top,
            ss: crate::arch::x86::kernel::gdt::sel::USER_DS as u64,
        };

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            let action = RtSigAction {
                sa_handler: 0x4444,
                sa_restorer: 0x5555,
                ..Default::default()
            };
            assert_eq!(
                sys_rt_sigaction(
                    SIGALRM,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>(),
                ),
                0
            );
            let old_blocked = SigSet {
                bits: sig_bit(SIGUSR1),
            };
            assert_eq!(
                sys_rt_sigprocmask(
                    SIG_SETMASK,
                    &old_blocked,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>(),
                ),
                0
            );
            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGALRM),
                0
            );

            let temporary = SigSet::default();
            assert_eq!(
                sys_rt_sigsuspend(&temporary, core::mem::size_of::<SigSet>()),
                -4
            );
            {
                let table = SIGNAL_TABLE.lock();
                let state = table.states.iter().find(|state| state.pid == 77).unwrap();
                assert_eq!(state.blocked.bits, 0);
                assert_eq!(state.sigsuspend_saved, Some(old_blocked));
            }

            assert!(do_signal(&mut regs));
            assert_eq!(regs.ip, action.sa_handler as u64);
            let frame = &*(regs.sp as *const crate::arch::x86::kernel::signal::RtSigFrame);
            assert_eq!(frame.uc.uc_sigmask, old_blocked);
            {
                let table = SIGNAL_TABLE.lock();
                let state = table.states.iter().find(|state| state.pid == 77).unwrap();
                assert!(state.blocked.contains(SIGUSR1));
                assert!(state.blocked.contains(SIGALRM));
                assert_eq!(state.sigsuspend_saved, None);
            }

            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn process_scoped_signal_queues_shared_pending_only() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 78;
        current.tgid = 78;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(
                send_signal_to_task_scoped(
                    &mut *current as *mut TaskStruct,
                    SIGALRM,
                    SignalQueueScope::Shared,
                ),
                0
            );
            sched::set_current(previous);
        }

        let table = SIGNAL_TABLE.lock();
        let state = table.states.iter().find(|state| state.pid == 78).unwrap();
        assert!(!state.pending.contains(SIGALRM));
        assert!(state.shared_pending.contains(SIGALRM));
        drop(table);
        reset_for_tests();
    }

    #[test]
    fn process_scoped_signal_wakes_unblocked_sibling() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut leader = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        leader.pid = 120;
        leader.tgid = 120;
        leader.cred = &raw const INIT_CRED;
        let mut sibling = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        sibling.pid = 121;
        sibling.tgid = 120;
        sibling.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *sibling as *mut TaskStruct);
            register_test_task(120, 120);
            register_test_task(121, 120);
            {
                let mut table = SIGNAL_TABLE.lock();
                table
                    .get_by_pid_mut(120)
                    .expect("leader")
                    .blocked
                    .add(SIGALRM);
            }

            assert_eq!(
                send_signal_to_task_scoped(
                    &mut *leader as *mut TaskStruct,
                    SIGALRM,
                    SignalQueueScope::Shared,
                ),
                0
            );

            assert_eq!(
                leader.thread_info.flags & crate::kernel::task::TIF_SIGPENDING,
                0
            );
            assert!(sibling.thread_info.flags & crate::kernel::task::TIF_SIGPENDING != 0);
            assert!(has_unblocked_pending_signals(
                &mut *sibling as *mut TaskStruct
            ));
            assert!(!has_unblocked_pending_signals(
                &mut *leader as *mut TaskStruct
            ));

            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn sibling_dequeue_clears_process_shared_pending_once() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut leader = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        leader.pid = 122;
        leader.tgid = 122;
        leader.cred = &raw const INIT_CRED;
        let mut sibling = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        sibling.pid = 123;
        sibling.tgid = 122;
        sibling.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *sibling as *mut TaskStruct);
            register_test_task(122, 122);
            register_test_task(123, 122);
            {
                let mut table = SIGNAL_TABLE.lock();
                table
                    .get_by_pid_mut(122)
                    .expect("leader")
                    .blocked
                    .add(SIGUSR1);
            }

            assert_eq!(
                send_signal_to_task_scoped(
                    &mut *leader as *mut TaskStruct,
                    SIGUSR1,
                    SignalQueueScope::Shared,
                ),
                0
            );
            assert!(has_pending_signal_for_pid(122, SIGUSR1));
            assert!(has_pending_signal_for_pid(123, SIGUSR1));

            let info =
                dequeue_current_pending_signal_mask(sig_bit(SIGUSR1)).expect("shared signal");
            assert_eq!(info.signo, SIGUSR1);
            assert!(!has_pending_signal_for_pid(122, SIGUSR1));
            assert!(!has_pending_signal_for_pid(123, SIGUSR1));
            assert!(dequeue_current_pending_signal_mask(sig_bit(SIGUSR1)).is_none());

            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn task_scoped_signal_queues_task_pending_only() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 79;
        current.tgid = 79;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(
                send_signal_to_task_scoped(
                    &mut *current as *mut TaskStruct,
                    SIGTERM,
                    SignalQueueScope::Task,
                ),
                0
            );
            sched::set_current(previous);
        }

        let table = SIGNAL_TABLE.lock();
        let state = table.states.iter().find(|state| state.pid == 79).unwrap();
        assert!(state.pending.contains(SIGTERM));
        assert!(!state.shared_pending.contains(SIGTERM));
        drop(table);
        reset_for_tests();
    }

    #[test]
    fn delivered_user_signal_recalculates_pending_flag() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 82;
        current.tgid = 82;
        current.cred = &raw const INIT_CRED;

        let mut stack = [0u8; 4096];
        let stack_top = unsafe { stack.as_mut_ptr().add(stack.len()) as u64 };
        let mut regs = crate::kernel::task::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            bp: 0,
            bx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            ax: 0,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            orig_ax: NO_SYSCALL,
            ip: 0x7000,
            cs: crate::arch::x86::kernel::gdt::sel::USER_CS as u64,
            flags: 0x202,
            sp: stack_top,
            ss: crate::arch::x86::kernel::gdt::sel::USER_DS as u64,
        };

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            let action = RtSigAction {
                sa_handler: 0x4444,
                sa_restorer: 0x5555,
                ..Default::default()
            };
            assert_eq!(
                sys_rt_sigaction(
                    SIGUSR1,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>(),
                ),
                0
            );
            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGUSR1),
                0
            );
            assert!(current.thread_info.flags & crate::kernel::task::TIF_SIGPENDING != 0);

            assert!(do_signal(&mut regs));
            assert_eq!(regs.ip, action.sa_handler as u64);
            assert_eq!(
                current.thread_info.flags & crate::kernel::task::TIF_SIGPENDING,
                0
            );

            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn syscall_restart_handler_with_sa_restart_rewinds_original_syscall() {
        let mut regs = syscall_restart_regs(ERESTARTSYS);
        let action = RtSigAction {
            sa_flags: SA_RESTART,
            ..Default::default()
        };

        apply_syscall_restart_for_handler(&mut regs, &action);

        assert_eq!(regs.ax, regs.orig_ax);
        assert_eq!(regs.ip, 0x400100);
    }

    #[test]
    fn syscall_restart_handler_without_sa_restart_returns_eintr() {
        let mut regs = syscall_restart_regs(ERESTARTSYS);
        let action = RtSigAction::default();

        apply_syscall_restart_for_handler(&mut regs, &action);

        assert_eq!(regs.ax, neg_errno(EINTR));
        assert_eq!(regs.ip, 0x400102);
    }

    #[test]
    fn syscall_restart_no_handler_rewinds_original_syscall() {
        let mut regs = syscall_restart_regs(ERESTARTNOHAND);

        apply_syscall_restart_without_handler(&mut regs);

        assert_eq!(regs.ax, regs.orig_ax);
        assert_eq!(regs.ip, 0x400100);
    }

    #[test]
    fn syscall_restartblock_no_handler_uses_restart_syscall() {
        let mut regs = syscall_restart_regs(ERESTART_RESTARTBLOCK);

        apply_syscall_restart_without_handler(&mut regs);

        assert_eq!(regs.ax, X86_64_NR_RESTART_SYSCALL);
        assert_eq!(regs.ip, 0x400100);
    }

    #[test]
    fn syscall_m76_signal_parity() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 81;
        current.tgid = 80;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            let action = RtSigAction {
                sa_handler: 0x1234,
                sa_flags: SA_RESTART,
                sa_restorer: 0,
                sa_mask: SigSet { bits: sig_bit(12) },
            };
            let mut old = RtSigAction::default();
            assert_eq!(
                sys_rt_sigaction(
                    12,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>()
                ),
                0
            );
            assert_eq!(
                sys_rt_sigaction(
                    12,
                    core::ptr::null(),
                    &mut old,
                    core::mem::size_of::<SigSet>()
                ),
                0
            );
            assert_eq!(old.sa_handler, action.sa_handler);
            assert_eq!(
                sys_rt_sigaction(
                    SIGKILL,
                    &action,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>()
                ),
                -22
            );
            assert_eq!(sys_rt_sigaction(12, &action, core::ptr::null_mut(), 4), -22);

            let set = SigSet {
                bits: sig_bit(SIGKILL) | sig_bit(10),
            };
            let mut oldset = SigSet::default();
            assert_eq!(
                sys_rt_sigprocmask(SIG_BLOCK, &set, &mut oldset, core::mem::size_of::<SigSet>()),
                0
            );
            assert_eq!(
                sys_rt_sigprocmask(
                    99,
                    &set,
                    core::ptr::null_mut(),
                    core::mem::size_of::<SigSet>()
                ),
                -22
            );
            assert_eq!(
                sys_rt_sigprocmask(
                    SIG_SETMASK,
                    core::ptr::null(),
                    &mut oldset,
                    core::mem::size_of::<SigSet>()
                ),
                0
            );
            assert!(oldset.contains(10));
            assert!(!oldset.contains(SIGKILL));

            let mut pending = SigSet::default();
            assert_eq!(
                sys_rt_sigpending(&mut pending, core::mem::size_of::<SigSet>()),
                0
            );
            assert_eq!(
                sys_rt_sigpending(core::ptr::null_mut(), core::mem::size_of::<SigSet>()),
                -14
            );
            assert_eq!(sys_tkill(81, 12), 0);
            assert_eq!(sys_tkill(9999, 12), -3);
            assert_eq!(sys_tkill(81, 1000), -22);
            assert_eq!(sys_tgkill(80, 81, 13), 0);
            assert_eq!(sys_tgkill(99, 81, 13), -3);
            assert_eq!(sys_rt_sigqueueinfo(81, 14, core::ptr::null()), 0);
            assert_eq!(
                crate::kernel::syscalls::sys_rt_tgsigqueueinfo(80, 81, 15, core::ptr::null()),
                0
            );

            let wait = SigSet { bits: sig_bit(12) };
            let mut info = SigInfo::default();
            assert_eq!(
                sys_rt_sigtimedwait(
                    &wait,
                    &mut info,
                    core::ptr::null(),
                    core::mem::size_of::<SigSet>()
                ),
                12
            );
            assert_eq!(info.signo, 12);
            assert_eq!(
                sys_rt_sigtimedwait(
                    core::ptr::null(),
                    &mut info,
                    core::ptr::null(),
                    core::mem::size_of::<SigSet>()
                ),
                -14
            );

            let new_ss = SigAltStack {
                ss_sp: 0x7000,
                ss_flags: SS_DISABLE,
                ss_size: 8192,
            };
            let mut old_ss = SigAltStack::default();
            assert_eq!(sys_sigaltstack(&new_ss, &mut old_ss), 0);
            assert_eq!(sys_sigaltstack(core::ptr::null(), &mut old_ss), 0);
            assert_eq!(old_ss.ss_sp, new_ss.ss_sp);
            let bad_ss = SigAltStack {
                ss_sp: 0,
                ss_flags: 4,
                ss_size: 0,
            };
            assert_eq!(sys_sigaltstack(&bad_ss, core::ptr::null_mut()), -22);
            assert_eq!(sys_rt_sigreturn(), 0);

            sched::set_current(previous);
        }
    }

    #[test]
    fn rt_sigreturn_accepts_frame_base_or_popped_restorer_sp() {
        let mut stack = [0u8; 2048];
        let frame_base = unsafe { stack.as_mut_ptr().add(256) as u64 };
        let frame =
            unsafe { &mut *(frame_base as *mut crate::arch::x86::kernel::signal::RtSigFrame) };
        frame.pretcode = 0x7000;
        frame.uc.uc_flags = 0;
        frame.uc.uc_mcontext.rip = 0x401000;
        frame.uc.uc_mcontext.rsp = 0x7fff_1000;
        frame.uc.uc_mcontext.cs = 0x33;
        frame.uc.uc_mcontext.ss = 0x2b;

        assert_eq!(
            unsafe { rt_sigframe_addr_from_sp(frame_base + core::mem::size_of::<u64>() as u64) },
            frame_base
        );
        assert_eq!(unsafe { rt_sigframe_addr_from_sp(frame_base) }, frame_base);
    }

    #[test]
    fn rt_sigreturn_rejects_implausible_frame_and_forces_sigsegv() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 93;
        current.tgid = 93;
        current.cred = &raw const INIT_CRED;

        let mut stack = [0u8; 2048];
        let frame_base = unsafe { stack.as_mut_ptr().add(256) as u64 };
        let frame =
            unsafe { &mut *(frame_base as *mut crate::arch::x86::kernel::signal::RtSigFrame) };
        frame.uc.uc_flags = 0;
        frame.uc.uc_mcontext.rip = 0x3030_3030_3030_3030;
        frame.uc.uc_mcontext.rsp = 0x7fff_1000;
        frame.uc.uc_mcontext.cs = 0x33;
        frame.uc.uc_mcontext.ss = 0x2b;
        frame.uc.uc_mcontext.rax = 0xfeed;

        let mut regs = crate::kernel::task::PtRegs {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            bp: 0,
            bx: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            ax: crate::arch::x86::entry::syscall::SYS_RT_SIGRETURN,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            orig_ax: crate::arch::x86::entry::syscall::SYS_RT_SIGRETURN,
            ip: 0x7000,
            cs: crate::arch::x86::kernel::gdt::sel::USER_CS as u64,
            flags: 0x202,
            sp: frame_base + core::mem::size_of::<u64>() as u64,
            ss: crate::arch::x86::kernel::gdt::sel::USER_DS as u64,
        };

        unsafe {
            sched::set_current(&mut *current);
            assert_eq!(sys_rt_sigreturn_impl(&mut regs), 0);
            sched::set_current(previous);
        }

        assert_eq!(regs.ip, 0x7000, "bad sigframe must not restore RIP");
        assert_eq!(regs.ax, crate::arch::x86::entry::syscall::SYS_RT_SIGRETURN);
        assert!(has_pending_signal_for_pid(93, SIGSEGV));
        reset_for_tests();
    }

    #[test]
    fn sigkill_wakes_uninterruptible_task_and_can_be_taken_by_blocking_wait() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 91;
        current.tgid = 91;
        current.cred = &raw const INIT_CRED;
        current.__state.store(
            crate::kernel::task::task_state::TASK_UNINTERRUPTIBLE,
            core::sync::atomic::Ordering::Release,
        );

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGKILL),
                0
            );
            assert_eq!(
                current.__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::TASK_RUNNING
            );
            assert_eq!(take_current_fatal_signal(), Some(SIGKILL));
            assert!(!has_pending_signal_for_pid(91, SIGKILL));

            sched::set_current(previous);
        }
    }

    #[test]
    fn late_sigkill_does_not_resurrect_zombie_task() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut sender = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        sender.pid = 190;
        sender.tgid = 190;
        sender.cred = &raw const INIT_CRED;
        let mut zombie = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        zombie.pid = 191;
        zombie.tgid = 191;
        zombie.m26.exit_state = crate::kernel::task::task_state::EXIT_ZOMBIE;
        zombie.__state.store(
            crate::kernel::task::task_state::EXIT_ZOMBIE,
            core::sync::atomic::Ordering::Release,
        );

        unsafe {
            sched::set_current(&mut *sender as *mut TaskStruct);
            assert_eq!(
                send_signal_to_task(&mut *zombie as *mut TaskStruct, SIGKILL),
                0
            );
            assert_eq!(
                zombie.__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::EXIT_ZOMBIE
            );
            sched::set_current(previous);
        }
        reset_for_tests();
    }

    #[test]
    fn sigterm_wakes_interruptible_task_for_syscall_exit_delivery() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 92;
        current.tgid = 92;
        current.cred = &raw const INIT_CRED;
        current.__state.store(
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            core::sync::atomic::Ordering::Release,
        );

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGTERM),
                0
            );
            assert_eq!(
                current.__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::TASK_RUNNING
            );
            assert!(has_pending_signal_for_pid(92, SIGTERM));

            sched::set_current(previous);
        }
    }

    #[test]
    fn default_sigterm_can_be_taken_by_blocking_wait() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 94;
        current.tgid = 94;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGTERM),
                0
            );
            assert_eq!(take_current_fatal_signal(), Some(SIGTERM));
            assert!(!has_pending_signal_for_pid(94, SIGTERM));

            sched::set_current(previous);
        }
    }

    #[test]
    fn handled_sigterm_is_preserved_for_signal_delivery() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 95;
        current.tgid = 95;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            register_test_task(95, 95);
            {
                let mut table = SIGNAL_TABLE.lock();
                let state = table.get_by_pid_mut(95).expect("registered");
                state.actions[SIGTERM as usize].sa_handler = 0x1234;
            }

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGTERM),
                0
            );
            assert_eq!(take_current_fatal_signal(), None);
            assert!(has_pending_signal_for_pid(95, SIGTERM));
            assert!(has_unblocked_pending_signals(
                &mut *current as *mut TaskStruct
            ));

            sched::set_current(previous);
        }
    }

    #[test]
    fn blocked_sigterm_is_not_taken_by_blocking_wait() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 96;
        current.tgid = 96;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            register_test_task(96, 96);
            {
                let mut table = SIGNAL_TABLE.lock();
                let state = table.get_by_pid_mut(96).expect("registered");
                state.blocked.add(SIGTERM);
            }

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGTERM),
                0
            );
            assert_eq!(take_current_fatal_signal(), None);
            assert!(has_pending_signal_for_pid(96, SIGTERM));
            assert!(!has_unblocked_pending_signals(
                &mut *current as *mut TaskStruct
            ));

            sched::set_current(previous);
        }
    }

    #[test]
    fn default_ignored_sigchld_does_not_interrupt_blocking_waits() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 97;
        current.tgid = 97;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                send_signal_to_task(&mut *current as *mut TaskStruct, SIGCHLD),
                0
            );
            assert!(!has_pending_signal_for_pid(97, SIGCHLD));
            assert!(!has_unblocked_pending_signals(
                &mut *current as *mut TaskStruct
            ));

            sched::set_current(previous);
        }
    }

    #[test]
    fn stop_signal_sent_to_other_task_is_wait_observable_without_dequeue() {
        let _guard = TEST_LOCK.lock();
        reset_for_tests();
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 101;
        current.tgid = 101;
        current.cred = &raw const INIT_CRED;
        let mut child = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        child.pid = 102;
        child.tgid = 102;
        child.cred = &raw const INIT_CRED;
        child.m26.real_parent = &mut *current as *mut TaskStruct;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                send_signal_to_task(&mut *child as *mut TaskStruct, SIGCONT),
                0
            );
            assert!(has_pending_signal_for_pid(102, SIGCONT));

            assert_eq!(
                send_signal_to_task(&mut *child as *mut TaskStruct, SIGTSTP),
                0
            );
            assert_eq!(
                child.__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::__TASK_STOPPED
            );
            assert_eq!(child.m26.ptrace_stop_signal, SIGTSTP);
            assert!(!has_pending_signal_for_pid(102, SIGTSTP));
            assert!(!has_pending_signal_for_pid(102, SIGCONT));
            assert!(!has_pending_signal_for_pid(101, SIGCHLD));

            assert_eq!(
                send_signal_to_task(&mut *child as *mut TaskStruct, SIGCONT),
                0
            );
            assert_eq!(
                child.__state.load(core::sync::atomic::Ordering::Acquire),
                crate::kernel::task::task_state::TASK_RUNNING
            );
            assert_eq!(child.m26.ptrace_stop_signal, 0);
            assert!(!has_pending_signal_for_pid(102, SIGTSTP));

            sched::set_current(previous);
        }
    }
}
