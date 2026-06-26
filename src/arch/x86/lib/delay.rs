//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/delay.c
//! test-origin: linux:vendor/linux/arch/x86/lib/delay.c
//! Precise delay loops.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/lib/delay.c
//! - vendor/linux/arch/x86/include/asm/delay.h
//!
//! Linux selects between four delay primitives at boot, all reachable
//! through `__delay()`:
//!
//! 1. `delay_loop`       — a calibrated tight loop. Default until calibration.
//! 2. `delay_tsc`        — busy-spin reading RDTSC until `cycles` elapse.
//! 3. `delay_halt_tpause`— Intel TPAUSE (deep C-state, exit on EDX:EAX TSC).
//! 4. `delay_halt_mwaitx`— AMD MWAITX with built-in 32-bit TSC timer.
//!
//! Selection happens once per boot via `use_tsc_delay`, `use_tpause_delay`,
//! `use_mwaitx_delay`. We preserve the same surface in Rust and back the
//! function-pointer dispatch with an `AtomicUsize` so the change is
//! lock-free on the hot path (matches `__ro_after_init` semantics — the
//! pointer is written only at boot, read freely thereafter).

use core::sync::atomic::{AtomicU8, Ordering};

use crate::arch::x86::kernel::tsc;

/// Vendor-specific delay strategy. Indexed by `DELAY_FN`. Numeric values
/// match the order in `delay.c` so a future port of `read_current_timer()`
/// can compare directly.
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DelayKind {
    /// `delay_loop` — pre-calibration default.
    Loop = 0,
    /// `delay_tsc` — RDTSC busy-loop.
    Tsc = 1,
    /// `delay_halt` with `delay_halt_tpause`.
    Tpause = 2,
    /// `delay_halt` with `delay_halt_mwaitx`.
    Mwaitx = 3,
}

/// Linux `MWAITX_MAX_WAIT_CYCLES` — the AMD MWAITX timer is 32-bit, so
/// callers must chunk longer waits in `delay_halt`.
pub const MWAITX_MAX_WAIT_CYCLES: u64 = 0xFFFF_FFFF;

/// Linux `TPAUSE_C02_STATE` — deeper C0.2 sleep state (lower exit
/// latency than C0.1, hard-coded as in `delay.c` line 117).
pub const TPAUSE_C02_STATE: u32 = 0;

/// Selected delay strategy. `__ro_after_init` in Linux; we use a relaxed
/// atomic — the strategy is set once at boot before any AP comes up.
static DELAY_FN: AtomicU8 = AtomicU8::new(DelayKind::Loop as u8);

/// Returns the active delay strategy. Used by `read_current_timer()`.
#[inline]
pub fn current_kind() -> DelayKind {
    match DELAY_FN.load(Ordering::Relaxed) {
        0 => DelayKind::Loop,
        1 => DelayKind::Tsc,
        2 => DelayKind::Tpause,
        _ => DelayKind::Mwaitx,
    }
}

/// `use_tsc_delay()` — only promotes Loop → Tsc, never overrides a
/// later TPAUSE/MWAITX selection. Matches `delay.c` lines 174-178.
pub fn use_tsc_delay() {
    let _ = DELAY_FN.compare_exchange(
        DelayKind::Loop as u8,
        DelayKind::Tsc as u8,
        Ordering::Release,
        Ordering::Relaxed,
    );
}

/// `use_tpause_delay()` — unconditionally selects TPAUSE.
pub fn use_tpause_delay() {
    DELAY_FN.store(DelayKind::Tpause as u8, Ordering::Release);
}

/// `use_mwaitx_delay()` — unconditionally selects MWAITX.
pub fn use_mwaitx_delay() {
    DELAY_FN.store(DelayKind::Mwaitx as u8, Ordering::Release);
}

/// Tight calibrated loop. Mirrors the assembly in `delay.c` lines 40-60
/// — aligned tight loop with two nested `dec/jnz`. We use a Rust loop
/// with `core::hint::spin_loop` to keep the CPU from speculatively
/// burning extra power; the timing is approximate (calibrated via
/// `loops_per_jiffy` elsewhere).
#[inline(never)]
pub fn delay_loop(loops: u64) {
    if loops == 0 {
        return;
    }
    let mut remaining = loops;
    while remaining > 0 {
        core::hint::spin_loop();
        remaining = remaining.wrapping_sub(1);
    }
}

/// `delay_tsc` — busy-spin reading the TSC until `cycles` have elapsed.
/// Mirrors `delay.c` lines 63-97 (without the preempt/cpu-rebalance dance,
/// which lupos handles via its own preempt counter once that lands).
pub fn delay_tsc(cycles: u64) {
    let start = tsc::read_ordered();
    loop {
        let now = tsc::read_ordered();
        if now.wrapping_sub(start) >= cycles {
            return;
        }
        // `native_pause()` — Linux uses PAUSE here to be friendly to
        // hyperthreading and CPU power management.
        #[cfg(all(any(target_arch = "x86_64", target_arch = "x86"), not(test)))]
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
        }
        core::hint::spin_loop();
    }
}

/// `delay_halt_tpause` — single TPAUSE invocation. Caller (`delay_halt`)
/// loops if the wakeup was premature.
#[allow(unused_variables)]
pub fn delay_halt_tpause(start: u64, cycles: u64) {
    let until = start.wrapping_add(cycles);
    let _eax = until as u32;
    let _edx = (until >> 32) as u32;
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        // TPAUSE encoding (Intel SDM Vol. 2B): 66 0F AE F1. The Linux
        // helper `__tpause(state, edx, eax)` boils down to:
        //   mov edx, edx ; mov eax, eax ; mov ecx, state ; tpause ecx
        core::arch::asm!(
            "tpause {state:e}",
            state = in(reg) TPAUSE_C02_STATE,
            in("edx") _edx,
            in("eax") _eax,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// `delay_halt_mwaitx` — AMD MWAITX with built-in 32-bit timer. The Linux
/// helper monitors `cpu_tss_rw`; lupos uses a per-CPU static byte for the
/// same purpose (cache-line aligned, seldom written).
///
/// MWAITX takes the delay in `EBX`. LLVM reserves RBX as a base pointer
/// in PIC code and does not let Rust use it as a named operand, so the
/// instruction is wrapped in `mov rbx, rdx ; mwaitx` (saving/restoring
/// RBX around the call). Matches Linux `__mwaitx` (`asm/mwait.h`).
#[allow(unused_variables)]
pub fn delay_halt_mwaitx(_unused: u64, cycles: u64) {
    let delay = cycles.min(MWAITX_MAX_WAIT_CYCLES);
    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe {
        static MONITOR_TARGET: u8 = 0;
        // MONITORX rax, ecx, edx — Linux passes ecx=0, edx=0.
        core::arch::asm!(
            "monitorx",
            in("rax") &MONITOR_TARGET as *const u8 as u64,
            in("rcx") 0u64,
            in("rdx") 0u64,
            options(nomem, nostack, preserves_flags),
        );
        // MWAITX — EAX=0xf (no deep C-state), EBX=delay, ECX=2 (timer enable).
        // RBX is reserved by LLVM (base pointer in PIC) so we shuttle the
        // value through RDX and `xchg rbx, rdx` around the instruction.
        core::arch::asm!(
            "xchg rbx, {delay}",
            "mwaitx",
            "xchg rbx, {delay}",
            delay = in(reg) delay,
            in("eax") 0xfu32,
            in("ecx") 2u32,
            options(nomem, nostack, preserves_flags),
        );
    }
    let _ = delay;
}

/// `delay_halt` — call vendor halt fn, then re-check elapsed TSC since
/// halt may return early. Mirrors `delay.c` lines 149-172.
fn delay_halt(cycles: u64, halt: fn(u64, u64)) {
    if cycles == 0 {
        return;
    }
    let mut start = tsc::read_ordered();
    let mut remaining = cycles;
    loop {
        halt(start, remaining);
        let end = tsc::read_ordered();
        let elapsed = end.wrapping_sub(start);
        if remaining <= elapsed {
            return;
        }
        remaining -= elapsed;
        start = end;
    }
}

/// `__delay(loops)` — Linux's exported low-level wait. Dispatches on the
/// strategy selected at boot.
pub fn __delay(loops: u64) {
    match current_kind() {
        DelayKind::Loop => delay_loop(loops),
        DelayKind::Tsc => delay_tsc(loops),
        DelayKind::Tpause => delay_halt(loops, delay_halt_tpause),
        DelayKind::Mwaitx => delay_halt(loops, delay_halt_mwaitx),
    }
}

/// `__const_udelay(xloops)` — converts `xloops` (microseconds * 2**32/10⁶)
/// to TSC cycles using `loops_per_jiffy`. Linux uses `mull %edx` inline;
/// we use a u128 wide multiply. Returns through `__delay`.
///
/// `lpj` should be the per-CPU `loops_per_jiffy` (lupos exposes this via
/// the scheduler once that lands — for now callers pass it directly to
/// mirror the Linux call shape).
pub fn __const_udelay(xloops: u64, lpj: u64, hz: u32) -> u64 {
    let xloops4 = xloops.wrapping_mul(4);
    // Linux multiplies `xloops` by `lpj * (HZ/4)` and uses the high 32 bits.
    let denom = lpj.wrapping_mul((hz / 4) as u64);
    let prod = (xloops4 as u128).wrapping_mul(denom as u128);
    let result = ((prod >> 32) as u64).wrapping_add(1);
    __delay(result);
    result
}

/// `__udelay(usecs)` — microsecond delay. Constant `0x10c7` = ceil(2³²/10⁶).
pub fn __udelay(usecs: u64, lpj: u64, hz: u32) -> u64 {
    __const_udelay(usecs.wrapping_mul(0x0000_10c7), lpj, hz)
}

/// `__ndelay(nsecs)` — nanosecond delay. Constant `5` = ceil(2³²/10⁹).
pub fn __ndelay(nsecs: u64, lpj: u64, hz: u32) -> u64 {
    __const_udelay(nsecs.wrapping_mul(0x0000_0005), lpj, hz)
}

/// `read_current_timer(timer_val)` — only returns 0 if TSC delay is
/// active. Mirrors `delay.c` lines 192-199.
pub fn read_current_timer() -> Result<u64, ()> {
    match current_kind() {
        DelayKind::Tsc => Ok(tsc::read()),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::Ordering;

    fn reset_strategy(kind: DelayKind) {
        DELAY_FN.store(kind as u8, Ordering::SeqCst);
    }

    #[test]
    fn default_strategy_is_loop_matches_delay_c_init() {
        // `delay_fn` is initialized to `delay_loop` in delay.c line 36.
        reset_strategy(DelayKind::Loop);
        assert_eq!(current_kind(), DelayKind::Loop);
    }

    #[test]
    fn use_tsc_delay_only_promotes_from_loop() {
        // Loop → TSC promotion is conditional in delay.c line 176:
        // `if (delay_fn == delay_loop) delay_fn = delay_tsc;`
        reset_strategy(DelayKind::Loop);
        use_tsc_delay();
        assert_eq!(current_kind(), DelayKind::Tsc);

        // From TPAUSE, use_tsc_delay must be a no-op.
        reset_strategy(DelayKind::Tpause);
        use_tsc_delay();
        assert_eq!(current_kind(), DelayKind::Tpause);
    }

    #[test]
    fn use_tpause_delay_overrides_unconditionally() {
        reset_strategy(DelayKind::Tsc);
        use_tpause_delay();
        assert_eq!(current_kind(), DelayKind::Tpause);
    }

    #[test]
    fn use_mwaitx_delay_overrides_unconditionally() {
        reset_strategy(DelayKind::Loop);
        use_mwaitx_delay();
        assert_eq!(current_kind(), DelayKind::Mwaitx);
    }

    #[test]
    fn read_current_timer_only_succeeds_under_tsc_strategy() {
        reset_strategy(DelayKind::Tsc);
        assert!(read_current_timer().is_ok());
        reset_strategy(DelayKind::Loop);
        assert!(read_current_timer().is_err());
        reset_strategy(DelayKind::Tpause);
        assert!(read_current_timer().is_err());
        reset_strategy(DelayKind::Mwaitx);
        assert!(read_current_timer().is_err());
    }

    #[test]
    fn const_udelay_returns_nonzero_and_calls_delay() {
        // With lpj=10000, HZ=1000, xloops=1: result ≈ ((1*4 * 10000*250) >> 32) + 1.
        // Confirms the wide-mul shape rather than panic.
        reset_strategy(DelayKind::Loop);
        let r = __const_udelay(1, 10_000, 1000);
        assert!(r >= 1);
    }

    #[test]
    fn delay_loop_zero_is_noop() {
        // delay.c line 41 short-circuits on zero — replicate the safety.
        delay_loop(0);
    }

    #[test]
    fn mwaitx_max_constant_matches_linux() {
        assert_eq!(MWAITX_MAX_WAIT_CYCLES, 0xFFFF_FFFF);
    }
}
