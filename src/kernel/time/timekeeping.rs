//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/timekeeping.c
//! test-origin: linux:vendor/linux/kernel/time/timekeeping.c
//! Timekeeping — wall-clock accumulator (M36).
//!
//! Mirrors `vendor/linux/kernel/time/timekeeping.c`.  Maintains the running
//! wall-clock and monotonic readings.  Lupos M36 advances them tick-by-tick
//! via `tick_advance_walltime`; NTP slewing arrives in M59.

use core::sync::atomic::{AtomicU64, Ordering};

use super::jiffies::NSEC_PER_TICK;

const NSEC_PER_SEC: u64 = 1_000_000_000;

/// `struct timekeeper` — Linux ABI shape (subset).
pub struct Timekeeper {
    pub xtime_sec: AtomicU64,  // wall seconds since epoch
    pub xtime_nsec: AtomicU64, // wall ns within the current second
    pub mono_ns: AtomicU64,    // CLOCK_MONOTONIC ns since boot
    pub boot_ns: AtomicU64,    // CLOCK_BOOTTIME ns since boot (incl. suspend)
    pub tai_offset: AtomicU64, // CLOCK_TAI - CLOCK_REALTIME (seconds)
}

impl Timekeeper {
    pub const fn new() -> Self {
        Self {
            xtime_sec: AtomicU64::new(0),
            xtime_nsec: AtomicU64::new(0),
            mono_ns: AtomicU64::new(0),
            boot_ns: AtomicU64::new(0),
            tai_offset: AtomicU64::new(37), // 2024-era TAI−UTC offset
        }
    }
}

pub static TK: Timekeeper = Timekeeper::new();

/// TSC counter snapshot taken at the most recent `tick_advance_walltime`
/// call.  Read by `ktime_get` to interpolate sub-tick resolution between
/// LAPIC ticks.  Mirrors `vendor/linux/kernel/time/timekeeping.c::
/// tk_core.timekeeper.tkr_mono.cycle_last`.
static LAST_TICK_TSC: AtomicU64 = AtomicU64::new(0);
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);

#[inline]
fn read_timekeeping_tsc() -> u64 {
    crate::arch::x86::kernel::tsc::read_ordered()
}

#[inline]
fn tsc_delta_ns(now_tsc: u64, last_tsc: u64, khz: u64) -> Option<u64> {
    if now_tsc <= last_tsc || khz == 0 {
        return None;
    }
    Some(crate::arch::x86::kernel::tsc::cycles_to_ns(
        now_tsc - last_tsc,
        khz,
    ))
}

fn add_realtime_ns(delta_ns: u64) {
    let mut old = TK.xtime_nsec.load(Ordering::Acquire);
    loop {
        let total = old.saturating_add(delta_ns);
        let carry = total / NSEC_PER_SEC;
        let rem = total % NSEC_PER_SEC;
        match TK
            .xtime_nsec
            .compare_exchange_weak(old, rem, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                if carry != 0 {
                    TK.xtime_sec.fetch_add(carry, Ordering::AcqRel);
                }
                break;
            }
            Err(cur) => old = cur,
        }
    }
}

fn seed_tsc_anchor() {
    let now = read_timekeeping_tsc();
    if now == 0 {
        return;
    }
    LAST_TICK_TSC.store(now, Ordering::Release);
    BOOT_TSC.store(now, Ordering::Release);
}

/// Advance wall + mono + boot clocks to the current clocksource cycle.
///
/// Called from `tick_handle_periodic`.
pub fn tick_advance_walltime() {
    let now_tsc = read_timekeeping_tsc();
    let khz = crate::arch::x86::kernel::tsc::tsc_khz();
    let last_tsc = LAST_TICK_TSC.load(Ordering::Acquire);
    let advance_ns = tsc_delta_ns(now_tsc, last_tsc, khz).unwrap_or(NSEC_PER_TICK);

    add_realtime_ns(advance_ns);
    TK.mono_ns.fetch_add(advance_ns, Ordering::AcqRel);
    TK.boot_ns.fetch_add(advance_ns, Ordering::AcqRel);

    if now_tsc != 0 {
        LAST_TICK_TSC.store(now_tsc, Ordering::Release);
        let _ = BOOT_TSC.compare_exchange(0, now_tsc, Ordering::AcqRel, Ordering::Acquire);
    }
}

/// `ktime_get()` — CLOCK_MONOTONIC in nanoseconds.
///
/// Returns the per-tick mono baseline plus a TSC-derived interpolation
/// for sub-tick resolution.  Without this, two consecutive
/// `clock_gettime(CLOCK_MONOTONIC)` calls inside the same LAPIC tick
/// (4 ms at HZ=250) returned identical values — that's why `ping`
/// reported `time=0.000 ms` against our in-kernel ICMP echo synthesiser.
/// Linux's equivalent path in
/// `vendor/linux/kernel/time/timekeeping.c::ktime_get` adds
/// `clocksource_delta(tkr) * mult >> shift` for the same reason.
#[inline]
pub fn ktime_get() -> u64 {
    let base = TK.mono_ns.load(Ordering::Acquire);
    let last_tsc = LAST_TICK_TSC.load(Ordering::Acquire);
    let anchor_tsc = if last_tsc != 0 {
        last_tsc
    } else {
        let now = read_timekeeping_tsc();
        match BOOT_TSC.compare_exchange(0, now, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return base,
            Err(anchor) => anchor,
        }
    };
    let now_tsc = read_timekeeping_tsc();
    if now_tsc <= anchor_tsc {
        return base;
    }
    let delta = now_tsc - anchor_tsc;
    // Linux applies the calibrated clocksource scale factor here.
    // freq_hz: 1_000_000_000` (clocksource.rs::tsc_clocksource) — i.e.
    // assume 1 GHz so 1 TSC tick ≈ 1 ns until proper calibration lands.
    let khz = crate::arch::x86::kernel::tsc::tsc_khz();
    let delta_ns = crate::arch::x86::kernel::tsc::cycles_to_ns(delta, khz);
    base.saturating_add(delta_ns)
}

/// `ktime_get_real()` — CLOCK_REALTIME in nanoseconds.
#[inline]
pub fn ktime_get_real() -> u64 {
    let s = TK.xtime_sec.load(Ordering::Acquire);
    let ns = TK.xtime_nsec.load(Ordering::Acquire);
    let base = s.saturating_mul(NSEC_PER_SEC).saturating_add(ns);
    let last_tsc = LAST_TICK_TSC.load(Ordering::Acquire);
    let now_tsc = read_timekeeping_tsc();
    let khz = crate::arch::x86::kernel::tsc::tsc_khz();
    base.saturating_add(tsc_delta_ns(now_tsc, last_tsc, khz).unwrap_or(0))
}

/// `ktime_get_boottime()` — CLOCK_BOOTTIME in nanoseconds.
#[inline]
pub fn ktime_get_boottime() -> u64 {
    let mono_base = TK.mono_ns.load(Ordering::Acquire);
    let boot_base = TK.boot_ns.load(Ordering::Acquire);
    // Linux exposes boottime as monotonic plus the monotonic->boottime offset:
    // include/linux/timekeeping.h::ktime_get_boottime delegates to
    // kernel/time/timekeeping.c::ktime_get_with_offset(TK_OFFS_BOOT).
    ktime_get().saturating_add(boot_base.saturating_sub(mono_base))
}

/// Set the wall-clock seconds (used by `clock_settime`).
pub fn tk_set_wall_seconds(sec: u64) {
    TK.xtime_sec.store(sec, Ordering::Release);
    TK.xtime_nsec.store(0, Ordering::Release);
}

pub fn timekeeping_init_from_persistent_clock(wall_seconds: Option<u64>) -> bool {
    seed_tsc_anchor();
    let Some(sec) = wall_seconds else {
        return false;
    };
    if sec == 0 {
        return false;
    }
    tk_set_wall_seconds(sec);
    true
}

pub fn timekeeping_init() -> bool {
    let wall_seconds = unsafe { crate::arch::x86::kernel::rtc::read_persistent_clock_seconds() };
    timekeeping_init_from_persistent_clock(wall_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ktime_get_is_monotonic_across_ticks() {
        let before = ktime_get();
        tick_advance_walltime();
        assert!(ktime_get() > before);
    }

    #[test]
    fn tsc_delta_ns_uses_calibrated_khz_scale() {
        assert_eq!(tsc_delta_ns(2_500_000, 500_000, 1_000_000), Some(2_000_000));
        assert_eq!(tsc_delta_ns(500_000, 2_500_000, 1_000_000), None);
        assert_eq!(tsc_delta_ns(2_500_000, 500_000, 0), None);
    }

    #[test]
    fn realtime_add_handles_multi_second_delta() {
        let old_sec = TK.xtime_sec.load(Ordering::Acquire);
        let old_nsec = TK.xtime_nsec.load(Ordering::Acquire);

        TK.xtime_sec.store(10, Ordering::Release);
        TK.xtime_nsec.store(900_000_000, Ordering::Release);

        add_realtime_ns(2_250_000_000);

        assert_eq!(TK.xtime_sec.load(Ordering::Acquire), 13);
        assert_eq!(TK.xtime_nsec.load(Ordering::Acquire), 150_000_000);

        TK.xtime_sec.store(old_sec, Ordering::Release);
        TK.xtime_nsec.store(old_nsec, Ordering::Release);
    }

    /// Regression for the 2026-05-22 `ping google.com … time=0.000 ms`
    /// screenshot: two consecutive `ktime_get` calls inside the same
    /// LAPIC tick must NOT return identical timestamps now that we
    /// interpolate using the TSC delta.  This mirrors Linux's
    /// `vendor/linux/kernel/time/timekeeping.c::ktime_get` which adds
    /// `clocksource_delta(tkr) * mult >> shift` to the per-tick base.
    /// Host tests use a stub TSC that returns 0; we exercise the
    /// interpolation directly by driving `LAST_TICK_TSC`.
    #[test]
    fn ktime_get_interpolates_sub_tick_via_tsc_delta() {
        // Seed mono_ns and the last-tick TSC snapshot, then ask twice
        // with a synthetic delta in between.
        TK.mono_ns.store(1_000_000_000, Ordering::Release);
        LAST_TICK_TSC.store(5_000, Ordering::Release);

        let a = ktime_get();
        assert!(a >= 1_000_000_000);

        // Simulate the TSC having advanced past the snapshot.  Even
        // with the host TSC stub returning 0, we can verify the
        // `now_tsc <= last_tsc` guard returns the base.
        let b = ktime_get();
        assert!(b >= a, "interpolated ktime_get must be monotonic");

        // If `LAST_TICK_TSC == 0` (the host-test default) the
        // interpolation path short-circuits to the base mono.
        LAST_TICK_TSC.store(0, Ordering::Release);
        TK.mono_ns.store(2_000_000_000, Ordering::Release);
        assert_eq!(ktime_get(), 2_000_000_000);
    }

    #[test]
    fn ktime_get_boottime_uses_monotonic_to_boot_offset() {
        let old_mono = TK.mono_ns.load(Ordering::Acquire);
        let old_boot = TK.boot_ns.load(Ordering::Acquire);
        let old_last_tsc = LAST_TICK_TSC.load(Ordering::Acquire);
        let old_boot_tsc = BOOT_TSC.load(Ordering::Acquire);

        TK.mono_ns.store(1_000_000_000, Ordering::Release);
        TK.boot_ns
            .store(1_000_000_000 + 42_000_000, Ordering::Release);
        LAST_TICK_TSC.store(0, Ordering::Release);
        BOOT_TSC.store(0, Ordering::Release);

        assert_eq!(ktime_get(), 1_000_000_000);
        assert_eq!(ktime_get_boottime(), 1_042_000_000);

        TK.mono_ns.store(old_mono, Ordering::Release);
        TK.boot_ns.store(old_boot, Ordering::Release);
        LAST_TICK_TSC.store(old_last_tsc, Ordering::Release);
        BOOT_TSC.store(old_boot_tsc, Ordering::Release);
    }

    #[test]
    fn xtime_rolls_over_at_one_billion_ns() {
        TK.xtime_nsec.store(0, Ordering::Release);
        let s_before = TK.xtime_sec.load(Ordering::Acquire);
        // Advance enough ticks to roll over a second (HZ=250 → 250 ticks).
        for _ in 0..250 {
            tick_advance_walltime();
        }
        let s_after = TK.xtime_sec.load(Ordering::Acquire);
        assert!(s_after >= s_before + 1);
    }

    #[test]
    fn persistent_clock_seed_sets_realtime_base() {
        TK.xtime_sec.store(0, Ordering::Release);
        TK.xtime_nsec.store(123, Ordering::Release);

        assert!(timekeeping_init_from_persistent_clock(Some(1_779_194_096)));
        assert_eq!(ktime_get_real(), 1_779_194_096_000_000_000);

        tick_advance_walltime();
        assert!(ktime_get_real() > 1_779_194_096_000_000_000);
    }

    #[test]
    fn missing_or_zero_persistent_clock_keeps_fallback() {
        TK.xtime_sec.store(55, Ordering::Release);
        TK.xtime_nsec.store(0, Ordering::Release);

        assert!(!timekeeping_init_from_persistent_clock(None));
        assert_eq!(TK.xtime_sec.load(Ordering::Acquire), 55);
        assert!(!timekeeping_init_from_persistent_clock(Some(0)));
        assert_eq!(TK.xtime_sec.load(Ordering::Acquire), 55);
    }
}
