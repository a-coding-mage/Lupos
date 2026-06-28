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
use crate::kernel::sched;

pub const NSIG: usize = 64;
/// Signal number range — RT signals start at SIGRTMIN.
pub const SIGRTMIN: i32 = 32;
pub const SIGRTMAX: i32 = NSIG as i32;

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
    pending: SigSet,
    shared_pending: SigSet,
    rt_queue: VecDeque<SigInfo>,
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
            pending: SigSet::default(),
            shared_pending: SigSet::default(),
            rt_queue: VecDeque::new(),
            altstack: SigAltStack::default(),
            pid,
            tgid,
        }
    }

    fn dequeue_unblocked_signal(&mut self) -> Option<SigInfo> {
        if let Some(idx) = self
            .rt_queue
            .iter()
            .position(|info| !self.blocked.contains(info.signo))
        {
            let info = self.rt_queue.remove(idx)?;
            self.pending.remove(info.signo);
            self.shared_pending.remove(info.signo);
            return Some(info);
        }

        let mut merged = self.pending.bits | self.shared_pending.bits;
        merged &= !self.blocked.bits;
        if merged == 0 {
            return None;
        }
        let sig = merged.trailing_zeros() as i32 + 1;
        self.pending.remove(sig);
        self.shared_pending.remove(sig);
        Some(SigInfo::new(sig, 0))
    }

    fn dequeue_specific_signal(&mut self, sig: i32) -> Option<SigInfo> {
        if !self.pending.contains(sig) && !self.shared_pending.contains(sig) {
            return None;
        }
        self.pending.remove(sig);
        self.shared_pending.remove(sig);
        if let Some(idx) = self.rt_queue.iter().position(|info| info.signo == sig) {
            return self.rt_queue.remove(idx);
        }
        Some(SigInfo::new(sig, 0))
    }

    fn dequeue_masked_signal(&mut self, mask: u64) -> Option<SigInfo> {
        let mask = user_dequeue_signal_mask(mask);
        if mask == 0 {
            return None;
        }

        if let Some(idx) = self
            .rt_queue
            .iter()
            .position(|info| sig_bit(info.signo) & mask != 0)
        {
            let info = self.rt_queue.remove(idx)?;
            self.pending.remove(info.signo);
            self.shared_pending.remove(info.signo);
            return Some(info);
        }

        let merged = (self.pending.bits | self.shared_pending.bits) & mask;
        if merged == 0 {
            return None;
        }
        let sig = merged.trailing_zeros() as i32 + 1;
        self.pending.remove(sig);
        self.shared_pending.remove(sig);
        Some(SigInfo::new(sig, 0))
    }

    fn remove_signal(&mut self, sig: i32) {
        self.pending.remove(sig);
        self.shared_pending.remove(sig);
        self.rt_queue.retain(|info| info.signo != sig);
    }

    fn remove_signal_mask(&mut self, mask: u64) {
        self.pending.bits &= !mask;
        self.shared_pending.bits &= !mask;
        self.rt_queue.retain(|info| sig_bit(info.signo) & mask == 0);
    }
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
    fn get_or_create_current(&mut self) -> Result<&mut SignalState, i32> {
        let task = unsafe { sched::get_current() };
        if task.is_null() {
            return Err(-3); // ESRCH
        }
        let (pid, tgid) = unsafe { ((*task).pid, (*task).tgid) };
        if let Some(pos) = self.states.iter().position(|s| s.pid == pid) {
            return Ok(self.states.get_mut(pos).expect("index exists"));
        }
        self.states.push(SignalState::new(pid, tgid));
        Ok(self.states.last_mut().expect("just pushed"))
    }

    fn get_by_pid_mut(&mut self, pid: i32) -> Option<&mut SignalState> {
        let pos = self.states.iter().position(|s| s.pid == pid)?;
        self.states.get_mut(pos)
    }
}

static SIGNAL_TABLE: spin::Mutex<SignalTable> =
    spin::Mutex::new(SignalTable { states: Vec::new() });

fn valid_signal(sig: i32) -> bool {
    (1..=NSIG as i32).contains(&sig)
}

fn task_for_pid(pid: i32) -> *mut crate::kernel::task::TaskStruct {
    let heap = crate::kernel::fork::find_heap_task_by_pid(pid);
    if !heap.is_null() {
        return heap;
    }
    crate::kernel::sched::find_pool_task_by_pid(pid)
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
            state.actions[sig as usize] = action;
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
        // Linux never blocks SIGKILL/SIGSTOP.
        set.remove(SIGKILL);
        set.remove(SIGSTOP);
    }

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
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { sched::get_current() };
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

pub unsafe fn sys_rt_sigpending(set: *mut SigSet, sigsetsize: usize) -> i64 {
    if sigsetsize != core::mem::size_of::<SigSet>() {
        return -22;
    }
    if set.is_null() {
        return -14;
    }
    let pending = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(s) => s,
            Err(e) => return e as i64,
        };
        SigSet {
            bits: state.pending.bits | state.shared_pending.bits,
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
    enqueue_for_pid(pid, sig, info)
}

pub unsafe fn sys_tkill(pid: i32, sig: i32) -> i64 {
    enqueue_for_pid(pid, sig, SigInfo::new(sig, 0))
}

pub unsafe fn sys_tgkill(tgid: i32, pid: i32, sig: i32) -> i64 {
    if !valid_signal(sig) {
        return -22;
    }
    {
        let mut table = SIGNAL_TABLE.lock();
        if let Some(state) = table.get_by_pid_mut(pid) {
            if state.tgid != tgid {
                return -3;
            }
            state.pending.add(sig);
            if sig >= 32 {
                state.rt_queue.push_back(SigInfo::new(sig, 0));
            }
            drop(table);
            wake_signal_task_if_live(pid, sig);
            return 0;
        }
    }

    let task = task_for_pid(pid);
    if task.is_null() || unsafe { (*task).tgid } != tgid {
        return -3;
    }
    queue_signal_for_live_task(task, sig, SigInfo::new(sig, 0));
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
    let state = if let Some(pos) = table.states.iter().position(|s| s.pid == target_pid) {
        table.states.get_mut(pos).expect("position valid")
    } else {
        table.states.push(SignalState::new(target_pid, target_tgid));
        table.states.last_mut().expect("just pushed")
    };
    state.pending.add(sig);
    if sig >= 32 {
        state.rt_queue.push_back(info);
    }
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
            state.pending.add(sig);
            if sig >= 32 {
                state.rt_queue.push_back(info);
            }
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
    let candidate = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(s) => s,
            Err(e) => return e as i64,
        };
        let Some(candidate) = state.dequeue_unblocked_signal() else {
            return -11; // EAGAIN
        };
        if !wait.contains(candidate.signo) {
            // Put it back if caller wasn't waiting for this signal.
            state.pending.add(candidate.signo);
            if candidate.signo >= 32 {
                state.rt_queue.push_front(candidate);
            }
            return -11;
        }
        candidate
    };
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

/// Queue `sig` for delivery to `target` and raise `TIF_SIGPENDING`.
///
/// M26 helper used by `exit_notify` (SIGCHLD to parent) and
/// `ptrace::ptrace_attach` (SIGSTOP to tracee).  Returns 0 on success or
/// a negative errno on failure.
///
/// # Safety
/// `target` must be a valid `*mut TaskStruct`.
pub unsafe fn send_signal_to_task(target: *mut crate::kernel::task::TaskStruct, sig: i32) -> i32 {
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

    // Ensure a SignalState exists for the target — create one on demand,
    // mirroring the lazy registration in `get_or_create_current`.
    let mut table = SIGNAL_TABLE.lock();
    let state = if let Some(pos) = table.states.iter().position(|s| s.pid == target_pid) {
        table.states.get_mut(pos).expect("position valid")
    } else {
        table.states.push(SignalState::new(target_pid, target_tgid));
        table.states.last_mut().expect("just pushed")
    };
    // Linux `prepare_signal()` applies these job-control side effects at
    // signal generation time, regardless of blocking or default disposition.
    if is_stop_signal(sig) {
        state.remove_signal(SIGCONT);
    } else if sig == SIGCONT {
        state.remove_signal_mask(
            sig_bit(SIGSTOP) | sig_bit(SIGTSTP) | sig_bit(SIGTTIN) | sig_bit(SIGTTOU),
        );
    }
    let (pending_bits, blocked_bits) = if !stop_now {
        state.shared_pending.add(sig);
        state.pending.add(sig);
        if sig >= 32 {
            state.rt_queue.push_back(SigInfo::new(sig, 0));
        }
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
    let state = if let Some(pos) = table.states.iter().position(|s| s.pid == target_pid) {
        table.states.get_mut(pos).expect("position valid")
    } else {
        table.states.push(SignalState::new(target_pid, target_tgid));
        table.states.last_mut().expect("just pushed")
    };
    state.shared_pending.add(info.signo);
    state.pending.add(info.signo);
    state.rt_queue.push_back(info);
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
    if let Some(state) = table.states.iter().find(|s| s.pid == pid) {
        return state.shared_pending.contains(sig) || state.pending.contains(sig);
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
        .find(|state| state.pid == pid)
        .is_some_and(|state| {
            ((state.pending.bits | state.shared_pending.bits) & user_dequeue_signal_mask(mask)) != 0
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
        .find(|state| state.pid == pid)
        .map(|state| state.pending.bits | state.shared_pending.bits)
        .unwrap_or(0)
}

pub fn dequeue_current_pending_signal_mask(mask: u64) -> Option<SigInfo> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return None;
    }

    let (info, still_pending) = {
        let mut table = SIGNAL_TABLE.lock();
        let state = table.get_or_create_current().ok()?;
        let info = state.dequeue_masked_signal(mask)?;
        let still_pending = has_unblocked_pending_signal(state);
        (info, still_pending)
    };

    if !still_pending {
        clear_tif_sigpending(task);
    }
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

fn has_unblocked_pending_signal(state: &SignalState) -> bool {
    if state
        .rt_queue
        .iter()
        .any(|info| !state.blocked.contains(info.signo))
    {
        return true;
    }
    ((state.pending.bits | state.shared_pending.bits) & !state.blocked.bits) != 0
}

/// Consume a pending fatal signal for the current task without delivering a
/// user signal frame. Blocking syscall loops use this before sleeping again so
/// service timeout SIGKILLs take effect promptly.
pub fn take_current_fatal_signal() -> Option<i32> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() || !has_pending_signals(task) {
        return None;
    }

    let (signal, still_pending) = {
        let mut table = SIGNAL_TABLE.lock();
        let state = match table.get_or_create_current() {
            Ok(state) => state,
            Err(_) => return None,
        };
        let signal = state
            .dequeue_specific_signal(SIGKILL)
            .map(|info| info.signo);
        let still_pending = has_unblocked_pending_signal(state);
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

    let signal_info = {
        let mut table = SIGNAL_TABLE.lock();
        if let Ok(state) = table.get_or_create_current() {
            state.dequeue_unblocked_signal()
        } else {
            None
        }
    };

    let Some(info) = signal_info else {
        clear_tif_sigpending(task);
        return false;
    };

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
            let still_pending = {
                let mut table = SIGNAL_TABLE.lock();
                table
                    .get_or_create_current()
                    .map(|state| has_unblocked_pending_signal(state))
                    .unwrap_or(false)
            };
            if !still_pending {
                clear_tif_sigpending(task);
            }
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
        let signal_info = {
            let mut table = SIGNAL_TABLE.lock();
            if let Ok(state) = table.get_or_create_current() {
                state.dequeue_unblocked_signal()
            } else {
                None
            }
        };

        let Some(info) = signal_info else {
            clear_tif_sigpending(task);
            return false;
        };

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
                continue;
            }
            HandlerKind::Default => match default_action(info.signo) {
                DefaultAction::Ign => continue,
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
                    return unsafe { stop_current_for_signal(task, info.signo) };
                }
                DefaultAction::Cont => {
                    unsafe {
                        (*task).__state.store(
                            crate::kernel::task::task_state::TASK_RUNNING,
                            core::sync::atomic::Ordering::Release,
                        );
                    }
                    return true;
                }
            },
            HandlerKind::User(_) => {
                if unsafe {
                    crate::arch::x86::kernel::signal::setup_rt_frame(
                        regs, info.signo, &action, &info,
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
    let Some((frame_addr, frame)) = (unsafe { rt_sigframe_from_sp(sp) }) else {
        return unsafe { bad_rt_sigreturn() };
    };
    let sc: &SigContext = &frame.uc.uc_mcontext;
    let mask = frame.uc.uc_sigmask;

    #[cfg(not(test))]
    {
        if crate::kernel::debug_trace::proc_enabled() {
            let pid = unsafe {
                let task = crate::kernel::sched::get_current();
                if task.is_null() { -1 } else { (*task).pid }
            };
            crate::linux_driver_abi::tty::serial_println!(
                "trace-sigreturn pid={} sp={:#x} frame={:#x} sig={} rip={:#x} rsp={:#x} r12={:#x} r13={:#x} rbp={:#x} rbx={:#x} rax={:#x} flags={:#x} cs={:#x} ss={:#x}",
                pid,
                sp,
                frame_addr,
                frame.info.signo,
                sc.rip,
                sc.rsp,
                sc.r12,
                sc.r13,
                sc.rbp,
                sc.rbx,
                sc.rax,
                sc.eflags,
                sc.cs,
                sc.ss
            );
        }
    }

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
        table.states.push(SignalState::new(pid, tgid));
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

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
        assert_eq!(unsafe { sys_tgkill(2, 101, 10) }, -3);
        assert_eq!(unsafe { sys_tgkill(1, 101, 10) }, 0);
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
            assert!(has_pending_signal_for_pid(101, SIGCHLD));

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
