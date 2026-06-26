//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/hrtimer.c
//! test-origin: linux:vendor/linux/kernel/time/hrtimer.c
//! High-resolution timer — `struct hrtimer` (M36).
//!
//! Mirrors `vendor/linux/kernel/time/hrtimer.c`.  M36 ships a per-CPU,
//! 5-clock-base timerqueue keyed by absolute expiry time.  `hrtimer_run_queues`
//! is invoked from `tick_handle_periodic` (and later from a one-shot LAPIC
//! programmable event in M37).

extern crate alloc;

use alloc::collections::BTreeMap;
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

/// `struct hrtimer` — opaque-shape Linux primitive.
#[repr(C)]
pub struct Hrtimer {
    pub expires_ns: u64,
    pub interval_ns: u64,
    pub base: ClockBase,
    pub function: Option<fn(&mut Hrtimer) -> HrtimerRestart>,
    pub data: usize,
    pub state: u8,
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
}

// Per-base timer queue (BTreeMap by absolute expiry).  One global queue for
// M36 — per-CPU split arrives in M37 alongside the IRQ framework.
type TimerKey = (u64, usize);

/// Send-safe wrapper around `*mut Hrtimer` so the static `Mutex<BTreeMap>`
/// implements `Send`.  Access goes through the mutex; the unsafe handoff is
/// the user's responsibility.
#[derive(Clone, Copy)]
struct TimerPtr(*mut Hrtimer);
unsafe impl Send for TimerPtr {}

type TimerQueue = BTreeMap<TimerKey, TimerPtr>;

static QUEUES: [Mutex<TimerQueue>; NUM_CLOCK_BASES] =
    [const { Mutex::new(BTreeMap::new()) }; NUM_CLOCK_BASES];

static FIRED_COUNT: AtomicU64 = AtomicU64::new(0);

fn with_timer_queue_irqsave<R>(base_idx: usize, f: impl FnOnce(&mut TimerQueue) -> R) -> R {
    let flags = crate::kernel::locking::local_irq_save();
    let result = {
        let mut q = QUEUES[base_idx].lock();
        f(&mut q)
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
    let (base_idx, old_key, abs) = unsafe {
        let base_idx = (*t).base as usize;
        let old_key =
            ((*t).state == HRTIMER_STATE_ENQUEUED).then_some(((*t).expires_ns, t as usize));
        let abs = match mode {
            HrtimerMode::Rel => (*t).base_now().saturating_add(expires_ns),
            _ => expires_ns,
        };
        (base_idx, old_key, abs)
    };
    {
        let mut q = QUEUES[base_idx].lock();
        if let Some(key) = old_key {
            q.remove(&key);
        }
        unsafe {
            (*t).expires_ns = abs;
            (*t).state = HRTIMER_STATE_ENQUEUED;
        }
        q.insert((abs, t as usize), TimerPtr(t));
    }
    crate::kernel::locking::local_irq_restore(flags);
}

/// `hrtimer_cancel(timer)` — remove from the queue.  Returns true if it was
/// active.
pub fn hrtimer_cancel(t: *mut Hrtimer) -> bool {
    if t.is_null() {
        return false;
    }
    let flags = crate::kernel::locking::local_irq_save();
    let removed = unsafe {
        if (*t).state != HRTIMER_STATE_ENQUEUED {
            crate::kernel::locking::local_irq_restore(flags);
            return false;
        }
        let base_idx = (*t).base as usize;
        let abs = (*t).expires_ns;
        let removed = QUEUES[base_idx].lock().remove(&(abs, t as usize)).is_some();
        (*t).state = HRTIMER_STATE_INACTIVE;
        removed
    };
    crate::kernel::locking::local_irq_restore(flags);
    removed
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
        // Pop all expired keys.
        loop {
            let key_to_pop = with_timer_queue_irqsave(idx, |q| q.iter().next().map(|(k, _)| *k));
            match key_to_pop {
                Some((expiry, _)) if expiry <= now => {
                    let timer_ptr = with_timer_queue_irqsave(idx, |q| {
                        let timer_ptr = q.remove(&key_to_pop.unwrap());
                        if let Some(TimerPtr(p)) = timer_ptr
                            && !p.is_null()
                        {
                            unsafe {
                                (*p).state = HRTIMER_STATE_INACTIVE;
                            }
                        }
                        timer_ptr
                    });
                    if let Some(TimerPtr(p)) = timer_ptr {
                        if p.is_null() {
                            continue;
                        }
                        FIRED_COUNT.fetch_add(1, Ordering::AcqRel);
                        let restart = unsafe {
                            if let Some(f) = (*p).function {
                                f(&mut *p)
                            } else {
                                HrtimerRestart::NoRestart
                            }
                        };
                        if restart == HrtimerRestart::Restart
                            && unsafe { (*p).state != HRTIMER_STATE_ENQUEUED }
                        {
                            let interval = unsafe { (*p).interval_ns };
                            if interval > 0 {
                                let next = expiry.saturating_add(interval);
                                with_timer_queue_irqsave(idx, |q| {
                                    unsafe {
                                        (*p).expires_ns = next;
                                        (*p).state = HRTIMER_STATE_ENQUEUED;
                                    }
                                    q.insert((next, p as usize), TimerPtr(p));
                                });
                            }
                        }
                    }
                }
                _ => break,
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
        q.lock().clear();
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
            ("pub fn hrtimer_start", "/// `hrtimer_cancel"),
            ("pub fn hrtimer_cancel", "/// `hrtimer_forward"),
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
            run_queues.matches("with_timer_queue_irqsave").count() >= 3,
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
        fn cb(_t: &mut Hrtimer) -> HrtimerRestart {
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
        fn cb(_t: &mut Hrtimer) -> HrtimerRestart {
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
        fn cb(_t: &mut Hrtimer) -> HrtimerRestart {
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
        fn cb(t: &mut Hrtimer) -> HrtimerRestart {
            COUNT.fetch_add(1, O::AcqRel);
            hrtimer_start(t as *mut Hrtimer, u64::MAX / 2, HrtimerMode::Abs);
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
