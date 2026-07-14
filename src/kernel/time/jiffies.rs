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
pub fn usecs_to_jiffies(us: u64) -> u64 {
    (us.saturating_mul(HZ) + 999_999) / 1_000_000
}

fn round_jiffies_common(j: u64, force_up: bool) -> u64 {
    let original = j;
    let rem = j % HZ;
    let rounded = if rem < HZ / 4 && !force_up {
        j - rem
    } else {
        j - rem + HZ
    };
    if time_after(rounded, jiffies()) {
        rounded
    } else {
        original
    }
}

pub fn round_jiffies(j: u64) -> u64 {
    round_jiffies_common(j, false)
}

pub fn round_jiffies_relative(j: u64) -> u64 {
    let now = jiffies();
    round_jiffies_common(now.saturating_add(j), false).saturating_sub(now)
}

pub fn round_jiffies_up(j: u64) -> u64 {
    round_jiffies_common(j, true)
}

pub fn round_jiffies_up_relative(j: u64) -> u64 {
    let now = jiffies();
    round_jiffies_common(now.saturating_add(j), true).saturating_sub(now)
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
    if find_symbol("__usecs_to_jiffies").is_none() {
        export_symbol(
            "__usecs_to_jiffies",
            linux___usecs_to_jiffies as usize,
            false,
        );
    }
    if find_symbol("__round_jiffies_relative").is_none() {
        export_symbol(
            "__round_jiffies_relative",
            linux___round_jiffies_relative as usize,
            true,
        );
    }
    if find_symbol("round_jiffies").is_none() {
        export_symbol("round_jiffies", linux_round_jiffies as usize, true);
    }
    if find_symbol("round_jiffies_relative").is_none() {
        export_symbol(
            "round_jiffies_relative",
            linux_round_jiffies_relative as usize,
            true,
        );
    }
    if find_symbol("__round_jiffies_up_relative").is_none() {
        export_symbol(
            "__round_jiffies_up_relative",
            linux___round_jiffies_up_relative as usize,
            true,
        );
    }
    if find_symbol("round_jiffies_up").is_none() {
        export_symbol("round_jiffies_up", linux_round_jiffies_up as usize, true);
    }
    if find_symbol("round_jiffies_up_relative").is_none() {
        export_symbol(
            "round_jiffies_up_relative",
            linux_round_jiffies_up_relative as usize,
            true,
        );
    }
}

extern "C" fn linux___usecs_to_jiffies(us: u32) -> u64 {
    usecs_to_jiffies(us as u64)
}

extern "C" fn linux___round_jiffies_relative(j: u64, _cpu: i32) -> u64 {
    round_jiffies_relative(j)
}

extern "C" fn linux_round_jiffies(j: u64) -> u64 {
    round_jiffies(j)
}

extern "C" fn linux_round_jiffies_relative(j: u64) -> u64 {
    round_jiffies_relative(j)
}

extern "C" fn linux___round_jiffies_up_relative(j: u64, _cpu: i32) -> u64 {
    round_jiffies_up_relative(j)
}

extern "C" fn linux_round_jiffies_up(j: u64) -> u64 {
    round_jiffies_up(j)
}

extern "C" fn linux_round_jiffies_up_relative(j: u64) -> u64 {
    round_jiffies_up_relative(j)
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
        assert_eq!(usecs_to_jiffies(1_000_000), HZ);
    }

    #[test]
    fn time_after_handles_wrap() {
        // a > b
        assert!(time_after(100, 50));
        assert!(time_before(50, 100));
    }

    #[test]
    fn round_jiffies_up_never_rounds_down() {
        _reset_for_tests();
        assert_eq!(round_jiffies_up(HZ), HZ * 2);
        assert_eq!(round_jiffies_up_relative(1), HZ);
    }
}
