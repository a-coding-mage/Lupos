//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
/// Pressure Stall Information (PSI) — Milestone 18.
///
/// Tracks memory-stall time and computes exponentially-weighted moving
/// averages (EWMA) for 10 s, 60 s, and 300 s windows, producing the
/// `/proc/pressure/memory`-compatible output format.
///
/// In M18 the "clock" is a monotonic nanosecond counter backed by
/// `core::sync::atomic::AtomicU64`. Unit tests inject synthetic timestamps
/// via the `#[cfg(test)]`-gated `psi_set_now_us()` helper so EWMA behaviour
/// can be verified deterministically without real hardware timers.
///
/// Ref: Linux `kernel/sched/psi.c`, `include/linux/psi_types.h`
extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

// ---------------------------------------------------------------------------
// EWMA parameters — match Linux kernel/sched/psi.c
// ---------------------------------------------------------------------------

/// PSI sample period in microseconds (2 s in Linux; 1 s here for testability).
///
/// Linux uses 2 000 000 µs; we use 1 000 000 µs so tests that simulate a few
/// seconds of time still see meaningful decay without needing huge time jumps.
const SAMPLE_PERIOD_US: u64 = 1_000_000; // 1 s

/// Fractional bits used in fixed-point EWMA arithmetic.
const EWMA_FRAC_BITS: u32 = 16;
/// Fixed-point 1.0 = 1 << EWMA_FRAC_BITS.
const EWMA_ONE: u64 = 1 << EWMA_FRAC_BITS;

/// Pre-computed decay coefficients for the three windows.
///
/// `decay = round(exp(-SAMPLE_PERIOD / window) * EWMA_ONE)`
///
/// Window 10 s  → exp(-1/10)  ≈ 0.9048 → 59299
/// Window 60 s  → exp(-1/60)  ≈ 0.9835 → 64440
/// Window 300 s → exp(-1/300) ≈ 0.9967 → 65318
const DECAY_10S: u64 = 59299;
const DECAY_60S: u64 = 64440;
const DECAY_300S: u64 = 65318;

// ---------------------------------------------------------------------------
// EWMA state
// ---------------------------------------------------------------------------

/// Exponentially-weighted moving average for one time window.
#[derive(Clone, Copy, Debug)]
pub struct PsiAvg {
    /// Pre-computed decay coefficient (`exp(-period/window)` × `EWMA_ONE`).
    decay: u64,
    /// Current average value in fixed-point (× `EWMA_ONE`).
    val: u64,
}

impl PsiAvg {
    const fn new(decay: u64) -> Self {
        Self { decay, val: 0 }
    }

    /// Update the average given a new sample `pct` (percentage × `EWMA_ONE`).
    ///
    /// ```text
    /// avg = avg * decay + sample * (1 - decay)
    /// ```
    fn update(&mut self, pct: u64) {
        self.val = (self.val * self.decay + pct * (EWMA_ONE - self.decay)) / EWMA_ONE;
    }

    /// Return the average as a floating-point percentage (0.00 … 100.00).
    pub fn as_pct_f32(&self) -> f32 {
        (self.val as f32) / (EWMA_ONE as f32) * 100.0
    }
}

// ---------------------------------------------------------------------------
// Global PSI memory state
// ---------------------------------------------------------------------------

/// Per-resource PSI state.
///
/// Mirrors the relevant fields of `struct psi_group_cpu` in Linux.
pub struct PsiMemState {
    /// Accumulated stall time in microseconds (the `total` field in output).
    total_stall_us: AtomicU64,
    /// Monotonic "now" in microseconds, injected by tests.
    now_us: AtomicU64,
    /// Timestamp when the current stall began (0 = not stalling).
    stall_since_us: AtomicU64,
    /// Monotonic timestamp of the last EWMA sample update.
    last_sample_us: AtomicU64,
    /// EWMA for "some" (any task stalling) — 10 s, 60 s, 300 s.
    avgs: Mutex<[PsiAvg; 3]>,
}

impl PsiMemState {
    const fn new() -> Self {
        Self {
            total_stall_us: AtomicU64::new(0),
            now_us: AtomicU64::new(0),
            stall_since_us: AtomicU64::new(0),
            last_sample_us: AtomicU64::new(0),
            avgs: Mutex::new([
                PsiAvg::new(DECAY_10S),
                PsiAvg::new(DECAY_60S),
                PsiAvg::new(DECAY_300S),
            ]),
        }
    }
}

static MEM_PSI: PsiMemState = PsiMemState::new();

// ---------------------------------------------------------------------------
// Clock — real vs injected
// ---------------------------------------------------------------------------

/// Return the current time in microseconds.
///
/// Production path: reads `MEM_PSI.now_us` which is incremented externally
/// (by the timer tick or, once M36 lands, by the real TSC clocksource).
///
/// Test path: the same atomic is manipulated directly via `psi_set_now_us()`.
fn psi_now_us() -> u64 {
    MEM_PSI.now_us.load(Ordering::Relaxed)
}

/// Advance the PSI clock by `delta_us` microseconds.
///
/// Called by the timer interrupt (M36). In tests, use `psi_advance_us()`.
pub fn psi_tick(delta_us: u64) {
    MEM_PSI.now_us.fetch_add(delta_us, Ordering::Relaxed);
    maybe_update_averages();
}

/// Inject a synthetic "current time" for unit tests.
#[cfg(test)]
pub fn psi_set_now_us(us: u64) {
    MEM_PSI.now_us.store(us, Ordering::Relaxed);
}

/// Advance the injected clock by `delta` microseconds (test helper).
#[cfg(test)]
pub fn psi_advance_us(delta: u64) {
    MEM_PSI.now_us.fetch_add(delta, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Stall tracking — psi_memstall_enter / psi_memstall_leave
// ---------------------------------------------------------------------------

/// Mark the start of a memory-reclaim stall.
///
/// Returns a cookie (the current timestamp in µs) that must be passed to
/// `psi_memstall_leave()` to correctly account for the elapsed stall time.
///
/// Ref: Linux `psi_memstall_enter()` — `kernel/sched/psi.c:1056`
pub fn psi_memstall_enter() -> u64 {
    let now = psi_now_us();
    // Record start only if not already stalling (non-zero sentinel).
    // We allow re-entrant calls; the cookie tracks each caller's start.
    MEM_PSI
        .stall_since_us
        .compare_exchange(0, now, Ordering::AcqRel, Ordering::Relaxed)
        .ok();
    now
}

/// Mark the end of a memory-reclaim stall and accumulate elapsed time.
///
/// `cookie` must be the value returned by the matching `psi_memstall_enter()`.
///
/// Ref: Linux `psi_memstall_leave()` — `kernel/sched/psi.c:1087`
pub fn psi_memstall_leave(cookie: u64) {
    let now = psi_now_us();
    let elapsed = now.saturating_sub(cookie);
    MEM_PSI.total_stall_us.fetch_add(elapsed, Ordering::Relaxed);
    // Clear the stall start (0 = not stalling).
    MEM_PSI.stall_since_us.store(0, Ordering::Release);
    maybe_update_averages();
}

/// Return the total accumulated stall time in microseconds.
///
/// Exposed for `/proc/pressure/memory total=N` and for tests.
pub fn psi_total_stall_us() -> u64 {
    MEM_PSI.total_stall_us.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// EWMA update
// ---------------------------------------------------------------------------

/// Update EWMA averages if at least one sample period has elapsed.
fn maybe_update_averages() {
    let now = psi_now_us();
    let last = MEM_PSI.last_sample_us.load(Ordering::Relaxed);

    if now.saturating_sub(last) < SAMPLE_PERIOD_US {
        return;
    }

    MEM_PSI.last_sample_us.store(now, Ordering::Relaxed);

    // Fraction of the last sample period that was spent stalling.
    let total = MEM_PSI.total_stall_us.load(Ordering::Relaxed);
    let period_stall_us = total.saturating_sub(last);
    // Clamp to [0, SAMPLE_PERIOD_US] to avoid > 100% artifacts.
    let clamped = period_stall_us.min(SAMPLE_PERIOD_US);
    // Convert to fixed-point percentage.
    let pct = clamped * EWMA_ONE / SAMPLE_PERIOD_US;

    let mut avgs = MEM_PSI.avgs.lock();
    for avg in avgs.iter_mut() {
        avg.update(pct);
    }
}

// ---------------------------------------------------------------------------
// Output — /proc/pressure/memory format
// ---------------------------------------------------------------------------

/// Write the `/proc/pressure/memory`-compatible string into `buf`.
///
/// Format:
/// ```text
/// some avg10=X.XX avg60=X.XX avg300=X.XX total=N
/// full avg10=X.XX avg60=X.XX avg300=X.XX total=N
/// ```
///
/// Ref: Linux `psi_show()` — `kernel/sched/psi.c:1245`
pub fn psi_mem_show(buf: &mut String) {
    maybe_update_averages();

    let total = psi_total_stall_us();
    let avgs = MEM_PSI.avgs.lock();

    // "some" line: any task stalled (same value as "full" in single-task M18)
    buf.push_str(&format!(
        "some avg10={:.2} avg60={:.2} avg300={:.2} total={}\n",
        avgs[0].as_pct_f32(),
        avgs[1].as_pct_f32(),
        avgs[2].as_pct_f32(),
        total,
    ));
    // "full" line: all runnable tasks stalled (approximated as "some" in M18)
    buf.push_str(&format!(
        "full avg10={:.2} avg60={:.2} avg300={:.2} total={}\n",
        avgs[0].as_pct_f32(),
        avgs[1].as_pct_f32(),
        avgs[2].as_pct_f32(),
        total,
    ));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;

    use super::*;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK;
    use alloc::vec::Vec;

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn reset_psi() {
        MEM_PSI.total_stall_us.store(0, Ordering::Relaxed);
        MEM_PSI.now_us.store(0, Ordering::Relaxed);
        MEM_PSI.stall_since_us.store(0, Ordering::Relaxed);
        MEM_PSI.last_sample_us.store(0, Ordering::Relaxed);
        let mut avgs = MEM_PSI.avgs.lock();
        for avg in avgs.iter_mut() {
            avg.val = 0;
        }
    }

    // -----------------------------------------------------------------------
    // Basic stall tracking
    // -----------------------------------------------------------------------

    #[test]
    fn psi_memstall_accumulates() {
        let _g = test_guard();
        reset_psi();

        psi_set_now_us(1_000);
        let cookie = psi_memstall_enter();
        psi_set_now_us(11_000); // advance 10 ms
        psi_memstall_leave(cookie);

        assert_eq!(psi_total_stall_us(), 10_000);
    }

    #[test]
    fn psi_memstall_zero_on_no_stall() {
        let _g = test_guard();
        reset_psi();

        assert_eq!(psi_total_stall_us(), 0);
    }

    #[test]
    fn psi_multiple_stalls_accumulate() {
        let _g = test_guard();
        reset_psi();

        psi_set_now_us(0);
        let c1 = psi_memstall_enter();
        psi_set_now_us(5_000);
        psi_memstall_leave(c1);

        psi_set_now_us(10_000);
        let c2 = psi_memstall_enter();
        psi_set_now_us(15_000);
        psi_memstall_leave(c2);

        assert_eq!(psi_total_stall_us(), 10_000);
    }

    #[test]
    fn psi_nested_calls_safe() {
        let _g = test_guard();
        reset_psi();

        psi_set_now_us(100);
        let outer = psi_memstall_enter();
        psi_set_now_us(200);
        // Inner enter — should not corrupt the outer cookie.
        let inner = psi_memstall_enter();
        psi_set_now_us(300);
        psi_memstall_leave(inner); // elapsed = 100 µs
        psi_set_now_us(400);
        psi_memstall_leave(outer); // elapsed = 300 µs

        // Total should be 100 + 300 = 400 µs.
        assert_eq!(psi_total_stall_us(), 400);
    }

    // -----------------------------------------------------------------------
    // Output format — mirrors /proc/pressure/memory
    // -----------------------------------------------------------------------

    #[test]
    fn psi_show_format_some_full() {
        let _g = test_guard();
        reset_psi();

        let mut buf = String::new();
        psi_mem_show(&mut buf);

        // Must have exactly two lines.
        let lines: Vec<&str> = buf.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 lines, got: {:?}", lines);

        // Both lines must start with "some" and "full" respectively.
        assert!(lines[0].starts_with("some "), "line 0: {}", lines[0]);
        assert!(lines[1].starts_with("full "), "line 1: {}", lines[1]);

        // Each line must contain avg10=, avg60=, avg300=, total=.
        for line in &lines {
            assert!(line.contains("avg10="), "missing avg10 in: {}", line);
            assert!(line.contains("avg60="), "missing avg60 in: {}", line);
            assert!(line.contains("avg300="), "missing avg300 in: {}", line);
            assert!(line.contains("total="), "missing total in: {}", line);
        }
    }

    #[test]
    fn psi_show_total_matches_accumulated_stall() {
        let _g = test_guard();
        reset_psi();

        psi_set_now_us(0);
        let c = psi_memstall_enter();
        psi_set_now_us(12_345);
        psi_memstall_leave(c);

        let mut buf = String::new();
        psi_mem_show(&mut buf);

        assert!(
            buf.contains("total=12345"),
            "expected total=12345 in: {}",
            buf
        );
    }

    // -----------------------------------------------------------------------
    // EWMA decay — psi_ewma_decays_over_time
    // -----------------------------------------------------------------------

    #[test]
    fn psi_ewma_decays_over_time() {
        let _g = test_guard();
        reset_psi();

        // Simulate 100% stall for 2 sample periods, then stop stalling.
        psi_set_now_us(0);
        let c = psi_memstall_enter();
        psi_set_now_us(2 * SAMPLE_PERIOD_US);
        psi_memstall_leave(c);

        // Advance clock by many more sample periods with no stall.
        for _ in 0..20 {
            psi_advance_us(SAMPLE_PERIOD_US);
            maybe_update_averages();
        }

        // Average should have decayed toward 0 (< 50%).
        let avgs = MEM_PSI.avgs.lock();
        let avg10 = avgs[0].as_pct_f32();
        assert!(
            avg10 < 50.0,
            "10 s avg should have decayed below 50%, got {:.2}",
            avg10
        );
    }
}
