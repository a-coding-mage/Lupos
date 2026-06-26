//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/time.c
//! test-origin: linux:vendor/linux/kernel/time/time.c
//! Common time conversion helpers for M36.
//!
//! Mirrors `vendor/linux/kernel/time/time.c`.

use super::posix_clock::Timespec64;

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
}
