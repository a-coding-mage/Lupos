//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/jiffies.c
//! test-origin: linux:vendor/linux/kernel/time/jiffies.c
//! `jiffies` — tick counter (M36).
//!
//! Mirrors `vendor/linux/include/linux/jiffies.h`.  `HZ=250` ⇒ 4 ms per tick.
//! `JIFFIES` increments on every periodic clock event.

use core::sync::atomic::{AtomicU64, Ordering};

/// Linux `CONFIG_HZ` default.  Must match the LAPIC programming.
pub const HZ: u64 = 250;

/// Nanoseconds per tick (4 ms with HZ=250).
pub const NSEC_PER_TICK: u64 = 1_000_000_000 / HZ;

static JIFFIES: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn jiffies() -> u64 {
    JIFFIES.load(Ordering::Acquire)
}

/// Bump JIFFIES — invoked from `apic_timer::on_tick` once M36 is wired in.
#[inline]
pub fn tick_jiffies() {
    JIFFIES.fetch_add(1, Ordering::AcqRel);
}

#[inline]
pub fn jiffies_to_msecs(j: u64) -> u64 {
    j.saturating_mul(1000) / HZ
}

#[inline]
pub fn jiffies_to_usecs(j: u64) -> u64 {
    j.saturating_mul(1_000_000) / HZ
}

#[inline]
pub fn msecs_to_jiffies(ms: u64) -> u64 {
    (ms.saturating_mul(HZ) + 999) / 1000
}

#[inline]
pub fn time_after(a: u64, b: u64) -> bool {
    (a as i64).wrapping_sub(b as i64) > 0
}
#[inline]
pub fn time_before(a: u64, b: u64) -> bool {
    time_after(b, a)
}

/// Export `jiffies`/`jiffies_64` for vendor-built `.ko` modules.
///
/// Linux exports `jiffies` and its 64-bit alias `jiffies_64` as *data* symbols;
/// modules read the tick counter directly through the symbol address instead of
/// calling a function (e.g. `virtio_net` references `jiffies`).  We hand out the
/// address of [`JIFFIES`], whose little-endian `u64` storage matches Linux's
/// `volatile unsigned long jiffies`.  On 64-bit Linux `jiffies` aliases
/// `jiffies_64`, so both names resolve to the same address.
/// Ref: `vendor/linux/kernel/time/timer.c` `EXPORT_SYMBOL(jiffies)` and
/// `vendor/linux/kernel/time/jiffies.c` `EXPORT_SYMBOL(jiffies_64)`.
pub fn register_module_exports() {
    use crate::kernel::module::{export_symbol, find_symbol};

    let addr = core::ptr::addr_of!(JIFFIES) as usize;
    if find_symbol("jiffies").is_none() {
        export_symbol("jiffies", addr, false);
    }
    if find_symbol("jiffies_64").is_none() {
        export_symbol("jiffies_64", addr, false);
    }
}

/// Reset to zero — used by tests only.
#[doc(hidden)]
pub fn _reset_for_tests() {
    JIFFIES.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hz_default_is_250() {
        assert_eq!(HZ, 250);
        assert_eq!(NSEC_PER_TICK, 4_000_000);
    }

    #[test]
    fn tick_increments_jiffies() {
        _reset_for_tests();
        let before = jiffies();
        tick_jiffies();
        assert_eq!(jiffies(), before + 1);
    }

    #[test]
    fn msecs_to_jiffies_round_trip() {
        // 1000 ms = HZ jiffies
        assert_eq!(msecs_to_jiffies(1000), HZ);
        assert_eq!(jiffies_to_msecs(HZ), 1000);
    }

    #[test]
    fn time_after_handles_wrap() {
        // a > b
        assert!(time_after(100, 50));
        assert!(time_before(50, 100));
    }
}
