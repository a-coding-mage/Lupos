//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! linux-source: vendor/linux/arch/x86/kernel/apic/apic.c
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! LAPIC timer driver.
//!
//! This file mirrors the Linux APIC timer programming shape in
//! `arch/x86/kernel/apic/apic.c`: LAPIC divisor 16, LVTT mode bits,
//! TSC-deadline helpers, shutdown masking plus counter zeroing, the
//! `lapic_timer_period = delta * APIC_DIVISOR / LAPIC_CAL_LOOPS` calibration
//! formula, and per-CPU tick accounting.
//!
//! The parity tag intentionally remains `partial`. Lupos still lacks Linux's
//! full clockevents/broadcast integration and PMTMR/PIT cross-check during
//! APIC calibration. Runtime calibration is bounded and TSC-derived when a
//! trusted TSC frequency is already available; otherwise we use a conservative
//! fallback period and keep reporting that as partial behavior.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::arch::x86::kernel::apic;
use crate::arch::x86::kernel::cpuid;
use crate::arch::x86::kernel::i8253;
use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::arch::x86::kernel::idt::TIMER_VECTOR;
use crate::arch::x86::kernel::msr;
use crate::arch::x86::kernel::tsc;

/// Linux `APIC_DIVISOR`: the LAPIC bus-clock divisor used for the local timer.
pub const APIC_DIVISOR: u32 = 16;

/// Linux `TSC_DIVISOR`: deadline deltas are scaled by this before WRMSR.
pub const TSC_DIVISOR: u64 = 8;

/// Divide-by-16 encoding for the timer divide configuration register.
///
/// Intel SDM Vol. 3A Table 10-10: divide by 16 is encoded as 0011b.
pub const DIVIDE_BY_16: u32 = 0b0011;

/// Linux `LAPIC_CAL_LOOPS`: calibrate over 100 ms when `HZ == 250`.
pub const LAPIC_CAL_LOOPS: u32 = i8253::HZ / 10;

/// Linux rejects LAPIC calibration results slower than 1 MHz / HZ.
pub const MIN_LAPIC_TIMER_PERIOD: u32 = 1_000_000 / i8253::HZ;

/// Integrated APIC LVTT TSC-deadline mode bit.
pub const LVT_TIMER_TSC_DEADLINE: u32 = 1 << 18;

/// IA32_TSC_DEADLINE MSR index.
pub const MSR_IA32_TSC_DEADLINE: u32 = 0x0000_06E0;

/// CPUID.1:ECX bit for TSC-deadline timer support.
pub const CPUID_TSC_DEADLINE_TIMER: u32 = 1 << 24;

/// Fallback initial count used only when TSC-backed calibration is unavailable.
///
/// This keeps the boot smoke test alive on QEMU while preserving the honest
/// `partial` tag until a PIT/PMTMR-verified calibration path exists.
pub const FALLBACK_INITIAL_COUNT: u32 = 125_000;

/// Fallback period in Linux's "APIC bus clocks" units.
pub const FALLBACK_LAPIC_TIMER_PERIOD: u32 = FALLBACK_INITIAL_COUNT * APIC_DIVISOR;

/// Backward-compatible name used by older tests/docs.
pub const NOMINAL_INITIAL_COUNT: u32 = FALLBACK_INITIAL_COUNT;

static LAPIC_TIMER_PERIOD: AtomicU32 = AtomicU32::new(0);

/// Monotonic system tick counter, advanced only by the designated timekeeper
/// CPU.  Per-CPU LAPIC delivery counts live in `PER_CPU_TIMER_TICKS`.
pub static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

static PER_CPU_TIMER_TICKS: [AtomicU64; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicU64::new(0) }; crate::kernel::sched::MAX_CPUS];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TimerMode {
    Periodic,
    Oneshot,
    TscDeadline,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TimerShutdownAction {
    ZeroInitialCount,
    ZeroTscDeadline,
}

const fn periodic_initial_count(period: u32) -> u32 {
    let count = period / APIC_DIVISOR;
    if count == 0 { 1 } else { count }
}

const fn lvt_timer_config_value(vector: u8, mode: TimerMode, irq_enabled: bool) -> u32 {
    let mut value = vector as u32;
    value |= match mode {
        TimerMode::Periodic => apic::LVT_TIMER_PERIODIC,
        TimerMode::Oneshot => 0,
        TimerMode::TscDeadline => LVT_TIMER_TSC_DEADLINE,
    };
    if !irq_enabled {
        value |= apic::LVT_MASKED;
    }
    value
}

/// Build the LVT Timer value: vector | optional periodic mode, unmasked.
///
/// Kept stable for existing host tests and callers.
pub const fn lvt_timer_value(vector: u8, periodic: bool) -> u32 {
    let mode = if periodic {
        TimerMode::Periodic
    } else {
        TimerMode::Oneshot
    };
    lvt_timer_config_value(vector, mode, true)
}

const fn shutdown_lvt_value(current_lvt: u32) -> u32 {
    current_lvt | apic::LVT_MASKED | (TIMER_VECTOR as u32)
}

const fn shutdown_action_for_lvt(current_lvt: u32) -> TimerShutdownAction {
    if current_lvt & LVT_TIMER_TSC_DEADLINE != 0 {
        TimerShutdownAction::ZeroTscDeadline
    } else {
        TimerShutdownAction::ZeroInitialCount
    }
}

const fn lapic_timer_period_from_delta(delta: u32) -> Option<u32> {
    let period = (delta as u64).saturating_mul(APIC_DIVISOR as u64) / (LAPIC_CAL_LOOPS as u64);
    if period < MIN_LAPIC_TIMER_PERIOD as u64 || period > u32::MAX as u64 {
        None
    } else {
        Some(period as u32)
    }
}

const fn tsc_deadline_value(now: u64, delta: u32) -> u64 {
    now.saturating_add((delta as u64).saturating_mul(TSC_DIVISOR))
}

#[inline]
pub fn tsc_deadline_supported() -> bool {
    cpuid::cpuid(1, 0).ecx & CPUID_TSC_DEADLINE_TIMER != 0
}

#[inline]
unsafe fn write_tsc_deadline(value: u64) {
    unsafe {
        msr::write(MSR_IA32_TSC_DEADLINE, value);
    }
}

#[inline]
unsafe fn set_tsc_deadline_delta(delta: u32) {
    let deadline = tsc_deadline_value(tsc::read(), delta);
    unsafe {
        write_tsc_deadline(deadline);
    }
}

#[inline]
fn serialize_tsc_deadline_lvtt() {
    #[cfg(not(test))]
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
}

unsafe fn setup_apic_lvtt(clocks: u32, mode: TimerMode, irq_enabled: bool) {
    let lvtt = lvt_timer_config_value(TIMER_VECTOR, mode, irq_enabled);
    unsafe {
        apic::timer_write_lvt(lvtt);
    }

    if mode == TimerMode::TscDeadline {
        serialize_tsc_deadline_lvtt();
        return;
    }

    unsafe {
        apic::timer_write_divide(DIVIDE_BY_16);
    }

    if mode == TimerMode::Periodic {
        unsafe {
            apic::timer_write_init_count(periodic_initial_count(clocks));
        }
    }
}

unsafe fn shutdown_lapic_timer(current_lvt: u32) {
    let masked_lvt = shutdown_lvt_value(current_lvt);
    unsafe {
        apic::timer_write_lvt(masked_lvt);
    }

    match shutdown_action_for_lvt(masked_lvt) {
        TimerShutdownAction::ZeroTscDeadline => unsafe {
            write_tsc_deadline(0);
        },
        TimerShutdownAction::ZeroInitialCount => unsafe {
            apic::timer_write_init_count(0);
        },
    }
}

fn calibrated_or_fallback_period() -> u32 {
    let period = LAPIC_TIMER_PERIOD.load(Ordering::Acquire);
    if period != 0 {
        period
    } else {
        FALLBACK_LAPIC_TIMER_PERIOD
    }
}

unsafe fn calibrate_apic_clock_or_fallback() -> u32 {
    let cached = LAPIC_TIMER_PERIOD.load(Ordering::Acquire);
    if cached != 0 {
        return cached;
    }

    let khz = tsc::tsc_khz();
    if khz == 0 {
        LAPIC_TIMER_PERIOD.store(FALLBACK_LAPIC_TIMER_PERIOD, Ordering::Release);
        return FALLBACK_LAPIC_TIMER_PERIOD;
    }

    let wait_cycles = tsc::ns_to_cycles(100_000_000, khz);
    if wait_cycles == 0 {
        LAPIC_TIMER_PERIOD.store(FALLBACK_LAPIC_TIMER_PERIOD, Ordering::Release);
        return FALLBACK_LAPIC_TIMER_PERIOD;
    }

    unsafe {
        setup_apic_lvtt(u32::MAX, TimerMode::Periodic, false);
    }

    let start_count = unsafe { apic::timer_read_current() };
    let start_tsc = tsc::read_ordered();
    while tsc::read_ordered().wrapping_sub(start_tsc) < wait_cycles {
        core::hint::spin_loop();
    }
    let end_count = unsafe { apic::timer_read_current() };

    let delta = start_count.saturating_sub(end_count);
    let period = lapic_timer_period_from_delta(delta).unwrap_or(FALLBACK_LAPIC_TIMER_PERIOD);
    LAPIC_TIMER_PERIOD.store(period, Ordering::Release);
    period
}

fn record_tick_for_cpu(cpu: usize) -> u64 {
    let slot = cpu.min(crate::kernel::sched::MAX_CPUS - 1);
    PER_CPU_TIMER_TICKS[slot].fetch_add(1, Ordering::Release) + 1
}

/// Return the LAPIC timer ticks observed on a CPU slot.
pub fn timer_ticks_for_cpu(cpu: usize) -> Option<u64> {
    PER_CPU_TIMER_TICKS
        .get(cpu)
        .map(|ticks| ticks.load(Ordering::Acquire))
}

/// Initialize the BSP's LAPIC timer in periodic mode.
///
/// # Safety
/// LAPIC MMIO must be accessible; the caller must have run `apic::init()`.
pub unsafe fn init() {
    let period = unsafe { calibrate_apic_clock_or_fallback() };
    unsafe {
        setup_apic_lvtt(period, TimerMode::Periodic, true);
    }
}

/// Initialize or shut down an AP-side LAPIC timer.
///
/// APs get a real periodic timer only once production SMP scheduling is
/// enabled. Earlier SMP smoke tests keep AP timers masked and zeroed.
///
/// # Safety
/// Same constraints as `init()`.
pub unsafe fn init_ap() {
    if crate::kernel::sched::production_smp_scheduler_enabled() {
        let period = calibrated_or_fallback_period();
        unsafe {
            setup_apic_lvtt(period, TimerMode::Periodic, true);
        }
    } else {
        let lvt = lvt_timer_config_value(TIMER_VECTOR, TimerMode::Periodic, false);
        unsafe {
            shutdown_lapic_timer(lvt);
        }
    }
}

/// Called from the IDT timer ISR (`idt::on_timer_interrupt`).
#[inline]
pub fn on_tick(frame: Option<&ExceptionFrame>) {
    let cpu = crate::arch::x86::kernel::setup_percpu::current_cpu_number();
    record_tick_for_cpu(cpu);
    if crate::kernel::time::clockevents::tick_do_timer_cpu(cpu) {
        TIMER_TICKS.fetch_add(1, Ordering::Release);
    }
    crate::kernel::time::clockevents::tick_handle_periodic_for_cpu(cpu);
    // Wake timed sleepers whose deadline has passed before the scheduler runs,
    // so a `schedule_timeout`/`msleep` returns promptly (event-driven) instead
    // of the task busy-yielding to its deadline. The timeout wheel is currently
    // global, so the same CPU that owns jiffies is its sole expiry runner.
    if crate::kernel::time::clockevents::tick_do_timer_cpu(cpu) {
        crate::kernel::time::sleep_timeout::sleep_timers_expire(
            crate::kernel::time::jiffies::jiffies(),
        );
    }
    crate::kernel::watchdog::watchdog_tick(cpu, frame);
    crate::kernel::sched::scheduler_tick();
}

#[cfg(feature = "test-timer")]
#[inline]
fn rdtsc() -> u64 {
    tsc::read()
}

#[cfg(feature = "test-timer")]
pub fn run_timer_test() {
    use core::sync::atomic::Ordering;

    const TARGET_TICKS: u64 = 100;
    const TIMEOUT_CYCLES: u64 = 5_000_000_000;

    let start_tsc = rdtsc();
    let deadline = start_tsc.saturating_add(TIMEOUT_CYCLES);

    crate::kernel::printk::log_info!("timer", "timer: waiting for {} ticks...", TARGET_TICKS);

    loop {
        let ticks = TIMER_TICKS.load(Ordering::Acquire);
        if ticks >= TARGET_TICKS {
            let drift = rdtsc().saturating_sub(start_tsc);
            crate::kernel::printk::log_info!("timer", "timer: ticks={} drift={}", ticks, drift);

            #[cfg(feature = "qemu-test")]
            unsafe {
                crate::linux_driver_abi::platform::qemu::exit_success();
            }
            return;
        }
        if rdtsc() >= deadline {
            panic!(
                "timer: TDD test FAILED - only {} ticks observed in 5s (expected {})",
                ticks, TARGET_TICKS
            );
        }
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX_APIC_C: &str = include_str!("../../../../vendor/linux/arch/x86/kernel/apic/apic.c");

    #[test]
    fn linux_source_contains_apic_timer_parity_anchors() {
        assert!(LINUX_APIC_C.contains("#define APIC_DIVISOR 16"));
        assert!(LINUX_APIC_C.contains("#define TSC_DIVISOR  8"));
        assert!(LINUX_APIC_C.contains("#define LAPIC_CAL_LOOPS"));
        assert!(LINUX_APIC_C.contains("APIC_LVT_TIMER_TSCDEADLINE"));
        assert!(LINUX_APIC_C.contains("lapic_timer_shutdown"));
        assert!(
            LINUX_APIC_C.contains("lapic_timer_period = (delta * APIC_DIVISOR) / LAPIC_CAL_LOOPS")
        );
    }

    #[test]
    fn linux_constants_match_local_timer_constants() {
        assert_eq!(APIC_DIVISOR, 16);
        assert_eq!(TSC_DIVISOR, 8);
        assert_eq!(DIVIDE_BY_16, 0b0011);
        assert_eq!(LAPIC_CAL_LOOPS, 25);
        assert_eq!(MIN_LAPIC_TIMER_PERIOD, 4_000);
        assert_eq!(LVT_TIMER_TSC_DEADLINE, 1 << 18);
        assert_eq!(MSR_IA32_TSC_DEADLINE, 0x0000_06E0);
        assert_eq!(CPUID_TSC_DEADLINE_TIMER, 1 << 24);
    }

    #[test]
    fn fallback_period_programs_expected_initial_count() {
        assert_eq!(
            FALLBACK_LAPIC_TIMER_PERIOD,
            FALLBACK_INITIAL_COUNT * APIC_DIVISOR
        );
        assert_eq!(
            periodic_initial_count(FALLBACK_LAPIC_TIMER_PERIOD),
            FALLBACK_INITIAL_COUNT
        );
        assert_eq!(NOMINAL_INITIAL_COUNT, FALLBACK_INITIAL_COUNT);
    }

    #[test]
    fn lvt_periodic_mode_uses_vector_and_unmasked_periodic_bit() {
        let lvt = lvt_timer_config_value(TIMER_VECTOR, TimerMode::Periodic, true);
        assert_eq!(lvt & 0xFF, TIMER_VECTOR as u32);
        assert_ne!(lvt & apic::LVT_TIMER_PERIODIC, 0);
        assert_eq!(lvt & LVT_TIMER_TSC_DEADLINE, 0);
        assert_eq!(lvt & apic::LVT_MASKED, 0);
    }

    #[test]
    fn lvt_oneshot_mode_clears_periodic_and_deadline_bits() {
        let lvt = lvt_timer_config_value(TIMER_VECTOR, TimerMode::Oneshot, true);
        assert_eq!(lvt & 0xFF, TIMER_VECTOR as u32);
        assert_eq!(lvt & apic::LVT_TIMER_PERIODIC, 0);
        assert_eq!(lvt & LVT_TIMER_TSC_DEADLINE, 0);
        assert_eq!(lvt & apic::LVT_MASKED, 0);
    }

    #[test]
    fn lvt_tsc_deadline_mode_sets_deadline_bit_only() {
        let lvt = lvt_timer_config_value(TIMER_VECTOR, TimerMode::TscDeadline, true);
        assert_eq!(lvt & 0xFF, TIMER_VECTOR as u32);
        assert_ne!(lvt & LVT_TIMER_TSC_DEADLINE, 0);
        assert_eq!(lvt & apic::LVT_TIMER_PERIODIC, 0);
        assert_eq!(lvt & apic::LVT_MASKED, 0);
    }

    #[test]
    fn lvt_masking_matches_linux_irq_disabled_setup() {
        let lvt = lvt_timer_config_value(TIMER_VECTOR, TimerMode::Periodic, false);
        assert_ne!(lvt & apic::LVT_MASKED, 0);
        assert_eq!(shutdown_lvt_value(0) & apic::LVT_MASKED, apic::LVT_MASKED);
        assert_eq!(shutdown_lvt_value(0) & 0xFF, TIMER_VECTOR as u32);
    }

    #[test]
    fn shutdown_action_zeroes_deadline_or_initial_count() {
        assert_eq!(
            shutdown_action_for_lvt(lvt_timer_config_value(
                TIMER_VECTOR,
                TimerMode::Periodic,
                true
            )),
            TimerShutdownAction::ZeroInitialCount
        );
        assert_eq!(
            shutdown_action_for_lvt(lvt_timer_config_value(
                TIMER_VECTOR,
                TimerMode::TscDeadline,
                true
            )),
            TimerShutdownAction::ZeroTscDeadline
        );
    }

    #[test]
    fn calibration_formula_matches_linux_delta_math() {
        let delta = 1_000_000;
        let expected = (delta * APIC_DIVISOR) / LAPIC_CAL_LOOPS;
        assert_eq!(lapic_timer_period_from_delta(delta), Some(expected));
        assert_eq!(expected, 640_000);
        assert_eq!(periodic_initial_count(expected), 40_000);
    }

    #[test]
    fn calibration_rejects_linux_too_slow_period() {
        assert_eq!(lapic_timer_period_from_delta(1), None);
        assert_eq!(
            lapic_timer_period_from_delta(6_250),
            Some(MIN_LAPIC_TIMER_PERIOD)
        );
    }

    #[test]
    fn tsc_deadline_delta_uses_linux_tsc_divisor() {
        assert_eq!(tsc_deadline_value(1_000, 10), 1_080);
    }

    #[test]
    fn per_cpu_tick_accounting_increments_selected_cpu() {
        let cpu = 0;
        PER_CPU_TIMER_TICKS[cpu].store(0, Ordering::Relaxed);
        assert_eq!(record_tick_for_cpu(cpu), 1);
        assert_eq!(timer_ticks_for_cpu(cpu), Some(1));
    }

    #[test]
    fn per_cpu_tick_accounting_clamps_out_of_range_cpu() {
        let last = crate::kernel::sched::MAX_CPUS - 1;
        PER_CPU_TIMER_TICKS[last].store(0, Ordering::Relaxed);
        assert_eq!(record_tick_for_cpu(usize::MAX), 1);
        assert_eq!(timer_ticks_for_cpu(last), Some(1));
        assert_eq!(timer_ticks_for_cpu(crate::kernel::sched::MAX_CPUS), None);
    }

    #[test]
    fn compatibility_lvt_wrapper_still_matches_periodic_boolean() {
        assert_eq!(
            lvt_timer_value(TIMER_VECTOR, true),
            lvt_timer_config_value(TIMER_VECTOR, TimerMode::Periodic, true)
        );
        assert_eq!(
            lvt_timer_value(TIMER_VECTOR, false),
            lvt_timer_config_value(TIMER_VECTOR, TimerMode::Oneshot, true)
        );
    }
}
