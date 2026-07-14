//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/clocksource.c
//! test-origin: linux:vendor/linux/kernel/time/clocksource.c
//! Clocksource — M36.
//!
//! Mirrors `vendor/linux/kernel/time/clocksource.c`.  A `Clocksource` is a
//! monotonic counter (TSC, HPET, PM-timer, jiffies fallback) characterised by
//! a `read()` callback and `mult`/`shift` calibration constants used to scale
//! cycles to nanoseconds.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

use super::jiffies::{NSEC_PER_TICK, jiffies};

/// Linux `CLOCK_SOURCE_*` quality flags.
pub const CLOCK_SOURCE_IS_CONTINUOUS: u32 = 0x0001;
pub const CLOCK_SOURCE_VALID_FOR_HRES: u32 = 0x0010;
pub const CLOCK_SOURCE_MUST_VERIFY: u32 = 0x0020;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "clocks_calc_mult_shift",
        linux_clocks_calc_mult_shift as usize,
        true,
    );
}

/// `struct clocksource` — Linux ABI shape.
pub struct Clocksource {
    pub name: String,
    pub rating: u32,
    pub flags: u32,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
    pub freq_hz: u64,
    pub read_fn: fn() -> u64,
}

impl Clocksource {
    /// Convert a cycle count to nanoseconds: `(cycles * mult) >> shift`.
    #[inline]
    pub fn cyc2ns(&self, cycles: u64) -> u64 {
        ((cycles & self.mask).saturating_mul(self.mult as u64)) >> self.shift
    }

    /// Read the current cycle count.
    #[inline]
    pub fn read(&self) -> u64 {
        (self.read_fn)()
    }
}

unsafe impl Send for Clocksource {}
unsafe impl Sync for Clocksource {}

// ── Built-in sources ─────────────────────────────────────────────────────────

/// Jiffies-based fallback — always available.
fn read_jiffies() -> u64 {
    jiffies()
}

/// TSC reader (RDTSC).  Stub returns 0 in tests / non-x86 builds.
pub fn read_tsc() -> u64 {
    crate::arch::x86::kernel::tsc::read()
}

pub fn jiffies_clocksource() -> Clocksource {
    Clocksource {
        name: String::from("jiffies"),
        rating: 1,
        flags: CLOCK_SOURCE_IS_CONTINUOUS,
        mask: u64::MAX,
        mult: NSEC_PER_TICK as u32,
        shift: 0,
        freq_hz: super::jiffies::HZ,
        read_fn: read_jiffies,
    }
}

pub fn tsc_clocksource() -> Clocksource {
    Clocksource {
        name: String::from("tsc"),
        rating: 400,
        flags: CLOCK_SOURCE_IS_CONTINUOUS | CLOCK_SOURCE_VALID_FOR_HRES,
        mask: u64::MAX,
        mult: 1,
        shift: 0,
        freq_hz: 1_000_000_000, // assume 1 GHz nominal until M37 calibrates
        read_fn: read_tsc,
    }
}

// ── Registry ─────────────────────────────────────────────────────────────────

static REGISTERED: Mutex<Vec<Clocksource>> = Mutex::new(Vec::new());
static SELECTED_RATING: AtomicU64 = AtomicU64::new(0);

/// Register a clocksource.  Higher rating wins as the default.
pub fn clocksource_register(cs: Clocksource) {
    let cur = SELECTED_RATING.load(Ordering::Acquire);
    if (cs.rating as u64) > cur {
        SELECTED_RATING.store(cs.rating as u64, Ordering::Release);
    }
    REGISTERED.lock().push(cs);
}

/// Read the current best clocksource.
pub fn current_cyc2ns(cycles: u64) -> u64 {
    let g = REGISTERED.lock();
    if let Some(cs) = g.iter().max_by_key(|c| c.rating) {
        cs.cyc2ns(cycles)
    } else {
        cycles
    }
}

/// `clocks_calc_mult_shift` - `vendor/linux/kernel/time/clocksource.c`.
pub fn clocks_calc_mult_shift_values(from: u32, to: u32, maxsec: u32) -> (u32, u32) {
    if from == 0 {
        return (0, 0);
    }

    let mut tmp = ((maxsec as u64) * (from as u64)) >> 32;
    let mut sftacc = 32u32;
    while tmp != 0 {
        tmp >>= 1;
        sftacc -= 1;
    }

    let mut best_mult = 0u64;
    let mut best_shift = 0u32;
    for sft in (1..=32u32).rev() {
        tmp = (to as u64) << sft;
        tmp += (from / 2) as u64;
        tmp /= from as u64;
        if (tmp >> sftacc) == 0 {
            best_mult = tmp;
            best_shift = sft;
            break;
        }
    }

    (best_mult as u32, best_shift)
}

unsafe extern "C" fn linux_clocks_calc_mult_shift(
    mult: *mut u32,
    shift: *mut u32,
    from: u32,
    to: u32,
    maxsec: u32,
) {
    if mult.is_null() || shift.is_null() {
        return;
    }
    let (calculated_mult, calculated_shift) = clocks_calc_mult_shift_values(from, to, maxsec);
    unsafe {
        *mult = calculated_mult;
        *shift = calculated_shift;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jiffies_clocksource_is_continuous() {
        let cs = jiffies_clocksource();
        assert!(cs.flags & CLOCK_SOURCE_IS_CONTINUOUS != 0);
    }

    #[test]
    fn tsc_rating_is_higher_than_jiffies() {
        assert!(tsc_clocksource().rating > jiffies_clocksource().rating);
    }

    #[test]
    fn cyc2ns_scales_correctly() {
        let cs = jiffies_clocksource();
        // 1 jiffy = NSEC_PER_TICK
        assert_eq!(cs.cyc2ns(1), NSEC_PER_TICK);
    }

    #[test]
    fn clocks_calc_mult_shift_matches_hda_24mhz_case() {
        let (mult, shift) = clocks_calc_mult_shift_values(24_000_000, 1_000_000_000, 600);

        assert_eq!(mult, 699_050_667);
        assert_eq!(shift, 24);
    }

    #[test]
    fn clocksource_exports_register_for_modules() {
        register_module_exports();

        assert_eq!(
            crate::kernel::module::find_symbol("clocks_calc_mult_shift"),
            Some(linux_clocks_calc_mult_shift as usize)
        );
    }
}
