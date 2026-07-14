//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/time.c
//! test-origin: linux:vendor/linux/kernel/time/time.c
//! Common time conversion helpers for M36.
//!
//! Mirrors `vendor/linux/kernel/time/time.c`.

use super::posix_clock::Timespec64;
use crate::kernel::module::{export_symbol, find_symbol};

const NSEC_PER_SEC: i64 = 1_000_000_000;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("ns_to_timespec64", linux_ns_to_timespec64 as usize, false);
    export_symbol_once(
        "set_normalized_timespec64",
        linux_set_normalized_timespec64 as usize,
        false,
    );
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timeval64 {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

pub fn ns_to_timeval(ns: u64) -> Timeval64 {
    Timeval64 {
        tv_sec: (ns / 1_000_000_000) as i64,
        tv_usec: ((ns % 1_000_000_000) / 1_000) as i64,
    }
}

pub fn timeval_to_ns(tv: Timeval64) -> Option<u64> {
    if tv.tv_sec < 0 || tv.tv_usec < 0 || tv.tv_usec >= 1_000_000 {
        return None;
    }
    Some(
        (tv.tv_sec as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add((tv.tv_usec as u64) * 1_000),
    )
}

pub fn timespec64_to_ns(ts: Timespec64) -> Option<u64> {
    if ts.is_valid() {
        Some(ts.to_ns())
    } else {
        None
    }
}

pub fn ns_to_timespec64(ns: i64) -> Timespec64 {
    Timespec64 {
        tv_sec: ns.div_euclid(NSEC_PER_SEC),
        tv_nsec: ns.rem_euclid(NSEC_PER_SEC),
    }
}

pub fn set_normalized_timespec64(ts: &mut Timespec64, sec: i64, nsec: i64) {
    ts.tv_sec = sec + nsec.div_euclid(NSEC_PER_SEC);
    ts.tv_nsec = nsec.rem_euclid(NSEC_PER_SEC);
}

/// `ns_to_timespec64` - `vendor/linux/kernel/time/time.c:518`.
pub extern "C" fn linux_ns_to_timespec64(ns: i64) -> Timespec64 {
    ns_to_timespec64(ns)
}

/// `set_normalized_timespec64` - `vendor/linux/kernel/time/time.c:490`.
pub unsafe extern "C" fn linux_set_normalized_timespec64(ts: *mut Timespec64, sec: i64, nsec: i64) {
    if let Some(ts) = unsafe { ts.as_mut() } {
        set_normalized_timespec64(ts, sec, nsec);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeval_round_trip_preserves_microsecond_resolution() {
        let tv = ns_to_timeval(1_234_567_890);
        assert_eq!(
            tv,
            Timeval64 {
                tv_sec: 1,
                tv_usec: 234_567
            }
        );
        assert_eq!(timeval_to_ns(tv), Some(1_234_567_000));
    }

    #[test]
    fn ns_to_timespec64_matches_linux_normalization() {
        assert_eq!(ns_to_timespec64(0), Timespec64::new(0, 0));
        assert_eq!(
            ns_to_timespec64(1_234_567_890),
            Timespec64::new(1, 234_567_890)
        );
        assert_eq!(ns_to_timespec64(-1), Timespec64::new(-1, 999_999_999));
        assert_eq!(ns_to_timespec64(-1_000_000_000), Timespec64::new(-1, 0));
        assert_eq!(
            ns_to_timespec64(-1_000_000_001),
            Timespec64::new(-2, 999_999_999)
        );
    }

    #[test]
    fn time_conversion_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            find_symbol("ns_to_timespec64"),
            Some(linux_ns_to_timespec64 as usize)
        );
    }
}
