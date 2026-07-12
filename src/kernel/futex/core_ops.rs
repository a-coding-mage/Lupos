//! linux-parity: partial
//! linux-deviation: timed legacy futex waits use Linux absolute hrtimer-sleeper deadlines, but Lupos does not yet materialize the per-task restart_block needed for a handler-free restart_syscall continuation.
//! linux-source: vendor/linux/kernel/futex
//! test-origin: linux:vendor/linux/kernel/futex
//! Core futex operations — `futex_wait`, `futex_wake`, `futex_requeue`,
//! `futex_wake_op`.
//!
//! Mirrors `vendor/linux/kernel/futex/{waitwake,requeue}.c`.  Lupos M32 ships
//! the in-kernel hash bucket and the wait/wake state machine; userspace
//! `uaddr` reads currently expect kernel-mode pointers (we don't have a
//! `copy_from_user` until M59) and the test fixtures pass kernel pointers
//! directly via the in-kernel API.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use super::{
    EAGAIN, EFAULT, EINVAL, ENOSYS, ETIMEDOUT, FUTEX_32, FUTEX_OP_ADD, FUTEX_OP_ANDN,
    FUTEX_OP_CMP_EQ, FUTEX_OP_CMP_GE, FUTEX_OP_CMP_GT, FUTEX_OP_CMP_LE, FUTEX_OP_CMP_LT,
    FUTEX_OP_CMP_NE, FUTEX_OP_OPARG_SHIFT, FUTEX_OP_OR, FUTEX_OP_SET, FUTEX_OP_XOR, FUTEX_TID_MASK,
    FUTEX_WAITERS, FUTEX2_MPOL, FUTEX2_NUMA, FUTEX2_SIZE_MASK, FUTEX2_VALID_MASK, FutexWaitv,
};

// Kernel-internal restart pseudo-errno from vendor/linux/include/linux/errno.h.
const ERESTARTSYS: i32 = 512;

/// An absolute futex timeout and the clock against which it expires.
///
/// Linux converts relative `FUTEX_WAIT` timeouts to an absolute monotonic
/// `ktime_t` in `futex_init_timeout()` and leaves `FUTEX_WAIT_BITSET`/futex2
/// deadlines absolute on their selected clock.  Carrying that representation
/// into the wait core avoids restarting timeout accounting after user-copy or
/// waiter setup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FutexDeadline {
    clock: crate::kernel::time::hrtimer::ClockBase,
    expires_ns: u64,
}

impl FutexDeadline {
    pub const fn monotonic(expires_ns: u64) -> Self {
        Self {
            clock: crate::kernel::time::hrtimer::ClockBase::Monotonic,
            expires_ns,
        }
    }

    pub const fn realtime(expires_ns: u64) -> Self {
        Self {
            clock: crate::kernel::time::hrtimer::ClockBase::Realtime,
            expires_ns,
        }
    }

    pub fn relative_monotonic(duration_ns: u64) -> Self {
        Self::monotonic(crate::kernel::time::ktime_get().saturating_add(duration_ns))
    }

    fn now(self) -> u64 {
        match self.clock {
            crate::kernel::time::hrtimer::ClockBase::Realtime
            | crate::kernel::time::hrtimer::ClockBase::Tai => crate::kernel::time::ktime_get_real(),
            crate::kernel::time::hrtimer::ClockBase::Boottime => {
                crate::kernel::time::ktime_get_boottime()
            }
            crate::kernel::time::hrtimer::ClockBase::Monotonic
            | crate::kernel::time::hrtimer::ClockBase::MonotonicRaw => {
                crate::kernel::time::ktime_get()
            }
        }
    }
}

/// Stack-resident equivalent of Linux `struct hrtimer_sleeper` used by the
/// futex wait paths.  The timer is the first field so its callback can recover
/// the containing sleeper exactly as Linux's `container_of()` does.
#[repr(C)]
struct FutexTimeoutSleeper {
    timer: crate::kernel::time::hrtimer::Hrtimer,
    task: AtomicUsize,
    deadline: FutexDeadline,
}

impl FutexTimeoutSleeper {
    fn new(task: *mut crate::kernel::task::TaskStruct, deadline: FutexDeadline) -> Self {
        let mut timeout = Self {
            timer: crate::kernel::time::hrtimer::Hrtimer::new(),
            task: AtomicUsize::new(task as usize),
            deadline,
        };
        crate::kernel::time::hrtimer::hrtimer_init(
            &mut timeout.timer,
            deadline.clock,
            crate::kernel::time::hrtimer::HrtimerMode::Abs,
        );
        timeout.timer.function = Some(futex_timeout_wake);
        timeout
    }

    fn start(&mut self) {
        // Linux's hrtimer_sleeper_start_expires() does not enqueue an already
        // expired userspace timer.  Preserve that fast path even though the
        // current Lupos clockevent backend services hrtimers from the tick.
        if self.deadline.now() >= self.deadline.expires_ns {
            self.task.store(0, Ordering::Release);
            return;
        }
        crate::kernel::time::hrtimer::hrtimer_start(
            &mut self.timer,
            self.deadline.expires_ns,
            crate::kernel::time::hrtimer::HrtimerMode::Abs,
        );
    }

    fn expired(&self) -> bool {
        self.task.load(Ordering::Acquire) == 0 || self.deadline.now() >= self.deadline.expires_ns
    }

    fn cancel(&mut self) {
        let _ = crate::kernel::time::hrtimer::hrtimer_cancel(&mut self.timer);
        self.task.store(0, Ordering::Release);
    }
}

fn futex_timeout_wake(
    timer: *mut crate::kernel::time::hrtimer::Hrtimer,
) -> crate::kernel::time::hrtimer::HrtimerRestart {
    if timer.is_null() {
        return crate::kernel::time::hrtimer::HrtimerRestart::NoRestart;
    }
    let timeout = timer.cast::<FutexTimeoutSleeper>();
    let task = unsafe { (*timeout).task.swap(0, Ordering::AcqRel) }
        as *mut crate::kernel::task::TaskStruct;
    if !task.is_null() {
        unsafe {
            crate::kernel::sched::wake_task_normal(task);
        }
    }
    crate::kernel::time::hrtimer::HrtimerRestart::NoRestart
}

fn finish_futex_timeout(
    timeout: &mut FutexTimeoutSleeper,
    has_timeout: bool,
    task: *mut crate::kernel::task::TaskStruct,
) {
    if !task.is_null() {
        unsafe {
            (*task).__state.store(
                crate::kernel::task::task_state::TASK_RUNNING,
                Ordering::Release,
            );
        }
    }
    if has_timeout {
        timeout.cancel();
    }
}

// ── Hash bucket (vendor/linux/kernel/futex/core.c::futex_hash) ───────────────

/// Number of buckets — must be a power of two for fast hashing.  Linux uses
/// `roundup_pow_of_two(8 * num_possible_cpus())`; we hardcode 256 for the
/// in-kernel build (covers all M22 kthreads with comfortable headroom).
pub const FUTEX_HASH_BUCKETS: usize = 256;

/// One waiter parked on a futex.
pub struct FutexQ {
    pub uaddr: u64,
    pub mm_id: u64,
    pub bitset: u32,
    pub task: *mut crate::kernel::task::TaskStruct,
    pub task_pid: i32,
    pub waitv_id: u64,
    pub waitv_index: usize,
    pub requeue_pi: bool,
    /// Wakeup flag — set to 1 when `futex_wake` selects this entry.
    pub awoken: bool,
}

// Futex waiters are only accessed while holding their bucket lock; the raw
// task pointer is an opaque scheduler handle rather than shared interior data.
unsafe impl Send for FutexQ {}
unsafe impl Sync for FutexQ {}

/// Hash bucket — list of FutexQ entries for the same `(mm, uaddr)` bucket.
struct Bucket {
    waiters: Vec<FutexQ>,
}

impl Bucket {
    const fn new() -> Self {
        Self {
            waiters: Vec::new(),
        }
    }
}

static BUCKETS: [Mutex<Bucket>; FUTEX_HASH_BUCKETS] = [const {
    Mutex::new(Bucket {
        waiters: Vec::new(),
    })
}; FUTEX_HASH_BUCKETS];
static WAITV_SEQ: AtomicU64 = AtomicU64::new(1);
static PRIVATE_HASH_STATE: Mutex<Vec<PrivateHashState>> = Mutex::new(Vec::new());
#[cfg(test)]
static FUTEX_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
struct PrivateHashState {
    mm_id: u64,
    slots: u32,
    user_configured: bool,
    global_requested: bool,
}

#[inline]
fn hash_key(mm_id: u64, uaddr: u64) -> usize {
    let mixed = mm_id.wrapping_mul(0x9E3779B97F4A7C15) ^ uaddr.wrapping_mul(0xBF58476D1CE4E5B9);
    (mixed as usize) & (FUTEX_HASH_BUCKETS - 1)
}

/// Resolve the mm-id for the calling task — for FUTEX_PRIVATE_FLAG the
/// per-process address space; for shared we use 0 (single global namespace).
fn mm_for(private: bool) -> u64 {
    if private {
        // Host unit tests don't have a working LAPIC, so `get_current()` would
        // page-fault.  Use a constant pseudo-mm so tests stay deterministic;
        // the real per-mm key is restored when `cfg(target_os = "none")`.
        #[cfg(not(test))]
        unsafe {
            let cur = crate::kernel::sched::get_current();
            if cur.is_null() {
                return 0;
            }
            let mm = if !(*cur).mm.is_null() {
                (*cur).mm
            } else {
                (*cur).active_mm
            };
            return if mm.is_null() { cur as u64 } else { mm as u64 };
        }
        #[cfg(test)]
        return 0xCAFEBABE_DEADBEEF_u64;
    } else {
        0
    }
}

pub(crate) fn futex_private_mm_id() -> u64 {
    mm_for(true)
}

fn private_hash_default_slots() -> u32 {
    16
}

pub fn futex_private_hash_note_clone(mm_id: u64) {
    if mm_id == 0 {
        return;
    }
    let mut states = PRIVATE_HASH_STATE.lock();
    if let Some(state) = states.iter_mut().find(|state| state.mm_id == mm_id) {
        if !state.user_configured && !state.global_requested && state.slots == 0 {
            state.slots = private_hash_default_slots();
        }
        return;
    }
    states.push(PrivateHashState {
        mm_id,
        slots: private_hash_default_slots(),
        user_configured: false,
        global_requested: false,
    });
}

pub fn futex_private_hash_mm_destroy(mm_id: u64) {
    let mut states = PRIVATE_HASH_STATE.lock();
    states.retain(|state| state.mm_id != mm_id);
}

fn futex_hash_slots_valid(slots: u32) -> bool {
    slots >= 2 && slots <= (1 << 20) && slots.is_power_of_two()
}

pub fn futex_private_hash_get_slots() -> i64 {
    let mm_id = futex_private_mm_id();
    let states = PRIVATE_HASH_STATE.lock();
    states
        .iter()
        .find(|state| state.mm_id == mm_id)
        .map(|state| state.slots as i64)
        .unwrap_or(0)
}

pub fn futex_private_hash_set_slots(slots: u32) -> i64 {
    let mm_id = futex_private_mm_id();
    if mm_id == 0 {
        return -EINVAL as i64;
    }
    let mut states = PRIVATE_HASH_STATE.lock();
    let idx = if let Some(idx) = states.iter().position(|state| state.mm_id == mm_id) {
        idx
    } else {
        states.push(PrivateHashState {
            mm_id,
            slots: 0,
            user_configured: false,
            global_requested: false,
        });
        states.len() - 1
    };
    let state = &mut states[idx];
    if state.global_requested {
        return -EINVAL as i64;
    }
    if slots == 0 {
        state.slots = 0;
        state.user_configured = true;
        state.global_requested = true;
        return 0;
    }
    if !futex_hash_slots_valid(slots) {
        return -EINVAL as i64;
    }
    state.slots = slots;
    state.user_configured = true;
    0
}

#[inline]
fn futex_kernel_pointer_allowed() -> bool {
    #[cfg(test)]
    {
        true
    }
    #[cfg(not(test))]
    unsafe {
        let cur = crate::kernel::sched::get_current();
        cur.is_null() || (*cur).mm.is_null()
    }
}

#[inline]
fn futex_uaddr_valid(uaddr: u64) -> Result<bool, i32> {
    if uaddr == 0 {
        return Err(EFAULT);
    }
    if uaddr & 3 != 0 {
        return Err(EINVAL);
    }
    if futex_kernel_pointer_allowed() {
        return Ok(false);
    }
    if crate::arch::x86::kernel::uaccess::access_ok(uaddr, 4) {
        return Ok(true);
    }
    Err(EFAULT)
}

/// Read the futex word through the same user-access fault boundary as Linux
/// `get_user()`.
pub(crate) unsafe fn futex_get(uaddr: u64) -> Result<u32, i32> {
    if futex_uaddr_valid(uaddr)? {
        unsafe { crate::arch::x86::kernel::uaccess::get_user_u32(uaddr as *const u32) }
    } else {
        let p = uaddr as *const AtomicU32;
        Ok(unsafe { (*p).load(Ordering::Acquire) })
    }
}

pub(crate) unsafe fn futex_put(uaddr: u64, value: u32) -> Result<(), i32> {
    if futex_uaddr_valid(uaddr)? {
        unsafe { crate::arch::x86::kernel::uaccess::put_user_u32(uaddr as *mut u32, value) }
    } else {
        let p = uaddr as *const AtomicU32;
        unsafe { (*p).store(value, Ordering::Release) };
        Ok(())
    }
}

#[cfg(not(test))]
fn futex_user_range_has_perm(uaddr: u64, size: u64, write: bool) -> bool {
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() || unsafe { (*cur).mm }.is_null() {
        return true;
    }
    let mm = unsafe { &*(*cur).mm };
    let Some(vma) = crate::mm::vma::find_vma(mm, uaddr) else {
        return false;
    };
    let end = match uaddr.checked_add(size) {
        Some(end) => end,
        None => return false,
    };
    let flags = unsafe { (*vma).vm_flags };
    if uaddr < unsafe { (*vma).vm_start } || end > unsafe { (*vma).vm_end } {
        return false;
    }
    if flags & crate::mm::vm_flags::VM_READ == 0 {
        return false;
    }
    !write || flags & crate::mm::vm_flags::VM_WRITE != 0
}

#[cfg(test)]
fn futex_user_range_has_perm(_uaddr: u64, _size: u64, _write: bool) -> bool {
    true
}

pub(crate) unsafe fn futex2_prepare_key(uaddr: u64, flags: u32) -> Result<(), i32> {
    if flags & !FUTEX2_VALID_MASK != 0 || flags & FUTEX2_SIZE_MASK != FUTEX_32 {
        return Err(EINVAL);
    }
    let size = if flags & FUTEX2_NUMA != 0 { 8 } else { 4 };
    if uaddr == 0 {
        return Err(EFAULT);
    }
    if uaddr & (size - 1) != 0 {
        return Err(EINVAL);
    }
    if !futex_kernel_pointer_allowed() && !crate::arch::x86::kernel::uaccess::access_ok(uaddr, size)
    {
        return Err(EFAULT);
    }
    if flags & FUTEX2_NUMA == 0 {
        return Ok(());
    }
    if !futex_user_range_has_perm(uaddr, size, false) {
        return Err(EFAULT);
    }

    let node_addr = uaddr + 4;
    let node = unsafe { futex_get(node_addr)? };
    if node != u32::MAX {
        let online = crate::mm::mempolicy::online_nodes();
        if node as usize >= u64::BITS as usize || (online & (1u64 << node)) == 0 {
            return Err(EINVAL);
        }
        return Ok(());
    }

    if !futex_user_range_has_perm(node_addr, 4, true) {
        return Err(EFAULT);
    }
    let node = if flags & FUTEX2_MPOL != 0 {
        crate::mm::mempolicy::select_node_for_address(uaddr).unwrap_or(0) as u32
    } else {
        crate::mm::page_alloc::numa_node().max(0) as u32
    };
    unsafe { futex_put(node_addr, node)? };
    Ok(())
}

fn remove_waiter(bucket_idx: usize, pid: i32, uaddr: u64) {
    let mut b = BUCKETS[bucket_idx].lock();
    if let Some(idx) = b
        .waiters
        .iter()
        .position(|w| w.task_pid == pid && w.uaddr == uaddr)
    {
        b.waiters.remove(idx);
    }
}

fn remove_waitv_waiters(waitv_id: u64) {
    for bucket in BUCKETS.iter() {
        let mut b = bucket.lock();
        let mut i = 0;
        while i < b.waiters.len() {
            if b.waiters[i].waitv_id == waitv_id {
                b.waiters.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

/// Unqueue a complete waitv set while retaining a wake which won the bucket
/// lock before timeout or signal cleanup.  Removing each entry while its
/// bucket is locked gives the same wake-vs-unqueue ordering that Linux derives
/// from `futex_unqueue_multiple()` and each `q->lock_ptr`.
fn unqueue_waitv_waiters(waitv_id: u64) -> Option<usize> {
    let mut woken = None;
    for bucket in BUCKETS.iter() {
        let mut b = bucket.lock();
        let mut i = 0;
        while i < b.waiters.len() {
            if b.waiters[i].waitv_id != waitv_id {
                i += 1;
                continue;
            }
            let waiter = b.waiters.remove(i);
            if waiter.awoken && woken.is_none() {
                woken = Some(waiter.waitv_index);
            }
        }
    }
    woken
}

fn remove_waiter_any_bucket(pid: i32, uaddr: u64) {
    let _ = unqueue_waiter_any_bucket(pid, uaddr);
}

fn unqueue_waiter_any_bucket(pid: i32, uaddr: u64) -> Option<bool> {
    for bucket in BUCKETS.iter() {
        let mut b = bucket.lock();
        if let Some(idx) = b
            .waiters
            .iter()
            .position(|w| w.task_pid == pid && (w.uaddr == uaddr || w.requeue_pi))
        {
            return Some(b.waiters.remove(idx).awoken);
        }
    }
    None
}

fn waiter_location(pid: i32, original_uaddr: u64, requeue_pi: bool) -> Option<(usize, u64, bool)> {
    for (bucket_idx, bucket) in BUCKETS.iter().enumerate() {
        let b = bucket.lock();
        if let Some(waiter) = b.waiters.iter().find(|w| {
            w.task_pid == pid && (w.uaddr == original_uaddr || (requeue_pi && w.requeue_pi))
        }) {
            return Some((bucket_idx, waiter.uaddr, waiter.awoken));
        }
    }
    None
}

fn remove_waiter_at(bucket_idx: usize, pid: i32, uaddr: u64) -> bool {
    unqueue_waiter_at(bucket_idx, pid, uaddr).is_some()
}

fn unqueue_waiter_at(bucket_idx: usize, pid: i32, uaddr: u64) -> Option<bool> {
    let mut b = BUCKETS[bucket_idx].lock();
    if let Some(idx) = b
        .waiters
        .iter()
        .position(|w| w.task_pid == pid && w.uaddr == uaddr)
    {
        Some(b.waiters.remove(idx).awoken)
    } else {
        None
    }
}

pub unsafe fn futex_exit_release(task: *mut crate::kernel::task::TaskStruct) {
    if task.is_null() {
        return;
    }
    unsafe {
        super::pi::futex_pi_exit_release(task);
    }
    let pid = unsafe { (*task).pid };
    for bucket in BUCKETS.iter() {
        let mut b = bucket.lock();
        let mut i = 0;
        while i < b.waiters.len() {
            if b.waiters[i].task == task || b.waiters[i].task_pid == pid {
                b.waiters.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

fn take_awoken_waitv_index(waitv_id: u64) -> Option<usize> {
    let mut hit = None;
    for bucket in BUCKETS.iter() {
        let mut b = bucket.lock();
        let mut i = 0;
        while i < b.waiters.len() {
            if b.waiters[i].waitv_id == waitv_id {
                if b.waiters[i].awoken && hit.is_none() {
                    hit = Some(b.waiters[i].waitv_index);
                }
                if hit.is_some() {
                    b.waiters.remove(i);
                    continue;
                }
            }
            i += 1;
        }
    }
    if hit.is_some() {
        remove_waitv_waiters(waitv_id);
    }
    hit
}

// ── futex_wait ───────────────────────────────────────────────────────────────

/// Linux `futex_wait(uaddr, val, bitset, timeout)`.
///
/// Returns 0 on wakeup, -EAGAIN if `*uaddr != val`, -ETIMEDOUT on timeout,
/// -EINVAL on invalid arguments.
unsafe fn futex_wait_impl(
    uaddr: u64,
    val: u32,
    bitset: u32,
    deadline: Option<FutexDeadline>,
    private: bool,
    requeue_pi: bool,
) -> i64 {
    if bitset == 0 {
        return -EINVAL as i64;
    }
    if let Err(errno) = futex_uaddr_valid(uaddr) {
        return -(errno as i64);
    }

    let mm_id = mm_for(private);
    let bucket_idx = hash_key(mm_id, uaddr);
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() {
        let observed = match unsafe { futex_get(uaddr) } {
            Ok(value) => value,
            Err(errno) => return -(errno as i64),
        };
        return if observed == val {
            -ETIMEDOUT as i64
        } else {
            -EAGAIN as i64
        };
    }
    let pid = if cur.is_null() {
        0
    } else {
        unsafe { (*cur).pid }
    };
    let has_timeout = deadline.is_some();
    let mut timeout =
        FutexTimeoutSleeper::new(cur, deadline.unwrap_or(FutexDeadline::monotonic(0)));
    {
        let mut b = BUCKETS[bucket_idx].lock();
        let observed = match unsafe { futex_get(uaddr) } {
            Ok(value) => value,
            Err(errno) => return -(errno as i64),
        };
        if observed != val {
            return -EAGAIN as i64;
        }
        unsafe {
            (*cur).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                Ordering::Release,
            );
        }
        b.waiters.push(FutexQ {
            uaddr,
            mm_id,
            bitset,
            task: cur,
            task_pid: pid,
            waitv_id: 0,
            waitv_index: 0,
            requeue_pi,
            awoken: false,
        });
    }
    if has_timeout {
        timeout.start();
    }

    loop {
        // A spurious scheduler wake leaves the futex queued.  Publish the
        // interruptible state again before inspecting persistent wake, timer,
        // and signal conditions so none of them can be lost before schedule.
        unsafe {
            (*cur).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                Ordering::Release,
            );
        }
        if requeue_pi
            && (crate::kernel::signal::has_pending_signals(cur)
                || crate::kernel::signal::has_current_pending_signal_mask(
                    1u64 << crate::kernel::signal::SIGUSR1,
                ))
        {
            if let Some((requeued_bucket, current_uaddr, _)) =
                waiter_location(pid, uaddr, requeue_pi)
            {
                if current_uaddr != uaddr {
                    let _ = remove_waiter_at(requeued_bucket, pid, current_uaddr);
                    finish_futex_timeout(&mut timeout, has_timeout, cur);
                    return -EAGAIN as i64;
                }
                let _ = crate::kernel::signal::dequeue_current_pending_signal_mask(
                    1u64 << crate::kernel::signal::SIGUSR1,
                );
            }
        }

        if let Some(sig) = crate::kernel::signal::take_current_fatal_signal() {
            if requeue_pi {
                remove_waiter_any_bucket(pid, uaddr);
            } else {
                remove_waiter(bucket_idx, pid, uaddr);
            }
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            unsafe { crate::kernel::signal::exit_current_for_signal(sig) };
        }

        // Drain check.
        let mut woken = false;
        let mut timed_out = false;
        {
            if requeue_pi {
                if let Some((current_bucket, current_uaddr, awoken)) =
                    waiter_location(pid, uaddr, requeue_pi)
                {
                    if awoken {
                        let _ = remove_waiter_at(current_bucket, pid, current_uaddr);
                        woken = true;
                    }
                }
            } else {
                let mut b = BUCKETS[bucket_idx].lock();
                if let Some(idx) = b
                    .waiters
                    .iter()
                    .position(|w| w.task_pid == pid && w.uaddr == uaddr)
                {
                    if b.waiters[idx].awoken {
                        b.waiters.remove(idx);
                        woken = true;
                    }
                }
            }
            if !woken && has_timeout && timeout.expired() {
                let wake_won = if requeue_pi {
                    if let Some((current_bucket, current_uaddr, _)) =
                        waiter_location(pid, uaddr, requeue_pi)
                    {
                        unqueue_waiter_at(current_bucket, pid, current_uaddr)
                    } else {
                        None
                    }
                } else {
                    unqueue_waiter_at(bucket_idx, pid, uaddr)
                };
                if wake_won == Some(true) {
                    woken = true;
                } else {
                    timed_out = true;
                }
            }
        }
        if woken {
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return 0;
        }
        if timed_out {
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -ETIMEDOUT as i64;
        }
        if crate::kernel::signal::has_unblocked_pending_signals(cur) {
            let wake_won = if requeue_pi {
                unqueue_waiter_any_bucket(pid, uaddr)
            } else {
                unqueue_waiter_at(bucket_idx, pid, uaddr)
            };
            if wake_won == Some(true) {
                finish_futex_timeout(&mut timeout, has_timeout, cur);
                return 0;
            }
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            // Linux's futex wait paths return -ERESTARTSYS here; the arch
            // signal-exit path turns it into EINTR or restarts the syscall.
            return -ERESTARTSYS as i64;
        }
        #[cfg(test)]
        if !has_timeout {
            if requeue_pi {
                if let Some((current_bucket, current_uaddr, _)) =
                    waiter_location(pid, uaddr, requeue_pi)
                {
                    let _ = remove_waiter_at(current_bucket, pid, current_uaddr);
                }
            } else {
                let mut b = BUCKETS[bucket_idx].lock();
                if let Some(idx) = b
                    .waiters
                    .iter()
                    .position(|w| w.task_pid == pid && w.uaddr == uaddr)
                {
                    b.waiters.remove(idx);
                }
            }
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -ETIMEDOUT as i64;
        }
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
    }
}

pub unsafe fn futex_wait(uaddr: u64, val: u32, bitset: u32, timeout_ns: u64, private: bool) -> i64 {
    let deadline = (timeout_ns != 0).then(|| FutexDeadline::relative_monotonic(timeout_ns));
    unsafe { futex_wait_impl(uaddr, val, bitset, deadline, private, false) }
}

pub unsafe fn futex_wait_deadline(
    uaddr: u64,
    val: u32,
    bitset: u32,
    deadline: Option<FutexDeadline>,
    private: bool,
) -> i64 {
    unsafe { futex_wait_impl(uaddr, val, bitset, deadline, private, false) }
}

pub unsafe fn futex_wait_requeue_pi_prepare(
    uaddr: u64,
    val: u32,
    bitset: u32,
    timeout_ns: u64,
    private: bool,
) -> i64 {
    let deadline = (timeout_ns != 0).then(|| FutexDeadline::relative_monotonic(timeout_ns));
    unsafe { futex_wait_impl(uaddr, val, bitset, deadline, private, true) }
}

// ── futex_wake ───────────────────────────────────────────────────────────────

/// Linux `futex_wake(uaddr, bitset, nr_to_wake)`.
///
/// Returns the number of waiters actually woken (>= 0), or -EINVAL.
pub unsafe fn futex_wake(uaddr: u64, nr: i32, bitset: u32, private: bool) -> i64 {
    if bitset == 0 {
        return -EINVAL as i64;
    }
    if let Err(errno) = futex_uaddr_valid(uaddr) {
        return -(errno as i64);
    }
    if nr <= 0 {
        return 0;
    }
    let mm_id = mm_for(private);
    let bucket_idx = hash_key(mm_id, uaddr);
    let mut woken = 0i64;
    let mut wake_tasks: Vec<*mut crate::kernel::task::TaskStruct> = Vec::new();
    let mut b = BUCKETS[bucket_idx].lock();
    for w in b.waiters.iter_mut() {
        if woken >= nr as i64 {
            break;
        }
        if w.uaddr == uaddr && w.mm_id == mm_id && (w.bitset & bitset) != 0 && !w.awoken {
            w.awoken = true;
            wake_tasks.push(w.task);
            woken += 1;
        }
    }
    drop(b);
    for task in wake_tasks {
        unsafe {
            crate::kernel::sched::wake_task(task);
        }
    }
    woken
}

// ── futex_requeue / futex_wake_op (skeleton) ─────────────────────────────────

pub unsafe fn futex_requeue(
    uaddr1: u64,
    uaddr2: u64,
    nr_wake: i32,
    nr_requeue: i32,
    cmpval: u32,
    cmp: bool,
    private: bool,
) -> i64 {
    if nr_wake < 0 || nr_requeue < 0 {
        return -EINVAL as i64;
    }
    if let Err(errno) = futex_uaddr_valid(uaddr1) {
        return -(errno as i64);
    }
    if let Err(errno) = futex_uaddr_valid(uaddr2) {
        return -(errno as i64);
    }
    if cmp {
        let observed = match unsafe { futex_get(uaddr1) } {
            Ok(value) => value,
            Err(errno) => return -(errno as i64),
        };
        if observed != cmpval {
            return -EAGAIN as i64;
        }
    }
    let mut woken = 0i64;
    let mut requeued = 0i64;
    let mm_id = mm_for(private);
    let src_idx = hash_key(mm_id, uaddr1);
    let dst_idx = hash_key(mm_id, uaddr2);

    let mut wake_tasks: Vec<*mut crate::kernel::task::TaskStruct> = Vec::new();
    if src_idx == dst_idx {
        let mut b = BUCKETS[src_idx].lock();
        for w in b.waiters.iter_mut() {
            if woken >= nr_wake as i64 {
                break;
            }
            if w.uaddr == uaddr1 && w.mm_id == mm_id && !w.awoken {
                w.awoken = true;
                wake_tasks.push(w.task);
                woken += 1;
            }
        }
        for w in b.waiters.iter_mut() {
            if requeued >= nr_requeue as i64 {
                break;
            }
            if w.uaddr == uaddr1 && w.mm_id == mm_id && !w.awoken {
                w.uaddr = uaddr2;
                requeued += 1;
            }
        }
    } else {
        let mut src = BUCKETS[src_idx].lock();
        for w in src.waiters.iter_mut() {
            if woken >= nr_wake as i64 {
                break;
            }
            if w.uaddr == uaddr1 && w.mm_id == mm_id && !w.awoken {
                w.awoken = true;
                wake_tasks.push(w.task);
                woken += 1;
            }
        }
        let mut moved: Vec<FutexQ> = Vec::new();
        let mut i = 0;
        while i < src.waiters.len() && requeued < nr_requeue as i64 {
            if src.waiters[i].uaddr == uaddr1
                && src.waiters[i].mm_id == mm_id
                && !src.waiters[i].awoken
            {
                let mut entry = src.waiters.remove(i);
                entry.uaddr = uaddr2;
                moved.push(entry);
                requeued += 1;
            } else {
                i += 1;
            }
        }
        drop(src);
        let mut dst = BUCKETS[dst_idx].lock();
        dst.waiters.append(&mut moved);
    }

    for task in wake_tasks {
        unsafe {
            crate::kernel::sched::wake_task(task);
        }
    }

    woken + requeued
}

pub unsafe fn futex_requeue_pi_checked(
    uaddr1: u64,
    uaddr2: u64,
    nr_wake: i32,
    nr_requeue: i32,
    cmpval: u32,
    private: bool,
) -> i64 {
    if nr_wake < 0 || nr_requeue < 0 {
        return -EINVAL as i64;
    }
    if let Err(errno) = futex_uaddr_valid(uaddr1) {
        return -(errno as i64);
    }
    if let Err(errno) = futex_uaddr_valid(uaddr2) {
        return -(errno as i64);
    }
    let observed = match unsafe { futex_get(uaddr1) } {
        Ok(value) => value,
        Err(errno) => return -(errno as i64),
    };
    if observed != cmpval {
        return -EAGAIN as i64;
    }

    let mm_id = mm_for(private);
    let src_idx = hash_key(mm_id, uaddr1);
    let dst_idx = hash_key(mm_id, uaddr2);
    {
        let b = BUCKETS[src_idx].lock();
        if b.waiters
            .iter()
            .any(|w| w.uaddr == uaddr1 && w.mm_id == mm_id && !w.requeue_pi)
        {
            return -EINVAL as i64;
        }
    }

    let limit = (nr_wake as i64).saturating_add(nr_requeue as i64);
    if limit <= 0 {
        return 0;
    }
    let target_word = match unsafe { futex_get(uaddr2) } {
        Ok(value) => value,
        Err(errno) => return -(errno as i64),
    };

    let mut selected: Vec<FutexQ> = Vec::new();
    let mut existing_target_waiters = false;
    if src_idx == dst_idx {
        let mut bucket = BUCKETS[src_idx].lock();
        let mut i = 0;
        while i < bucket.waiters.len() && (selected.len() as i64) < limit {
            if bucket.waiters[i].uaddr == uaddr1
                && bucket.waiters[i].mm_id == mm_id
                && bucket.waiters[i].requeue_pi
                && !bucket.waiters[i].awoken
            {
                let mut waiter = bucket.waiters.remove(i);
                waiter.uaddr = uaddr2;
                selected.push(waiter);
            } else {
                i += 1;
            }
        }
        existing_target_waiters = bucket
            .waiters
            .iter()
            .any(|w| w.uaddr == uaddr2 && w.mm_id == mm_id && w.requeue_pi && !w.awoken);
    } else {
        let mut src = BUCKETS[src_idx].lock();
        let mut i = 0;
        while i < src.waiters.len() && (selected.len() as i64) < limit {
            if src.waiters[i].uaddr == uaddr1
                && src.waiters[i].mm_id == mm_id
                && src.waiters[i].requeue_pi
                && !src.waiters[i].awoken
            {
                let mut waiter = src.waiters.remove(i);
                waiter.uaddr = uaddr2;
                selected.push(waiter);
            } else {
                i += 1;
            }
        }
        drop(src);
        let dst = BUCKETS[dst_idx].lock();
        existing_target_waiters = dst
            .waiters
            .iter()
            .any(|w| w.uaddr == uaddr2 && w.mm_id == mm_id && w.requeue_pi && !w.awoken);
    }

    if selected.is_empty() {
        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        return 0;
    }

    let mut wake_task = core::ptr::null_mut();
    let target_owner = target_word & FUTEX_TID_MASK;
    let target_free = target_owner == 0;
    let mut owner_tid = 0;
    if target_free {
        selected[0].awoken = true;
        wake_task = selected[0].task;
        owner_tid = (selected[0].task_pid as u32) & FUTEX_TID_MASK;
    }

    let selected_count = selected.len() as i64;
    let queued_after_handoff =
        existing_target_waiters || selected.iter().any(|waiter| !waiter.awoken);

    {
        let mut dst = BUCKETS[dst_idx].lock();
        dst.waiters.append(&mut selected);
    }

    let next_word = if target_free {
        owner_tid
            | if queued_after_handoff {
                FUTEX_WAITERS
            } else {
                0
            }
    } else {
        target_word | FUTEX_WAITERS
    };
    if let Err(errno) = unsafe { futex_put(uaddr2, next_word) } {
        return -(errno as i64);
    }
    if !wake_task.is_null() {
        unsafe {
            crate::kernel::sched::wake_task(wake_task);
        }
    }
    selected_count
}

pub unsafe fn futex_pi_wake_next(uaddr: u64, private: bool) -> bool {
    if futex_uaddr_valid(uaddr).is_err() {
        return false;
    }
    let mm_id = mm_for(private);
    let bucket_idx = hash_key(mm_id, uaddr);
    let mut wake_task = core::ptr::null_mut();
    let mut owner_tid = 0u32;
    let mut more_waiters = false;
    {
        let mut bucket = BUCKETS[bucket_idx].lock();
        if let Some(pos) = bucket
            .waiters
            .iter()
            .position(|w| w.uaddr == uaddr && w.mm_id == mm_id && w.requeue_pi && !w.awoken)
        {
            bucket.waiters[pos].awoken = true;
            wake_task = bucket.waiters[pos].task;
            owner_tid = (bucket.waiters[pos].task_pid as u32) & FUTEX_TID_MASK;
            more_waiters = bucket.waiters.iter().enumerate().any(|(idx, waiter)| {
                idx != pos
                    && waiter.uaddr == uaddr
                    && waiter.mm_id == mm_id
                    && waiter.requeue_pi
                    && !waiter.awoken
            });
        }
    }
    if owner_tid == 0 {
        return false;
    }
    let next_word = owner_tid | if more_waiters { FUTEX_WAITERS } else { 0 };
    if unsafe { futex_put(uaddr, next_word) }.is_err() {
        return false;
    }
    if !wake_task.is_null() {
        unsafe {
            crate::kernel::sched::wake_task(wake_task);
        }
    }
    true
}

fn sign_extend_12(value: u32) -> i32 {
    let value = value & 0x0fff;
    if value & 0x0800 != 0 {
        (value | 0xffff_f000) as i32
    } else {
        value as i32
    }
}

unsafe fn futex_cmpxchg_value(
    uaddr: u64,
    user_addr: bool,
    expected: u32,
    new: u32,
) -> Result<u32, i32> {
    if user_addr {
        unsafe {
            crate::arch::x86::kernel::uaccess::cmpxchg_user_u32(uaddr as *mut u32, expected, new)
        }
    } else {
        let atomic = uaddr as *const AtomicU32;
        match unsafe {
            (*atomic).compare_exchange(expected, new, Ordering::AcqRel, Ordering::Acquire)
        } {
            Ok(prev) => Ok(prev),
            Err(prev) => Ok(prev),
        }
    }
}

unsafe fn futex_atomic_op_inuser(encoded_op: u32, uaddr: u64) -> i64 {
    let user_addr = match futex_uaddr_valid(uaddr) {
        Ok(user_addr) => user_addr,
        Err(errno) => return -(errno as i64),
    };

    let op = (encoded_op >> 28) & 0x7;
    let cmp = (encoded_op >> 24) & 0xf;
    let mut oparg = sign_extend_12((encoded_op >> 12) & 0x0fff);
    let cmparg = sign_extend_12(encoded_op & 0x0fff);

    if encoded_op & (FUTEX_OP_OPARG_SHIFT << 28) != 0 {
        if !(0..=31).contains(&oparg) {
            oparg &= 31;
        }
        oparg = 1_i32.wrapping_shl(oparg as u32);
    }

    let oldval = match op {
        FUTEX_OP_SET => {
            let mut observed = match unsafe { futex_get(uaddr) } {
                Ok(value) => value,
                Err(errno) => return -(errno as i64),
            };
            loop {
                let previous = match unsafe {
                    futex_cmpxchg_value(uaddr, user_addr, observed, oparg as u32)
                } {
                    Ok(previous) => previous,
                    Err(errno) => return -(errno as i64),
                };
                if previous == observed {
                    break previous;
                }
                observed = previous;
            }
        }
        FUTEX_OP_ADD | FUTEX_OP_OR | FUTEX_OP_ANDN | FUTEX_OP_XOR => {
            let mut observed = match unsafe { futex_get(uaddr) } {
                Ok(value) => value,
                Err(errno) => return -(errno as i64),
            };
            loop {
                let next = match op {
                    FUTEX_OP_ADD => (observed as i32).wrapping_add(oparg) as u32,
                    FUTEX_OP_OR => observed | oparg as u32,
                    FUTEX_OP_ANDN => observed & !(oparg as u32),
                    FUTEX_OP_XOR => observed ^ oparg as u32,
                    _ => unreachable!(),
                };
                let previous =
                    match unsafe { futex_cmpxchg_value(uaddr, user_addr, observed, next) } {
                        Ok(previous) => previous,
                        Err(errno) => return -(errno as i64),
                    };
                if previous == observed {
                    break previous;
                }
                observed = previous;
            }
        }
        _ => return -ENOSYS as i64,
    } as i32;

    let matched = match cmp {
        FUTEX_OP_CMP_EQ => oldval == cmparg,
        FUTEX_OP_CMP_NE => oldval != cmparg,
        FUTEX_OP_CMP_LT => oldval < cmparg,
        FUTEX_OP_CMP_LE => oldval <= cmparg,
        FUTEX_OP_CMP_GT => oldval > cmparg,
        FUTEX_OP_CMP_GE => oldval >= cmparg,
        _ => return -ENOSYS as i64,
    };
    if matched { 1 } else { 0 }
}

pub unsafe fn futex_wake_op(
    uaddr1: u64,
    uaddr2: u64,
    nr_wake: i32,
    nr_wake2: i32,
    op: u32,
    private: bool,
) -> i64 {
    if nr_wake < 0 || nr_wake2 < 0 {
        return -EINVAL as i64;
    }
    let op_ret = unsafe { futex_atomic_op_inuser(op, uaddr2) };
    if op_ret < 0 {
        return op_ret;
    }
    let woken1 = unsafe { futex_wake(uaddr1, nr_wake, super::FUTEX_BITSET_MATCH_ANY, private) };
    if woken1 < 0 {
        return woken1;
    }
    if op_ret == 0 {
        return woken1;
    }
    let woken2 = unsafe { futex_wake(uaddr2, nr_wake2, super::FUTEX_BITSET_MATCH_ANY, private) };
    if woken2 < 0 {
        return woken2;
    }
    woken1 + woken2
}

pub unsafe fn futex_waitv(waiters: &[FutexWaitv], timeout_ns: u64) -> i64 {
    let deadline = (timeout_ns != 0).then(|| FutexDeadline::relative_monotonic(timeout_ns));
    unsafe { futex_waitv_deadline(waiters, deadline) }
}

pub unsafe fn futex_waitv_deadline(waiters: &[FutexWaitv], deadline: Option<FutexDeadline>) -> i64 {
    if waiters.is_empty() || waiters.len() > super::FUTEX_WAITV_MAX {
        return -EINVAL as i64;
    }
    let cur = unsafe { crate::kernel::sched::get_current() };
    if cur.is_null() {
        for waiter in waiters {
            if waiter.val > u32::MAX as u64 {
                return -EINVAL as i64;
            }
            let observed = match unsafe { futex_get(waiter.uaddr) } {
                Ok(value) => value,
                Err(errno) => return -(errno as i64),
            };
            if observed != waiter.val as u32 {
                return -EAGAIN as i64;
            }
        }
        return -ETIMEDOUT as i64;
    }

    let waitv_id = WAITV_SEQ.fetch_add(1, Ordering::AcqRel);
    let pid = unsafe { (*cur).pid };
    let has_timeout = deadline.is_some();
    let mut timeout =
        FutexTimeoutSleeper::new(cur, deadline.unwrap_or(FutexDeadline::monotonic(0)));
    unsafe {
        (*cur).__state.store(
            crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
            Ordering::Release,
        );
    }

    for (index, waiter) in waiters.iter().enumerate() {
        if waiter.flags & FUTEX_32 == 0 || waiter._reserved != 0 {
            remove_waitv_waiters(waitv_id);
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -EINVAL as i64;
        }
        if waiter.val > u32::MAX as u64 {
            remove_waitv_waiters(waitv_id);
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -EINVAL as i64;
        }
        let private = waiter.flags & super::FUTEX2_PRIVATE != 0;
        let mm_id = mm_for(private);
        let bucket_idx = hash_key(mm_id, waiter.uaddr);
        let mut b = BUCKETS[bucket_idx].lock();
        let observed = match unsafe { futex_get(waiter.uaddr) } {
            Ok(value) => value,
            Err(errno) => {
                drop(b);
                remove_waitv_waiters(waitv_id);
                finish_futex_timeout(&mut timeout, has_timeout, cur);
                return -(errno as i64);
            }
        };
        if observed != waiter.val as u32 {
            drop(b);
            remove_waitv_waiters(waitv_id);
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -EAGAIN as i64;
        }
        b.waiters.push(FutexQ {
            uaddr: waiter.uaddr,
            mm_id,
            bitset: super::FUTEX_BITSET_MATCH_ANY,
            task: cur,
            task_pid: pid,
            waitv_id,
            waitv_index: index,
            requeue_pi: false,
            awoken: false,
        });
    }
    if has_timeout {
        timeout.start();
    }

    loop {
        // As in futex_wait_multiple_setup(), state publication precedes every
        // wake/timeout/signal check.  A wake which raced an earlier spurious
        // scheduler return therefore cannot be overwritten before schedule.
        unsafe {
            (*cur).__state.store(
                crate::kernel::task::task_state::TASK_INTERRUPTIBLE,
                Ordering::Release,
            );
        }
        if let Some(index) = take_awoken_waitv_index(waitv_id) {
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return index as i64;
        }
        if has_timeout && timeout.expired() {
            if let Some(index) = unqueue_waitv_waiters(waitv_id) {
                finish_futex_timeout(&mut timeout, has_timeout, cur);
                return index as i64;
            }
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -ETIMEDOUT as i64;
        }
        if crate::kernel::signal::has_unblocked_pending_signals(cur) {
            if let Some(index) = unqueue_waitv_waiters(waitv_id) {
                finish_futex_timeout(&mut timeout, has_timeout, cur);
                return index as i64;
            }
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -ERESTARTSYS as i64;
        }
        #[cfg(test)]
        if !has_timeout {
            remove_waitv_waiters(waitv_id);
            finish_futex_timeout(&mut timeout, has_timeout, cur);
            return -ETIMEDOUT as i64;
        }
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

/// For unit tests — flush all waiters from all buckets.
#[doc(hidden)]
pub fn _flush_for_tests() {
    for b in BUCKETS.iter() {
        b.lock().waiters.clear();
    }
}

#[cfg(test)]
#[doc(hidden)]
pub fn _with_test_lock<R>(f: impl FnOnce() -> R) -> R {
    let _guard = FUTEX_TEST_LOCK.lock();
    f()
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    fn with_test_current<R>(pid: i32, f: impl FnOnce(*mut TaskStruct) -> R) -> R {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = pid;
        current.tgid = pid;
        current.cred = &raw const INIT_CRED;
        let task = &mut *current as *mut TaskStruct;
        unsafe {
            sched::set_current(task);
        }
        let ret = f(task);
        unsafe {
            sched::set_current(previous);
        }
        ret
    }

    #[test]
    fn hash_key_is_pow2_bucket() {
        for mm in 0..4u64 {
            for uaddr in [0u64, 8, 16, 0xFFFF_0000].iter() {
                let h = hash_key(mm, *uaddr);
                assert!(h < FUTEX_HASH_BUCKETS);
            }
        }
    }

    #[test]
    fn hash_key_is_stable() {
        assert_eq!(hash_key(1, 0x1000), hash_key(1, 0x1000));
    }

    #[test]
    fn empty_wake_returns_zero() {
        _with_test_lock(|| {
            _flush_for_tests();
            let woken = unsafe {
                futex_wake(
                    0xdead_beec_u64,
                    1,
                    super::super::FUTEX_BITSET_MATCH_ANY,
                    true,
                )
            };
            assert_eq!(woken, 0);
            _flush_for_tests();
        });
    }

    #[test]
    fn wake_invalid_bitset_returns_einval() {
        let r = unsafe { futex_wake(0x1000, 1, 0, true) };
        assert_eq!(r, -EINVAL as i64);
    }

    #[test]
    fn wait_invalid_bitset_returns_einval() {
        let r = unsafe { futex_wait(0x1000, 0, 0, 0, true) };
        assert_eq!(r, -EINVAL as i64);
    }

    #[test]
    fn futex_wait_returns_erestartsys_for_unblocked_pending_signal() {
        _with_test_lock(|| {
            let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
            crate::kernel::signal::reset_for_tests();
            _flush_for_tests();
            with_test_current(31_001, |task| {
                let futex_word = AtomicU32::new(0);
                let action = crate::kernel::signal::RtSigAction {
                    sa_handler: 0x1234,
                    ..Default::default()
                };
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::sys_rt_sigaction(
                            crate::kernel::signal::SIGTERM,
                            &action,
                            core::ptr::null_mut(),
                            core::mem::size_of::<crate::kernel::signal::SigSet>(),
                        )
                    },
                    0
                );
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::send_signal_to_task(
                            task,
                            crate::kernel::signal::SIGTERM,
                        )
                    },
                    0
                );

                let ret = unsafe {
                    futex_wait(
                        &futex_word as *const AtomicU32 as u64,
                        0,
                        super::super::FUTEX_BITSET_MATCH_ANY,
                        0,
                        true,
                    )
                };

                assert_eq!(ret, -ERESTARTSYS as i64);
            });
            _flush_for_tests();
        });
    }

    #[test]
    fn futex_wait_ignores_blocked_pending_signal() {
        _with_test_lock(|| {
            let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
            crate::kernel::signal::reset_for_tests();
            _flush_for_tests();
            with_test_current(31_002, |task| {
                let mut blocked = crate::kernel::signal::SigSet {
                    bits: 1u64 << (crate::kernel::signal::SIGUSR1 - 1),
                };
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::sys_rt_sigprocmask(
                            crate::kernel::signal::SIG_BLOCK,
                            &mut blocked,
                            core::ptr::null_mut(),
                            core::mem::size_of::<crate::kernel::signal::SigSet>(),
                        )
                    },
                    0
                );
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::send_signal_to_task(
                            task,
                            crate::kernel::signal::SIGUSR1,
                        )
                    },
                    0
                );

                let futex_word = AtomicU32::new(0);
                let ret = unsafe {
                    futex_wait(
                        &futex_word as *const AtomicU32 as u64,
                        0,
                        super::super::FUTEX_BITSET_MATCH_ANY,
                        0,
                        true,
                    )
                };

                assert_eq!(ret, -ETIMEDOUT as i64);
            });
            _flush_for_tests();
        });
    }

    #[test]
    fn futex_waitv_returns_erestartsys_for_unblocked_pending_signal() {
        _with_test_lock(|| {
            let _signal_guard = crate::kernel::signal::SIGNAL_TEST_LOCK.lock();
            crate::kernel::signal::reset_for_tests();
            _flush_for_tests();
            with_test_current(31_003, |task| {
                let futex_word = AtomicU32::new(0);
                let action = crate::kernel::signal::RtSigAction {
                    sa_handler: 0x1234,
                    ..Default::default()
                };
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::sys_rt_sigaction(
                            crate::kernel::signal::SIGTERM,
                            &action,
                            core::ptr::null_mut(),
                            core::mem::size_of::<crate::kernel::signal::SigSet>(),
                        )
                    },
                    0
                );
                assert_eq!(
                    unsafe {
                        crate::kernel::signal::send_signal_to_task(
                            task,
                            crate::kernel::signal::SIGTERM,
                        )
                    },
                    0
                );
                let waiters = [FutexWaitv {
                    val: 0,
                    uaddr: &futex_word as *const AtomicU32 as u64,
                    flags: FUTEX_32,
                    _reserved: 0,
                }];

                let ret = unsafe { futex_waitv(&waiters, 0) };

                assert_eq!(ret, -ERESTARTSYS as i64);
            });
            _flush_for_tests();
        });
    }

    #[test]
    fn requeue_cmp_mismatch_returns_eagain() {
        _with_test_lock(|| {
            _flush_for_tests();
            let src = AtomicU32::new(1);
            let dst = AtomicU32::new(0);
            let r = unsafe {
                futex_requeue(
                    &src as *const AtomicU32 as u64,
                    &dst as *const AtomicU32 as u64,
                    0,
                    1,
                    0,
                    true,
                    true,
                )
            };
            assert_eq!(r, -EAGAIN as i64);
            _flush_for_tests();
        });
    }

    #[test]
    fn requeue_moves_waiter_to_destination_bucket() {
        _with_test_lock(|| {
            _flush_for_tests();
            let src = AtomicU32::new(0);
            let dst = AtomicU32::new(0);
            let src_addr = &src as *const AtomicU32 as u64;
            let dst_addr = &dst as *const AtomicU32 as u64;
            let mm_id = mm_for(true);
            let src_idx = hash_key(mm_id, src_addr);
            let dst_idx = hash_key(mm_id, dst_addr);

            BUCKETS[src_idx].lock().waiters.push(FutexQ {
                uaddr: src_addr,
                mm_id,
                bitset: super::super::FUTEX_BITSET_MATCH_ANY,
                task: core::ptr::null_mut(),
                task_pid: 123,
                waitv_id: 0,
                waitv_index: 0,
                requeue_pi: false,
                awoken: false,
            });

            let moved = unsafe { futex_requeue(src_addr, dst_addr, 0, 1, 0, false, true) };
            assert_eq!(moved, 1);

            let src_has_waiter = BUCKETS[src_idx]
                .lock()
                .waiters
                .iter()
                .any(|w| w.uaddr == src_addr && w.mm_id == mm_id);
            let dst_has_waiter = BUCKETS[dst_idx]
                .lock()
                .waiters
                .iter()
                .any(|w| w.uaddr == dst_addr && w.mm_id == mm_id);
            assert!(
                !src_has_waiter || dst_has_waiter,
                "requeue must not leave a waiter stranded only on the source futex"
            );

            _flush_for_tests();
        });
    }

    #[test]
    fn wake_op_applies_atomic_user_operation_before_wake() {
        _with_test_lock(|| {
            _flush_for_tests();
            let futex1 = AtomicU32::new(0);
            let futex2 = AtomicU32::new(3);
            let op = (super::super::FUTEX_OP_ADD << 28)
                | (super::super::FUTEX_OP_CMP_GE << 24)
                | (2 << 12)
                | 3;

            let ret = unsafe {
                futex_wake_op(
                    &futex1 as *const AtomicU32 as u64,
                    &futex2 as *const AtomicU32 as u64,
                    1,
                    1,
                    op,
                    true,
                )
            };

            assert_eq!(ret, 0);
            assert_eq!(futex2.load(Ordering::Acquire), 5);
            _flush_for_tests();
        });
    }

    #[test]
    fn wake_op_rejects_unknown_operation() {
        let futex1 = AtomicU32::new(0);
        let futex2 = AtomicU32::new(3);
        let op = 7 << 28;
        let ret = unsafe {
            futex_wake_op(
                &futex1 as *const AtomicU32 as u64,
                &futex2 as *const AtomicU32 as u64,
                1,
                1,
                op,
                true,
            )
        };
        assert_eq!(ret, -ENOSYS as i64);
    }
}
