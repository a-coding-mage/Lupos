//! linux-parity: partial
//! linux-deviation: Lupos currently uses one global O(n) intrusive list per clock base rather than Linux's per-CPU rb timerqueues.
//! linux-source: vendor/linux/kernel/time/hrtimer.c
//! test-origin: linux:vendor/linux/kernel/time/hrtimer.c
//! High-resolution timer — `struct hrtimer` (M36).
//!
//! Mirrors `vendor/linux/kernel/time/hrtimer.c`.  M36 ships a per-CPU,
//! 5-clock-base timerqueue keyed by absolute expiry time.  `hrtimer_run_queues`
//! is invoked from `tick_handle_periodic` (and later from a one-shot LAPIC
//! programmable event in M37).

use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use super::timekeeping::{ktime_get, ktime_get_boottime, ktime_get_real};

/// Linux `enum hrtimer_base` — clock bases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ClockBase {
    Monotonic = 0,
    Realtime = 1,
    Boottime = 2,
    Tai = 3,
    MonotonicRaw = 4,
}

pub const NUM_CLOCK_BASES: usize = 5;

/// Linux `enum hrtimer_mode`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum HrtimerMode {
    Abs = 0x00,
    Rel = 0x01,
    Pinned = 0x02,
    Soft = 0x04,
    Hard = 0x08,
}

/// Linux `enum hrtimer_restart`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum HrtimerRestart {
    NoRestart = 0,
    Restart = 1,
}

/// Linux invokes hrtimer callbacks with a raw `struct hrtimer *`.  Keeping the
/// Rust callback ABI raw is equally important: cancellation and remote owner
/// operations may inspect the timer while the callback is running, which
/// cannot soundly overlap an exclusive `&mut Hrtimer` borrow.
pub type HrtimerCallback = fn(*mut Hrtimer) -> HrtimerRestart;

/// `struct hrtimer` — opaque-shape Linux primitive.
#[repr(C)]
pub struct Hrtimer {
    pub expires_ns: u64,
    pub interval_ns: u64,
    pub base: ClockBase,
    pub function: Option<HrtimerCallback>,
    pub data: usize,
    pub state: u8,
    /// Embedded timerqueue links.  As with Linux's `hrtimer::node`, an armed
    /// timer must remain at a stable address until synchronous cancellation
    /// has completed.
    queue_prev: *mut Hrtimer,
    queue_next: *mut Hrtimer,
}

unsafe impl Send for Hrtimer {}
unsafe impl Sync for Hrtimer {}

pub const HRTIMER_STATE_INACTIVE: u8 = 0;
pub const HRTIMER_STATE_ENQUEUED: u8 = 1;

impl Hrtimer {
    pub const fn new() -> Self {
        Self {
            expires_ns: 0,
            interval_ns: 0,
            base: ClockBase::Monotonic,
            function: None,
            data: 0,
            state: HRTIMER_STATE_INACTIVE,
            queue_prev: core::ptr::null_mut(),
            queue_next: core::ptr::null_mut(),
        }
    }

    /// Read the current value of this timer's clock base.
    pub fn base_now(&self) -> u64 {
        match self.base {
            ClockBase::Monotonic | ClockBase::MonotonicRaw => ktime_get(),
            ClockBase::Realtime | ClockBase::Tai => ktime_get_real(),
            ClockBase::Boottime => ktime_get_boottime(),
        }
    }
}

/// `hrtimer_init(timer, base, mode)`.
pub fn hrtimer_init(t: &mut Hrtimer, base: ClockBase, _mode: HrtimerMode) {
    t.base = base;
    t.state = HRTIMER_STATE_INACTIVE;
    t.function = None;
    t.data = 0;
    t.expires_ns = 0;
    t.interval_ns = 0;
    t.queue_prev = core::ptr::null_mut();
    t.queue_next = core::ptr::null_mut();
}

/// Send-safe wrapper around an intrusive `Hrtimer` pointer.  Access to queue
/// links goes through the base lock; pointer stability is the timer owner's
/// responsibility, exactly as for Linux's embedded `timerqueue_node`.
#[derive(Clone, Copy, Eq, PartialEq)]
struct TimerPtr(*mut Hrtimer);
unsafe impl Send for TimerPtr {}

/// Allocation-free expiry-ordered timerqueue.  Linux embeds an rb-node in
/// each hrtimer; this implementation uses the same intrusive ownership model
/// with a sorted doubly-linked queue.  Insertion is linear, while first-timer
/// lookup and removal of an armed timer are constant time.
struct TimerQueue {
    head: Option<TimerPtr>,
    tail: Option<TimerPtr>,
}

impl TimerQueue {
    const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    fn first(&self) -> Option<TimerPtr> {
        self.head
    }

    /// Insert after existing timers with the same expiry, matching the
    /// ordering produced by Linux's strict-less timerqueue comparator.
    fn insert(&mut self, timer: TimerPtr) {
        let p = timer.0;
        debug_assert!(!p.is_null());

        unsafe {
            (*p).queue_prev = core::ptr::null_mut();
            (*p).queue_next = core::ptr::null_mut();

            let mut cursor = self.head;
            while let Some(current) = cursor {
                if (*p).expires_ns < (*current.0).expires_ns {
                    let prev = (*current.0).queue_prev;
                    (*p).queue_prev = prev;
                    (*p).queue_next = current.0;
                    (*current.0).queue_prev = p;
                    if prev.is_null() {
                        self.head = Some(timer);
                    } else {
                        (*prev).queue_next = p;
                    }
                    return;
                }
                cursor = TimerPtr::from_raw((*current.0).queue_next);
            }

            if let Some(tail) = self.tail {
                (*tail.0).queue_next = p;
                (*p).queue_prev = tail.0;
            } else {
                self.head = Some(timer);
            }
            self.tail = Some(timer);
        }
    }

    /// Unlink by embedded node address, as `timerqueue_del()` does.  The
    /// caller holds the base lock and only invokes this for an enqueued timer.
    fn remove(&mut self, timer: TimerPtr) -> bool {
        let p = timer.0;
        if p.is_null() {
            return false;
        }

        unsafe {
            let prev = (*p).queue_prev;
            let next = (*p).queue_next;
            let is_head = self.head == Some(timer);
            if !is_head && prev.is_null() {
                return false;
            }

            if prev.is_null() {
                self.head = TimerPtr::from_raw(next);
            } else {
                (*prev).queue_next = next;
            }
            if next.is_null() {
                self.tail = TimerPtr::from_raw(prev);
            } else {
                (*next).queue_prev = prev;
            }
            (*p).queue_prev = core::ptr::null_mut();
            (*p).queue_next = core::ptr::null_mut();
        }
        true
    }

    fn pop_first(&mut self) -> Option<TimerPtr> {
        let timer = self.head?;
        let removed = self.remove(timer);
        debug_assert!(removed);
        removed.then_some(timer)
    }

    #[cfg(test)]
    fn clear(&mut self) {
        // Tests call this between cases, when no live timer may remain armed.
        // Do not follow potentially stale pointers left by a failed test.
        self.head = None;
        self.tail = None;
    }
}

impl TimerPtr {
    fn from_raw(ptr: *mut Hrtimer) -> Option<Self> {
        (!ptr.is_null()).then_some(Self(ptr))
    }
}

/// Minimal equivalent of Linux `struct hrtimer_clock_base` state needed to
/// synchronize cancellation against a callback which has already left the
/// timerqueue.  An inactive timer can still be executing: Linux records that
/// fact in `cpu_base->running` and makes `hrtimer_cancel()` wait for it.
struct TimerBase {
    queue: TimerQueue,
    running: Option<TimerPtr>,
}

impl TimerBase {
    const fn new() -> Self {
        Self {
            queue: TimerQueue::new(),
            running: None,
        }
    }
}

static QUEUES: [Mutex<TimerBase>; NUM_CLOCK_BASES] =
    [const { Mutex::new(TimerBase::new()) }; NUM_CLOCK_BASES];

static FIRED_COUNT: AtomicU64 = AtomicU64::new(0);

fn with_timer_queue_irqsave<R>(base_idx: usize, f: impl FnOnce(&mut TimerBase) -> R) -> R {
    let flags = crate::kernel::locking::local_irq_save();
    let result = {
        let mut base = QUEUES[base_idx].lock();
        f(&mut base)
    };
    crate::kernel::locking::local_irq_restore(flags);
    result
}

/// `hrtimer_start(timer, expires_ns, mode)`.
pub fn hrtimer_start(t: *mut Hrtimer, expires_ns: u64, mode: HrtimerMode) {
    if t.is_null() {
        return;
    }
    let flags = crate::kernel::locking::local_irq_save();
    let (base_idx, abs) = unsafe {
        let base_idx = (*t).base as usize;
        let abs = match mode {
            HrtimerMode::Rel => (*t).base_now().saturating_add(expires_ns),
            _ => expires_ns,
        };
        (base_idx, abs)
    };
    {
        let mut base = QUEUES[base_idx].lock();
        if unsafe { (*t).state == HRTIMER_STATE_ENQUEUED } {
            base.queue.remove(TimerPtr(t));
        }
        unsafe {
            (*t).expires_ns = abs;
            (*t).state = HRTIMER_STATE_ENQUEUED;
        }
        base.queue.insert(TimerPtr(t));
    }
    crate::kernel::locking::local_irq_restore(flags);
}

/// `hrtimer_try_to_cancel()` — return 1 for an enqueued timer, 0 for an
/// inactive timer, and -1 while its callback is executing.
///
/// The running test and queue removal share the same base lock, matching
/// `vendor/linux/kernel/time/hrtimer.c::hrtimer_try_to_cancel()`.
pub fn hrtimer_try_to_cancel(t: *mut Hrtimer) -> i32 {
    if t.is_null() {
        return 0;
    }
    let flags = crate::kernel::locking::local_irq_save();
    let ret = unsafe {
        let base_idx = (*t).base as usize;
        let mut base = QUEUES[base_idx].lock();
        if matches!(base.running, Some(running) if running.0 == t) {
            -1
        } else if (*t).state == HRTIMER_STATE_ENQUEUED {
            let removed = base.queue.remove(TimerPtr(t));
            (*t).state = HRTIMER_STATE_INACTIVE;
            i32::from(removed)
        } else {
            0
        }
    };
    crate::kernel::locking::local_irq_restore(flags);
    ret
}

/// Base-lock synchronized equivalent of Linux `hrtimer_active()`.
pub fn hrtimer_is_queued(t: *const Hrtimer) -> bool {
    if t.is_null() {
        return false;
    }
    let base_idx = unsafe { (*t).base as usize };
    with_timer_queue_irqsave(base_idx, |_| unsafe {
        (*t).state == HRTIMER_STATE_ENQUEUED
    })
}

/// Read the timer state under its clock-base lock. This is primarily useful
/// for owner-side snapshots and avoids racing expiry's ENQUEUED -> INACTIVE
/// transition.
pub fn hrtimer_state_snapshot(t: *const Hrtimer) -> u8 {
    if t.is_null() {
        return HRTIMER_STATE_INACTIVE;
    }
    let base_idx = unsafe { (*t).base as usize };
    with_timer_queue_irqsave(base_idx, |_| unsafe { (*t).state })
}

/// Linux `hrtimer_get_remaining()` semantics for this nanosecond timer model.
/// State and expiry are sampled together under the hrtimer base lock.
pub fn hrtimer_get_remaining(t: *const Hrtimer) -> u64 {
    if t.is_null() {
        return 0;
    }
    let base_idx = unsafe { (*t).base as usize };
    with_timer_queue_irqsave(base_idx, |_| unsafe {
        if (*t).state != HRTIMER_STATE_ENQUEUED || (*t).expires_ns == 0 {
            0
        } else {
            (*t).expires_ns.saturating_sub((*t).base_now())
        }
    })
}

/// Wait until a callback which won the dequeue race has returned.  Mainline
/// spins for hard-IRQ hrtimers and uses an expiry lock for PREEMPT_RT soft
/// timers.  Lupos currently expires this timer class from hard tick context,
/// so the hard-IRQ `cpu_relax()` behaviour is the matching implementation.
pub fn hrtimer_cancel_wait_running(t: *mut Hrtimer) {
    if t.is_null() {
        return;
    }
    let base_idx = unsafe { (*t).base as usize };
    loop {
        let running = with_timer_queue_irqsave(
            base_idx,
            |base| matches!(base.running, Some(running) if running.0 == t),
        );
        if !running {
            return;
        }
        core::hint::spin_loop();
    }
}

/// `hrtimer_cancel(timer)` — cancel an enqueued timer and synchronously wait
/// for an already-running callback.  Returns true if the timer was queued.
pub fn hrtimer_cancel(t: *mut Hrtimer) -> bool {
    loop {
        let ret = hrtimer_try_to_cancel(t);
        if ret >= 0 {
            return ret != 0;
        }
        hrtimer_cancel_wait_running(t);
    }
}

/// `hrtimer_forward(timer, now, interval)` — move an inactive timer's expiry
/// past `now` and return the number of elapsed intervals.
pub fn hrtimer_forward(t: &mut Hrtimer, now: u64, interval_ns: u64) -> u64 {
    if interval_ns == 0 || now < t.expires_ns || t.state == HRTIMER_STATE_ENQUEUED {
        return 0;
    }

    let delta = now.saturating_sub(t.expires_ns);
    let mut overruns = 1;
    if delta >= interval_ns {
        overruns = delta / interval_ns;
        t.expires_ns = t
            .expires_ns
            .saturating_add(interval_ns.saturating_mul(overruns));
        if t.expires_ns > now {
            return overruns;
        }
        overruns = overruns.saturating_add(1);
    }
    t.expires_ns = t.expires_ns.saturating_add(interval_ns);
    overruns
}

/// `hrtimer_forward_now(timer, interval)` — Linux helper used by timerfd read.
pub fn hrtimer_forward_now(t: &mut Hrtimer, interval_ns: u64) -> u64 {
    hrtimer_forward(t, t.base_now(), interval_ns)
}

/// `hrtimer_restart(timer)` — requeue at the timer's current absolute expiry.
pub fn hrtimer_restart(t: *mut Hrtimer) {
    if t.is_null() {
        return;
    }
    let _ = hrtimer_cancel(t);
    let expires = unsafe { (*t).expires_ns };
    hrtimer_start(t, expires, HrtimerMode::Abs);
}

/// `hrtimer_run_queues()` — fire any timers whose absolute expiry has passed.
/// Called from `tick_handle_periodic`.
pub fn hrtimer_run_queues() {
    for idx in 0..QUEUES.len() {
        let now = match unsafe { core::mem::transmute::<u8, ClockBase>(idx as u8) } {
            ClockBase::Monotonic | ClockBase::MonotonicRaw => ktime_get(),
            ClockBase::Realtime | ClockBase::Tai => ktime_get_real(),
            ClockBase::Boottime => ktime_get_boottime(),
        };
        // Pop and publish one expired timer atomically under the base lock.
        loop {
            let timer_to_run = with_timer_queue_irqsave(idx, |base| {
                if base.running.is_some() {
                    return None;
                }
                let timer = base.queue.first()?;
                let expiry = unsafe { (*timer.0).expires_ns };
                if expiry > now {
                    return None;
                }
                let timer = base.queue.pop_first()?;
                unsafe {
                    (*timer.0).state = HRTIMER_STATE_INACTIVE;
                }
                base.running = Some(timer);
                Some((timer, expiry))
            });
            match timer_to_run {
                Some((TimerPtr(p), expiry)) => {
                    FIRED_COUNT.fetch_add(1, Ordering::AcqRel);
                    let restart = unsafe {
                        if let Some(f) = (*p).function {
                            f(p)
                        } else {
                            HrtimerRestart::NoRestart
                        }
                    };
                    // Keep `running` published until every post-callback
                    // access to the raw timer pointer is complete.  A
                    // synchronous cancel/free racing this path therefore
                    // cannot return while `p` is still in use.
                    with_timer_queue_irqsave(idx, |base| {
                        if restart == HrtimerRestart::Restart
                            && unsafe { (*p).state != HRTIMER_STATE_ENQUEUED }
                        {
                            let interval = unsafe { (*p).interval_ns };
                            if interval > 0 {
                                let next = expiry.saturating_add(interval);
                                unsafe {
                                    (*p).expires_ns = next;
                                    (*p).state = HRTIMER_STATE_ENQUEUED;
                                }
                                base.queue.insert(TimerPtr(p));
                            }
                        }
                        if matches!(base.running, Some(running) if running.0 == p) {
                            base.running = None;
                        }
                    });
                }
                None => break,
            }
        }
    }
}

pub fn fired_count() -> u64 {
    FIRED_COUNT.load(Ordering::Acquire)
}

#[cfg(test)]
fn clear_queues_for_tests() {
    for q in QUEUES.iter() {
        let mut base = q.lock();
        base.queue.clear();
        base.running = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering as O};

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn hrtimer_constants_match_linux() {
        assert_eq!(NUM_CLOCK_BASES, 5);
        assert_eq!(HRTIMER_STATE_INACTIVE, 0);
        assert_eq!(HRTIMER_STATE_ENQUEUED, 1);
    }

    #[test]
    fn hrtimer_queue_lock_is_held_with_interrupts_disabled() {
        let source = include_str!("hrtimer.rs");
        let helper = source
            .split("fn with_timer_queue_irqsave")
            .nth(1)
            .expect("with_timer_queue_irqsave body")
            .split("/// `hrtimer_start")
            .next()
            .expect("with_timer_queue_irqsave body end");
        assert!(
            helper.contains("local_irq_save") && helper.contains("local_irq_restore"),
            "hrtimer queue helper must save/restore local IRQ state"
        );

        for (name, end_marker) in [
            ("pub fn hrtimer_start", "/// `hrtimer_try_to_cancel"),
            ("pub fn hrtimer_try_to_cancel", "/// Wait until a callback"),
        ] {
            let body = source
                .split(name)
                .nth(1)
                .unwrap_or_else(|| panic!("{name} body"))
                .split(end_marker)
                .next()
                .unwrap_or_else(|| panic!("{name} body end"));
            assert!(
                body.contains("local_irq_save") && body.contains("local_irq_restore"),
                "{name} must protect timer state and queue mutation from tick IRQ reentry"
            );
        }

        let run_queues = source
            .split("pub fn hrtimer_run_queues")
            .nth(1)
            .expect("hrtimer_run_queues body")
            .split("pub fn fired_count")
            .next()
            .expect("hrtimer_run_queues body end");
        assert!(
            run_queues.matches("with_timer_queue_irqsave").count() >= 2,
            "hrtimer_run_queues must use the IRQ-safe queue helper for expiry mutations"
        );
    }

    #[test]
    fn hrtimer_init_sets_base() {
        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Realtime, HrtimerMode::Rel);
        assert_eq!(t.base, ClockBase::Realtime);
        assert_eq!(t.state, HRTIMER_STATE_INACTIVE);
    }

    #[test]
    fn hrtimer_start_enqueues_and_run_fires() {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        fn cb(_t: *mut Hrtimer) -> HrtimerRestart {
            COUNT.fetch_add(1, O::AcqRel);
            HrtimerRestart::NoRestart
        }
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        COUNT.store(0, O::Release);

        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        t.function = Some(cb);
        // Start with absolute expiry = 0 so it fires immediately.
        hrtimer_start(&mut t as *mut Hrtimer, 0, HrtimerMode::Abs);
        assert_eq!(t.state, HRTIMER_STATE_ENQUEUED);
        hrtimer_run_queues();
        assert_eq!(COUNT.load(O::Acquire), 1);
        assert_eq!(t.state, HRTIMER_STATE_INACTIVE);
    }

    #[test]
    fn hrtimer_cancel_removes_active_timer() {
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        // Far-future expiry.
        hrtimer_start(&mut t as *mut Hrtimer, u64::MAX / 2, HrtimerMode::Abs);
        assert!(hrtimer_cancel(&mut t as *mut Hrtimer));
        assert!(!hrtimer_cancel(&mut t as *mut Hrtimer));
    }

    #[test]
    fn hrtimer_forward_now_rearms_after_current_time() {
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        t.expires_ns = 0;

        let overruns = hrtimer_forward_now(&mut t, 1_000_000_000);

        assert!(overruns >= 1);
        assert!(t.expires_ns > t.base_now());
    }

    #[test]
    fn hrtimer_start_replaces_existing_queue_entry() {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        fn cb(_t: *mut Hrtimer) -> HrtimerRestart {
            COUNT.fetch_add(1, O::AcqRel);
            HrtimerRestart::NoRestart
        }
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        COUNT.store(0, O::Release);

        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        t.function = Some(cb);

        hrtimer_start(&mut t as *mut Hrtimer, 0, HrtimerMode::Abs);
        hrtimer_start(&mut t as *mut Hrtimer, u64::MAX / 2, HrtimerMode::Abs);
        hrtimer_run_queues();

        assert_eq!(COUNT.load(O::Acquire), 0);
        assert!(hrtimer_cancel(&mut t as *mut Hrtimer));
    }

    #[test]
    fn hrtimer_cancel_after_restart_removes_only_live_entry() {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        fn cb(_t: *mut Hrtimer) -> HrtimerRestart {
            COUNT.fetch_add(1, O::AcqRel);
            HrtimerRestart::NoRestart
        }
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        COUNT.store(0, O::Release);

        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        t.function = Some(cb);

        hrtimer_start(&mut t as *mut Hrtimer, 0, HrtimerMode::Abs);
        hrtimer_start(&mut t as *mut Hrtimer, u64::MAX / 2, HrtimerMode::Abs);
        assert!(hrtimer_cancel(&mut t as *mut Hrtimer));
        hrtimer_run_queues();

        assert_eq!(COUNT.load(O::Acquire), 0);
        assert_eq!(t.state, HRTIMER_STATE_INACTIVE);
    }

    #[test]
    fn hrtimer_callback_restart_skips_duplicate_when_callback_rearmed() {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        fn cb(t: *mut Hrtimer) -> HrtimerRestart {
            COUNT.fetch_add(1, O::AcqRel);
            hrtimer_start(t, u64::MAX / 2, HrtimerMode::Abs);
            HrtimerRestart::Restart
        }
        let _guard = TEST_LOCK.lock();
        clear_queues_for_tests();
        COUNT.store(0, O::Release);

        let mut t = Hrtimer::new();
        hrtimer_init(&mut t, ClockBase::Monotonic, HrtimerMode::Abs);
        t.function = Some(cb);
        t.interval_ns = 1;

        hrtimer_start(&mut t as *mut Hrtimer, 0, HrtimerMode::Abs);
        hrtimer_run_queues();
        assert_eq!(COUNT.load(O::Acquire), 1);
        assert!(hrtimer_cancel(&mut t as *mut Hrtimer));
        hrtimer_run_queues();

        assert_eq!(COUNT.load(O::Acquire), 1);
    }
}
